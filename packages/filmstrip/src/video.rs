//! The MP4: the frames as H.264 (`rusty_h264-encoder`) in an MP4 container (`mp4`), one video
//! track, no audio, no B-frames so decode order is display order.

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use rusty_h264_common::YuvFrame;
use rusty_h264_encoder::{Encoder, EncoderConfig};

use crate::Frame;

/// H.264 levels from 3.0 up (Table A-1): `level_idc`, macroblocks per frame, macroblocks per
/// second.
const LEVELS: [(u8, usize, usize); 8] = [
    (30, 1_620, 40_500),
    (31, 3_600, 108_000),
    (32, 5_120, 216_000),
    (40, 8_192, 245_760),
    (42, 8_704, 522_240),
    (50, 22_080, 589_824),
    (51, 36_864, 983_040),
    (52, 36_864, 2_073_600),
];

/// The lowest level that holds `w` x `h` at `fps`: 3.0 for 640x360 at 10 fps, 4.0 for
/// 1080x1920 at 30.
fn level(w: usize, h: usize, fps: u16) -> u8 {
    let frame = w.div_ceil(16) * h.div_ceil(16);
    let rate = frame * usize::from(fps);
    LEVELS
        .iter()
        .find(|&&(_, size, speed)| frame <= size && rate <= speed)
        .map_or(52, |&(idc, ..)| idc)
}

/// Encode `frames` at `fps` into MP4 bytes. Width and height must be even.
///
/// # Examples
///
/// ```
/// use filmstrip::Frame;
///
/// let frames = vec![Frame::solid(32, 18, [0; 3]), Frame::solid(32, 18, [255; 3])];
/// let mp4 = filmstrip::video::encode(&frames, 10).expect("mp4");
/// assert_eq!(&mp4[4..8], b"ftyp");
/// ```
pub fn encode(frames: &[Frame], fps: u16) -> Result<Vec<u8>> {
    let Some(first) = frames.first() else {
        bail!("no frames");
    };
    let mut writer = Writer::new(first.width, first.height, fps)?;
    for f in frames {
        writer.push(f)?;
    }
    writer.finish()
}

/// An MP4 written frame by frame: each frame goes through the encoder at once, each picture
/// into the container as soon as the next one starts.
pub struct Writer {
    enc: Encoder,
    mp4: mp4::Mp4Writer<std::io::Cursor<Vec<u8>>>,
    size: (u32, u32),
    fps: u16,
    sps: Vec<u8>,
    pps: Vec<u8>,
    /// The picture being collected.
    pending: Option<Sample>,
    /// Pictures written, frames pushed.
    written: u64,
    pushed: u64,
    /// The last frame's pixels and its YUV: the same pixels are not converted again.
    last: Vec<u8>,
    yuv: Option<YuvFrame>,
}

impl std::fmt::Debug for Writer {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Writer")
            .field("size", &self.size)
            .field("fps", &self.fps)
            .finish_non_exhaustive()
    }
}

