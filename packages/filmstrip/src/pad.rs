//! A tall frame on a square canvas: the film as a full-height panel in the middle, the bars to
//! its left and right a blurred, toned-down copy of the frame, lit by a soft glow from the panel
//! and darkened towards the corners. [`Square::grain`] adds fine grain to the bars, moved every
//! few frames.

use crate::Frame;

/// Pixels of the edge to the left and right of the panel.
const EDGE: u32 = 2;
/// The background is blurred at 1/20 of the canvas, with a sigma of 12 at 54 px.
const SMALL: u32 = 20;
/// The tone curve of the background: 0 goes to `LOW`, 1 to `HIGH`.
const LOW: f32 = 0.08;
const HIGH: f32 = 0.60;
/// How far the background goes towards the glow colour right next to the panel.
const GLOW: f32 = 0.40;
/// How fast the glow fades away from the panel: its square distance in px at 1080 px over this.
const SPREAD: f32 = 30_000.0;
/// The grain: up to this much lighter or darker, in steps of 1/255.
const GRAIN: u32 = 6;
/// The grain moves every this many frames.
const HOLD: u64 = 4;
/// The grain is full from this green value up, weaker below it.
const LIT: i16 = 64;

/// The square-pad compositor for tall frames of one size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Square {
    /// Width and height of the tall frames.
    tall: (u32, u32),
    /// Width of the panel, without its edge.
    panel: u32,
    /// The colour of the edge.
    edge: [u8; 3],
    /// The colour of the glow.
    glow: [u8; 3],
}

impl Square {
    /// A compositor for `width` x `height` tall frames (both even, height above width); the
    /// canvas is `width` square, the panel `width` tall.
    #[must_use]
    ///
    /// # Panics
    ///
    /// If the frames are not tall: a wide or square frame leaves no room for the panel.
    pub fn new(width: u32, height: u32, edge: [u8; 3], glow: [u8; 3]) -> Self {
        assert!(
            height > width,
            "pad: square needs tall frames, got {width}x{height}"
        );
        let panel = (width * width + height) / (2 * height.max(1)) * 2;
        Self {
            tall: (width, height),
            panel: panel.min(width.saturating_sub(2 * EDGE)),
            edge,
            glow,
        }
    }

    /// Width and height of the canvas.
    #[must_use]
    pub const fn size(&self) -> u32 {
        self.tall.0
    }

    /// The columns of the panel, edge included: from, to (exclusive).
    const fn span(&self) -> (u32, u32) {
        let left = (self.size() - self.panel) / 2 - EDGE;
        (left, left + self.panel + 2 * EDGE)
    }

