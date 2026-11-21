# filmstrip

Frames in, films out. A list of RGB frames, whatever drew them, becomes:

- an animated GIF: one palette learned from the stills, delta frames (only the changed rectangle,
  unchanged pixels transparent), so a film of text cards stays small;
- an MP4, H.264 in pure Rust (`rusty_h264-encoder`, `mp4`), feature `mp4`;
- an A4 Word file (portrait for a tall film) with only the GIF and a line of links (Word plays the GIF in place),
  feature `docx`;
- a Screen annotation on a PDF page that plays the MP4 when it opens in Acrobat (other viewers
  show the page as is), feature `pdf`.

`gif::Writer` and `video::Writer` take the frames one by one (a frame and how often it repeats),
so a long film is never all in memory; the H.264 level follows the frame size and rate. `Frame::downscale` makes a GIF preview
of a large film.

`shots` plans a film from scenes (build-in frames, ticks per frame and a hold each) and `sequence` draws the plan: each build-in, the
hold on its last frame, a `Transition` into the next (fade, wipe, push, pixelize, circle, split
or cut; CPU-drawn, readable at 10 fps), and the last scene back into the first, so the loop
doesn't jump.

```rust
use filmstrip::{Frame, Scene, Timing, Transition};

let one = [Frame::solid(4, 2, [0; 3]), Frame::solid(4, 2, [40; 3])];
let two = [Frame::solid(4, 2, [200; 3])];
let scenes = [Scene { frames: &one, hold: 1.0 }, Scene { frames: &two, hold: 2.0 }];
let timing = Timing { fps: 10, fade: 0.2, transition: Transition::Wipe, ticks: 2 };
let frames = filmstrip::sequence(&scenes, timing)?;
let stills = [Frame::solid(4, 2, [40; 3]), Frame::solid(4, 2, [200; 3])];
let gif = filmstrip::gif::encode(&frames, &stills, 10, 16)?;
assert!(gif.starts_with(b"GIF89a"));
# Ok::<(), anyhow::Error>(())
```

Nothing is started: no ffmpeg, no browser. License: MIT OR Apache-2.0.