impl Writer {
    /// An MP4 of `width` x `height` (both even) at `fps`.
    pub fn new(width: u32, height: u32, fps: u16) -> Result<Self> {
        if fps == 0 {
            bail!("fps must be at least 1");
        }
        if !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            bail!("video needs even dimensions, got {width}x{height}");
        }
        let (w, h) = (width as usize, height as usize);
        let mut cfg = EncoderConfig::new(w, h);
        cfg.framerate = f32::from(fps);
        cfg.level_idc = level(w, h, fps);
        cfg.qp = 23;
        cfg.bframes = 0;
        cfg.lookahead = 0;
        let enc = Encoder::new(cfg).map_err(|e| anyhow::anyhow!("h264 encoder: {e:?}"))?;
        let config = mp4::Mp4Config {
            major_brand: "isom".parse().context("brand")?,
            minor_version: 512,
            compatible_brands: ["isom", "iso2", "avc1", "mp41"]
                .iter()
                .map(|b| b.parse().context("brand"))
                .collect::<Result<_>>()?,
            timescale: 1000,
        };
        let mp4 = mp4::Mp4Writer::write_start(std::io::Cursor::new(Vec::new()), &config)?;
        Ok(Self {
            enc,
            mp4,
            size: (width, height),
            fps,
            sps: Vec::new(),
            pps: Vec::new(),
            pending: None,
            written: 0,
            pushed: 0,
            last: Vec::new(),
            yuv: None,
        })
    }

    /// Encode the next frame; it has the video's size.
    pub fn push(&mut self, frame: &Frame) -> Result<()> {
        if (frame.width, frame.height) != self.size || !frame.is_consistent() {
            bail!("frames differ in size");
        }
        if self.yuv.is_none() || self.last != frame.rgb {
            self.last.clone_from(&frame.rgb);
            self.yuv = Some(i420(frame));
        }
        let Some(yuv) = &self.yuv else {
            bail!("no picture to encode");
        };
        let stream = self.enc.encode(yuv);
        self.pushed += 1;
        self.take(&stream)
    }

    /// Flush the encoder and close the container; the MP4's bytes.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        let tail = self.enc.flush();
        self.take(&tail)?;
        self.write_pending()?;
        if self.written == 0 {
            bail!("no frames");
        }
        if self.written != self.pushed {
            bail!(
                "the encoder returned {} pictures for {} frames",
                self.written,
                self.pushed
            );
        }
        self.mp4.write_end()?;
        Ok(self.mp4.into_writer().into_inner())
    }

    /// Sort the NALs of an Annex-B chunk: SPS and PPS kept for the track, a slice NAL (types 1
    /// and 5) starts a picture, other NALs travel with the current one.
    fn take(&mut self, stream: &[u8]) -> Result<()> {
        for nal in nals(stream) {
            let Some(&header) = nal.first() else { continue };
            let kind = header & 0x1f;
            match kind {
                7 => self.sps = nal.to_vec(),
                8 => self.pps = nal.to_vec(),
                9 | 6 => {}, // access unit delimiter, SEI: not needed in MP4
                1 | 5 => {
                    self.write_pending()?;
                    let mut sample = Vec::with_capacity(nal.len() + 4);
                    sample.extend_from_slice(&u32::try_from(nal.len())?.to_be_bytes());
                    sample.extend_from_slice(nal);
                    self.pending = Some((kind == 5, sample));
                },
                _ => {
                    if let Some((_, sample)) = self.pending.as_mut() {
                        sample.extend_from_slice(&u32::try_from(nal.len())?.to_be_bytes());
                        sample.extend_from_slice(nal);
                    }
                },
            }
        }
        Ok(())
    }

    /// Write the collected picture; the first one adds the track.
    fn write_pending(&mut self) -> Result<()> {
        let Some((is_sync, bytes)) = self.pending.take() else {
            return Ok(());
        };
        if self.written == 0 {
            if self.sps.is_empty() || self.pps.is_empty() {
                bail!("the encoder stream has no SPS/PPS");
            }
            self.mp4.add_track(&mp4::TrackConfig {
                track_type: mp4::TrackType::Video,
                timescale: u32::from(self.fps),
                language: "und".to_owned(),
                media_conf: mp4::MediaConfig::AvcConfig(mp4::AvcConfig {
                    width: u16::try_from(self.size.0)?,
                    height: u16::try_from(self.size.1)?,
                    seq_param_set: self.sps.clone(),
                    pic_param_set: self.pps.clone(),
                }),
            })?;
        }
        self.mp4.write_sample(
            1,
            &mp4::Mp4Sample {
                start_time: self.written,
                duration: 1,
                rendering_offset: 0,
                is_sync,
                bytes: Bytes::from(bytes),
            },
        )?;
        self.written += 1;
        Ok(())
    }
}