    /// The tall `frame` on the square canvas, without grain.
    #[must_use]
    pub fn frame(&self, frame: &Frame) -> Frame {
        let size = self.size();
        let (left, right) = self.span();
        let mut rgb = vec![0; (size * size * 3) as usize];
        // the panel: the whole frame, scaled to the canvas height
        let fg = area(frame, 0, frame.height, self.panel, size);
        for y in 0..size {
            let row = &mut rgb[(y * size * 3) as usize..((y + 1) * size * 3) as usize];
            for x in left..right {
                let at = (x * 3) as usize;
                let inner = x >= left + EDGE && x < right - EDGE;
                let px = if inner {
                    let from = ((y * self.panel + x - left - EDGE) * 3) as usize;
                    [0, 1, 2].map(|c| fg[from + c].round() as u8)
                } else {
                    self.edge
                };
                row[at..at + 3].copy_from_slice(&px);
            }
        }
        // the bars: the frame scaled to cover the square, blurred small, scaled back up
        let n = (size / SMALL).max(2);
        let mut small = area(frame, (frame.height - size) / 2, size, n, n);
        let sigma = 12.0 * n as f32 / 54.0;
        let radius = ((4.0 * sigma).mul_add(sigma, 1.0).sqrt() - 1.0) / 2.0;
        for _ in 0..3 {
            blur(&mut small, n, radius.round().max(1.0) as usize);
        }
        let half = size as f32 / 2.0;
        let panel = (right - left) as f32 / 2.0;
        let scale = size as f32 / 1080.0;
        let spread = SPREAD * scale * scale;
        for y in 0..size {
            let dy = (y as f32 + 0.5 - half) / half;
            for x in (0..left).chain(right..size) {
                let dx = (x as f32 + 0.5 - half) / half;
                // light from the panel: strongest next to it, the same all the way down
                let off = dx.abs().mul_add(half, -panel).max(0.0);
                // libm: the same bits on every platform, so the films are too
                let glow = GLOW * libm::expf(-off * off / spread);
                // the vignette, (1 - r^2)^1.5 with r = 1 in the corners: dark slate towards the
                // edges, black in the corners
                let open = 1.0 - dx.mul_add(dx, dy * dy) / 2.0;
                let dark = open * open.sqrt();
                let blurred = bilinear(
                    &small,
                    n,
                    x as f32 * n as f32 / size as f32,
                    y as f32 * n as f32 / size as f32,
                );
                let at = ((y * size + x) * 3) as usize;
                for c in 0..3 {
                    let toned = (HIGH - LOW).mul_add(blurred[c] / 255.0, LOW) * 255.0;
                    let lit = (f32::from(self.glow[c]) - toned).mul_add(glow, toned);
                    rgb[at + c] = (lit * dark).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        Frame {
            width: size,
            height: size,
            rgb,
        }
    }

    /// Add grain to the bars of a `frame` this compositor made: monochrome, weaker in the dark,
    /// the same for the same `index`. The grain is one fixed pattern, shifted by up to 3 px
    /// every 4th frame and repeating after 16 shifts: it reads as live grain, and an H.264
    /// encoder follows the shift where fresh noise in every frame would double its work and
    /// the file.
    pub fn grain(&self, frame: &mut Frame, index: u64) {
        let size = self.size();
        let (left, right) = self.span();
        // 0, 5, 10, 15, 4, ...: every step lands on another of the 16 shifts
        let shift = (index / HOLD % 16 * 5 % 16) as u32;
        let (sx, sy) = (shift % 4, shift / 4);
        for y in 0..size {
            for x in (0..left).chain(right..size) {
                let cell = (y + sy) * (size + 4) + x + sx;
                let noise = (mix(cell) % (2 * GRAIN + 1)) as i16 - GRAIN as i16;
                let at = ((y * size + x) * 3) as usize;
                // full grain from a green of LIT up, less in the dark, as on film
                let light = i16::from(frame.rgb[at + 1]).min(LIT);
                let noise = noise * light / LIT;
                for v in &mut frame.rgb[at..at + 3] {
                    *v = (i16::from(*v) + noise).clamp(0, 255) as u8;
                }
            }
        }
    }
}

/// A 32-bit integer hash (lowbias32).
const fn mix(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^ (h >> 16)
}

/// Rows `top` to `top + rows` of `frame` scaled down to `width` x `height` (both at most the
/// source's), every pixel the mean of the area it covers; RGB as floats.
fn area(frame: &Frame, top: u32, rows: u32, width: u32, height: u32) -> Vec<f32> {
    let across = taps(frame.width, width);
    let down = taps(rows, height);
    let mut wide = vec![0.0; (rows * width * 3) as usize];
    for y in 0..rows {
        let src = &frame.rgb[((top + y) * frame.width * 3) as usize..];
        for (x, (start, weights)) in across.iter().enumerate() {
            let out = (y as usize * width as usize + x) * 3;
            for (k, w) in weights.iter().enumerate() {
                let from = (start + k) * 3;
                for c in 0..3 {
                    wide[out + c] += w * f32::from(src[from + c]);
                }
            }
        }
    }
    let mut out = vec![0.0; (width * height * 3) as usize];
    let row = width as usize * 3;
    for (y, (start, weights)) in down.iter().enumerate() {
        let to = &mut out[y * row..(y + 1) * row];
        for (k, w) in weights.iter().enumerate() {
            let from = &wide[(start + k) * row..(start + k + 1) * row];
            for (o, v) in to.iter_mut().zip(from) {
                *o += w * v;
            }
        }
    }
    out
}

/// For every one of `to` cells over `from` (at least `to`): the first source cell it covers and
/// the share of each covered cell, the shares summing to 1.
fn taps(from: u32, to: u32) -> Vec<(usize, Vec<f32>)> {
    let scale = f64::from(from) / f64::from(to.max(1));
    (0..to)
        .map(|i| {
            let (a, b) = (f64::from(i) * scale, f64::from(i + 1) * scale);
            let start = a.floor() as usize;
            let end = (b.ceil() as usize).min(from as usize);
            let weights = (start..end)
                .map(|s| ((b.min(s as f64 + 1.0) - a.max(s as f64)) / scale) as f32)
                .collect();
            (start, weights)
        })
        .collect()
}

/// One box-blur pass of `radius` over an `n` x `n` RGB float image, rows then columns, the edge
/// pixels repeated.
fn blur(image: &mut [f32], n: u32, radius: usize) {
    let n = n as usize;
    let mut line = vec![0.0; n * 3];
    for (step, stride) in [(3, n * 3), (n * 3, 3)] {
        for lane in 0..n {
            let at = |i: usize| lane * stride + i * step;
            for i in 0..n {
                for c in 0..3 {
                    let mut sum = 0.0;
                    for k in i..=i + 2 * radius {
                        sum += image[at(k.saturating_sub(radius).min(n - 1)) + c];
                    }
                    line[i * 3 + c] = sum / (2 * radius + 1) as f32;
                }
            }
            for i in 0..n {
                for c in 0..3 {
                    image[at(i) + c] = line[i * 3 + c];
                }
            }
        }
    }
}

/// The `n` x `n` RGB float image at `x`, `y` in its own pixels, smoothly between pixel centres.
fn bilinear(image: &[f32], n: u32, x: f32, y: f32) -> [f32; 3] {
    let last = (n - 1) as f32;
    let (x, y) = ((x - 0.5).clamp(0.0, last), (y - 0.5).clamp(0.0, last));
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(n as usize - 1), (y0 + 1).min(n as usize - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let px = |x: usize, y: usize, c: usize| image[(y * n as usize + x) * 3 + c];
    [0, 1, 2].map(|c| {
        let top = (px(x1, y0, c) - px(x0, y0, c)).mul_add(fx, px(x0, y0, c));
        let bottom = (px(x1, y1, c) - px(x0, y1, c)).mul_add(fx, px(x0, y1, c));
        (bottom - top).mul_add(fy, top)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tall() -> Frame {
        // a dark frame with a white column down its middle
        Frame::drawn(108, 192, |x, _| if x == 54 { [255; 3] } else { [0; 3] })
    }

    #[test]
    #[should_panic(expected = "pad: square needs tall frames, got 192x108")]
    fn a_wide_frame_has_no_room_for_the_panel() {
        let _ = Square::new(192, 108, [1, 2, 3], [0, 0, 255]);
    }

    #[test]
    fn the_canvas_is_square_with_the_panel_full_height_in_the_middle() {
        let pad = Square::new(108, 192, [1, 2, 3], [0, 0, 255]);
        let out = pad.frame(&tall());
        assert!(out.is_consistent());
        assert_eq!((out.width, out.height), (108, 108));
        // 108 * 108 / 192 = 60.75: a panel of 60, from 24 to 84, the edges outside it
        assert_eq!(pad.span(), (22, 86));
        for y in [0, 54, 107] {
            assert_eq!(out.pixel(22, y), [1, 2, 3]);
            assert_eq!(out.pixel(85, y), [1, 2, 3]);
            assert_eq!(out.pixel(24, y), [0; 3]);
        }
        // the white column lands in the middle of the panel
        let middle = out.pixel(54, 54)[0].max(out.pixel(53, 54)[0]);
        assert!(middle > 60, "{middle}");
        assert_eq!(out.pixel(30, 54), [0; 3]);
    }

    #[test]
    fn the_bars_follow_the_curve_the_glow_and_the_vignette() {
        let black = Frame::drawn(108, 192, |_, _| [0; 3]);
        let white = Frame::drawn(108, 192, |_, _| [255; 3]);
        let pad = Square::new(108, 192, [0; 3], [0; 3]);
        // a glow to black and the vignette scale black and white alike: what is left of them
        // is the curve, 0 to LOW and 1 to HIGH
        let low = f32::from(pad.frame(&black).pixel(20, 54)[0]);
        let high = f32::from(pad.frame(&white).pixel(20, 54)[0]);
        assert!(low > 0.0 && low < LOW * 255.0, "{low}");
        assert!((high / low - HIGH / LOW).abs() < 0.5, "{high} / {low}");
        // next to the panel the glow pulls towards its colour
        let lit = Square::new(108, 192, [0; 3], [0, 0, 255]).frame(&black);
        assert!(lit.pixel(20, 54)[2] > lit.pixel(20, 54)[0] + 60);
        let blue = |x: u32, y: u32| i16::from(lit.pixel(x, y)[2]) - i16::from(lit.pixel(x, y)[0]);
        assert!(blue(20, 54) > blue(10, 54) && blue(10, 54) > blue(0, 54));
        // the glow is the same all the way down: blue over red, which the vignette keeps
        let tint = |y: u32| f32::from(lit.pixel(20, y)[2]) / f32::from(lit.pixel(20, y)[0]);
        assert!(
            (tint(8) / tint(54) - 1.0).abs() < 0.2,
            "{} {}",
            tint(8),
            tint(54)
        );
        // the corners are darker than the edge midpoints
        assert!(pad.frame(&white).pixel(0, 0)[0] < pad.frame(&white).pixel(0, 54)[0]);
    }

    #[test]
    fn the_grain_is_the_same_for_one_index_and_another_for_the_next() {
        let pad = Square::new(108, 192, [0; 3], [255; 3]);
        let base = pad.frame(&Frame::drawn(108, 192, |_, _| [255; 3]));
        let grained = |index| {
            let mut frame = base.clone();
            pad.grain(&mut frame, index);
            frame
        };
        assert_eq!(grained(7), grained(7));
        assert_ne!(grained(7), grained(8));
        assert_eq!(grained(4), grained(7), "the grain holds for {HOLD} frames");
        let one = grained(7);
        // only the bars, and within the amplitude
        assert_eq!(one.pixel(40, 40), base.pixel(40, 40));
        let most = one
            .rgb
            .iter()
            .zip(&base.rgb)
            .map(|(&a, &b)| (i16::from(a) - i16::from(b)).abs())
            .max();
        assert_eq!(most, Some(GRAIN as i16));
        // monochrome: every channel of a pixel moves by the same step
        let step = |c: usize| i16::from(one.pixel(3, 3)[c]) - i16::from(base.pixel(3, 3)[c]);
        assert!(step(0) == step(1) && step(1) == step(2));
    }

    #[test]
    fn an_area_scale_keeps_the_mean_and_shares_sum_to_one() {
        for (shares_from, to) in [(1920, 1080), (1080, 54), (7, 3)] {
            for (_, weights) in taps(shares_from, to) {
                assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-4);
            }
        }
        let grey = Frame::drawn(10, 10, |x, _| [if x % 2 == 0 { 0 } else { 200 }; 3]);
        assert!(
            area(&grey, 0, 10, 5, 5)
                .iter()
                .all(|v| (v - 100.0).abs() < 1e-3)
        );
    }
}
