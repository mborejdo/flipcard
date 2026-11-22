# filmstrip

Frames in, films out. A list of RGB frames, whatever drew them, becomes:

- an animated GIF: one palette learned from the stills, delta frames (only the changed rectangle,
  unchanged pixels transparent), so a film of text cards stays small;
- an MP4, H.264 in pure Rust (`rusty_h264-encoder`, `mp4`), feature `mp4`.

`gif::Writer` and `video::Writer` take the frames one by one (a frame and how often it repeats),
so a long film is never all in memory; the H.264 level follows the frame size and rate. A GIF
loops forever unless `gif::Repeat::Once` is set: then it plays once and rests on the last frame. `Frame::downscale` makes a GIF preview
of a large film.

`pad::Square` places a tall frame as a full-height panel on a square canvas over a blurred copy
of itself, with grain for the MP4; `video::Picture` is a frame converted for the encoder, so the
conversion can run ahead of it.

`shots` plans a film from scenes (build-in frames, ticks per frame and a hold each), one `Shot`
per frame: each build-in, the hold on its last frame, a `Transition` into the next (fade, wipe,
push, pixelize, circle, split or cut; CPU-drawn, readable at 10 fps), and the last scene back
into the first, so the loop doesn't jump.

```rust
use filmstrip::{Frame, gif};

let frame = |v: u8| Frame { width: 4, height: 2, rgb: vec![v; 24] };
let stills = [frame(0), frame(200)];
let mut writer = gif::Writer::new(4, 2, &stills, 10, 16)?;
writer.push(&stills[0], 10)?;
writer.push(&stills[1], 20)?;
assert!(writer.finish()?.starts_with(b"GIF89a"));
# Ok::<(), anyhow::Error>(())
```

Nothing is started: no ffmpeg, no browser. License: MIT OR Apache-2.0.