/// RGB to I420 (BT.601, limited range), chroma averaged over 2x2.
fn i420(frame: &Frame) -> YuvFrame {
    let (width, height) = (frame.width as usize, frame.height as usize);
    let mut luma = vec![0u8; width * height];
    let (chroma_w, chroma_h) = (width / 2, height / 2);
    let mut cb = vec![0u8; chroma_w * chroma_h];
    let mut cr = vec![0u8; chroma_w * chroma_h];
    let pixel = |col: usize, row: usize| {
        let i = (row * width + col) * 3;
        (
            f32::from(frame.rgb[i]),
            f32::from(frame.rgb[i + 1]),
            f32::from(frame.rgb[i + 2]),
        )
    };
    for row in 0..height {
        for col in 0..width {
            let (red, green, blue) = pixel(col, row);
            let sum = 24.966_f32.mul_add(blue, 128.553_f32.mul_add(green, 65.481 * red));
            luma[row * width + col] = (16.0 + sum / 255.0).round() as u8;
        }
    }
    for row in 0..chroma_h {
        for col in 0..chroma_w {
            let mut sum = (0.0_f32, 0.0_f32, 0.0_f32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (red, green, blue) = pixel(col * 2 + dx, row * 2 + dy);
                sum.0 += red;
                sum.1 += green;
                sum.2 += blue;
            }
            let (red, green, blue) = (sum.0 / 4.0, sum.1 / 4.0, sum.2 / 4.0);
            let blue_diff = 112.0_f32.mul_add(blue, 74.203_f32.mul_add(-green, -37.797 * red));
            let red_diff = 18.214_f32.mul_add(-blue, 93.786_f32.mul_add(-green, 112.0 * red));
            cb[row * chroma_w + col] = (128.0 + blue_diff / 255.0).round() as u8;
            cr[row * chroma_w + col] = (128.0 + red_diff / 255.0).round() as u8;
        }
    }
    YuvFrame {
        width,
        height,
        y: luma,
        u: cb,
        v: cr,
    }
}

/// One picture for the muxer: whether it is a keyframe, and its length-prefixed NALs.
type Sample = (bool, Vec<u8>);

/// The NALs of an Annex-B stream, start codes removed.
fn nals(stream: &[u8]) -> Vec<&[u8]> {
    let mut nals = Vec::new();
    let mut i = 0;
    let mut start: Option<usize> = None;
    while i + 3 <= stream.len() {
        let code = if stream[i..].starts_with(&[0, 0, 0, 1]) {
            Some(4)
        } else if stream[i..].starts_with(&[0, 0, 1]) {
            Some(3)
        } else {
            None
        };
        if let Some(len) = code {
            if let Some(s) = start {
                nals.push(&stream[s..i]);
            }
            i += len;
            start = Some(i);
        } else {
            i += 1;
        }
    }
    if let Some(s) = start {
        nals.push(&stream[s..]);
    }
    nals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_frame_becomes_a_sample_and_the_first_is_a_keyframe() {
        let frames: Vec<Frame> = (0..5)
            .map(|i| Frame::solid(32, 18, [i * 40, 0, 255 - i * 40]))
            .collect();
        let bytes = encode(&frames, 10).unwrap();
        let size = bytes.len() as u64;
        let mut reader = mp4::Mp4Reader::read_header(std::io::Cursor::new(bytes), size).unwrap();
        let (&id, track) = reader.tracks().iter().next().unwrap();
        assert_eq!(track.sample_count(), 5);
        assert_eq!((track.width(), track.height()), (32, 18));
        let first = reader.read_sample(id, 1).unwrap().unwrap();
        assert!(first.is_sync);
        let last = reader.read_sample(id, 5).unwrap().unwrap();
        assert_eq!(last.start_time, 4);
    }

    #[test]
    fn odd_sizes_and_empty_input_are_errors() {
        assert!(encode(&[], 10).is_err());
        assert!(encode(&[Frame::solid(31, 18, [0; 3])], 10).is_err());
    }

    #[test]
    fn the_level_follows_the_macroblocks() {
        assert_eq!(level(640, 360, 10), 30);
        assert_eq!(level(1080, 1920, 30), 40);
        assert_eq!(level(1920, 1080, 60), 42);
        assert_eq!(level(7680, 4320, 60), 52);
    }

    #[test]
    fn grey_stays_grey_in_yuv() {
        let f = Frame::solid(2, 2, [128, 128, 128]);
        let yuv = i420(&f);
        assert_eq!(yuv.u, vec![128]);
        assert_eq!(yuv.v, vec![128]);
        assert!(yuv.y.iter().all(|&y| (125..=128).contains(&y)));
    }
}
