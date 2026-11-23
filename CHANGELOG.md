# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## Unreleased

### Added

- Presets `wave` (the letters hop up and back in a ripple), `drop` (they fall into place one after
  the other, landing on a spring) and `decode` (each letter a scramble of its kind that settles
  from the left, the same on every run). `examples/letters` shows them.
- `heading-preset` (frontmatter, `[film]`, `--heading-preset`, the same precedence as `preset`):
  the heading comes in by a preset of its own, the text by `preset` or the scene's. `preset: type`
  with `heading-preset: rise`: the heading rises with its bar, then the text is typed. Unset, as
  before.

### Changed

- `stack`: the lines move up on a damped spring (`motion/spring.rs`: closed form for under-,
  critically and over-damped, set by response and damping fraction as SwiftUI does), carrying
  their momentum into the next line's move instead of each line's ease adding up on its own. A
  stacked scene rests a little later, once its spring has. `drop` lands on the same physics.
- The bounce curve of `slide` and the bold pulse is named `Bounce`, its bytes unchanged.
- A word is its letters (grapheme clusters, as Typst cuts them): the per-frame look of a word
  carries a look per letter (`l`) while they differ; letters alike move the word as one, so every
  other preset draws as before, byte for byte.
- `type` reveals the letters of a word laid out once at rest with its full text, each where the
  word's shaping puts it (kerning kept); the caret is placed after the letters typed so far
  (`c`: word and letters), outside the flow, and shows on a word not begun yet too.

### Fixed

- `type`: the caret no longer drops onto a line of its own at a word's end, which made the word's
  box a line taller: the marks were drawn a line lower and the block jumped up while the caret
  showed. Every word stays where it is at rest on every frame (a test checks it per letter
  preset).
- A highlighted run of words (`==Pending review==`) is one band without a seam: only its ends are
  rounded.

## 0.4.0 - 2026-11-22

### Added

- `pad: square` (`--pad square`, `[film] pad`): a tall film on a 1:1 canvas, the tall frame a
  full-height panel over a blurred, toned-down copy of itself with a glow in the accent colour,
  a vignette and, in the MP4, a fine grain that moves every 4th frame. Tall only; the safe area
  then defaults to none.
- filmstrip: `gif::Repeat` and `gif::Writer::repeat`: a GIF that plays once and rests on its last
  frame (`Repeat::Once`, no NETSCAPE2.0 block). `Repeat::Forever` stays the default, byte for
  byte.
- filmstrip: `pad::Square`, the compositor, and `video::Picture`, a frame converted for the
  encoder ahead of it.

### Changed

- Marks draw just after their word lands, over 0.7 s; words marked alike in a row draw as one
  sweep from the left instead of each on its own.
- The heading's bar on a tall or square card draws as a laser: a soft accent glow, a white-hot
  tip and a short streak, the tip fading once it is across.
- Tall and square cards: body lines and list items 1.33 apart (were about 1.28 and 1.18); the
  bar sits midway between the heading and the text.
- `:::{metrics}` in motion: each value's bar draws up from the bottom over two beats as the value
  starts to count and moves with it (rise, stack); on a tall or square card as a laser, like the
  heading's bar. Wide `flip` films keep their static bars.
- `flipcard lint` tags each line with its level, `slide 3 (Title): error: the card overflows
  onto 2 pages; trim it` and `slide 3 (Title): warning: 31 words on one phone screen, ...`, and
  ends with `N errors, N warnings` instead of `N of M cards overflow`. Exit codes stay.
- Messages: `frontmatter: fps: ...` and `logo: <path>: missing image` (a colon after the
  location); an unknown preset or transition reads `expected one of a, b or c, got "x"`; a folder
  that can't be created names it (`create <dir>: ...`).
- `--help` writes defaults as clap does: `[default: 10, 30 for tall and square]`.
- flipkit: `Format`, `Safe` and `Preset` parse to `flipkit::ParseNameError` and `Transition` to
  `filmstrip::ParseNameError` (were `anyhow::Error`); both implement `std::error::Error`.
- flipkit: `Format::is_phone` (tall or square) and `PHONE_WORDS`, the words a phone screen holds
  before `flipcard lint` warns; `Overflow::place` and `Overflow::reason`, the two halves of its
  line.
- flipkit: the card layout moved to `src/typ/film.typ`, next to `motion.typ`. Pixels unchanged.

### Fixed

- Motion: a metric's bar no longer shows at its resting place from the scene's first frame, cutting
  through the heading and the transition while the stacked lines were still held low.
- Square films take the phone look: big centred words that come in by a preset (`rise` by
  default), the bar under the heading, page-on, and the lint's word count.
- A run of bold words only tints; a lone bold word still pulses. Pulsing together, the words
  grew into each other's spaces.
- `stack`: the marks (`==`, `__`, `~~`, `**`) draw once their line has landed and move up with
  it. They were drawn where the word ends up, away from the still rising line, and only matched
  once every line was in. The heading's marker and bar no longer wait for the last line.
- `pad: square` with options built by hand for a wide film is an error, not a panic.
- A frame whose bytes don't match its size is named as such by the video encoder, not as odd
  dimensions.

## 0.3.0 - 2026-11-21

### Added

- Presets `type` (letter by letter at the pace, an accent caret blinking on the beat) and
  `karaoke` (every word in muted, filled with ink as it is read). Both breathe half a beat after
  `, ; :` and a beat after `. ! ?`.
- Marks in motion, drawn once the word has landed: `**bold**` turns to the accent and pulses,
  `==marked==` gets an accent band, `__underlined__` a line from the left, `~~struck~~` a line
  through and muted.
- `examples/phone` shows every mark, `type`, `karaoke`, `move` and `stack`.
- Presets `move` (magic move: a word the scene before shows too glides from where it was, at its
  size then; the others come in after) and `stack` (line by line from below at the pace, the
  lines in so far moving up). Word positions come from Typst (typstkit `Engine::query`).

### Fixed

- A follow-on page's heading stays where the slide's first page has it.
- A mark over several words (`~~big bang~~`) runs on over the spaces between them.
- A wrapped list item hangs under its first word, not under its bullet.

### Changed

- On presiterm v0.4.0 (typstkit `Engine::query`).
- On presiterm v0.3.0: deckmark keeps `==`, `__` and `~~` apart and strips `//`, so flipkit no
  longer reads headings itself.
- One Typst engine draws every frame side by side; Typst's cache is evicted per chunk. No engine
  pool.
- Wide films set headings and metric values in Inter Display Black too. The wide example is
  current; the goldens are blessed anew.

### Removed

- The CI parity job against presiterm v0.2.0: the goldens keep the bytes.

## 0.2.0 - 2026-11-21

### Added

- `format: wide | tall | square` (also `16:9`, `9:16`, `1:1`), `safe: none | phone`,
  `gif-width`, `gif-fps`: frontmatter, `[film]` and `--format --safe --gif-width --gif-fps`. Tall
  is 1080x1920 at 30 fps in the phone's safe area, its GIF a 360 px preview at 15 fps.
- `examples/phone`, a short tall film.
- `filmstrip`: `gif::Writer` and `video::Writer` (frame by frame), `Frame::downscale`, `shots`.
- Golden SHA-256 of both examples (`FLIPCARD_BLESS=1` rewrites them); a bench at 1080x1920.
- Motion presets `rise`, `slide`, `fade`, `pop`, `still` next to `flip`: frontmatter `preset:`,
  `[film] preset`, `--preset`, and `:::{scene} <preset>` per slide with `:pace:` and `:beat:`.
  Words come in on a beat, block after block; `//` waits a beat. The card is laid out once and
  the words move inside their boxes, so nothing reflows.
- `==word==` in a heading draws an accent marker behind it once the words have landed; a tall
  heading without one gets a bar under it. Metrics rise and count up in their own format.
- A tall slide that doesn't fit pages on into a follow-on scene with its heading;
  `flipcard lint` warns about a tall slide with more than 12 words (`flipkit::crowded`).
- `tests/golden/phone.jsonl`: every state of the phone film.
- Tall and square films set headings and metric values in Inter Display Black (SIL OFL 1.1,
  built in as a Latin subset), tracked -0.02em; wide keeps the theme's `display`.

### Changed

- Frames stream to the encoders in chunks, drawn side by side on one Typst engine per worker; a
  frame that stays is drawn once. A long film no longer sits in memory.
- `flipkit::render` takes the `Outputs` to encode; `Rendered` holds the stills, the GIF and the
  MP4.
- The H.264 level follows the macroblocks per frame and second (3.0 for 640x360, 4.0 for
  1080x1920 at 30 fps).
- Tall defaults to `rise` and big centred text (on the 360 pt canvas: heading 40, text 26, metric
  64 with its label at 18, footer 12; metrics one per row); wide stays `flip`. `hold` is a floor, unset by default (7 s for `flip`).
- `gif::Writer::push` and `video::Writer::push` take how often the frame repeats; `shots` takes
  ticks per scene. The GIF and MP4 encode on threads of their own while frames are drawn; a frame
  whose state didn't change is not drawn again.
- The phone example shows every preset, a marker, a pause, metrics and a slide that pages on.

### Removed

- The Word file and the PDF: `--docx`, `--pdf`, `Rendered::docx`, `Rendered::pdf`,
  `Rendered::poster`, `strip_scheme`, filmstrip's `docx` and `pdf` features (docx-rs, zip,
  lopdf, typst-pdf). flipcard writes GIF and MP4; other tools embed them. Frontmatter `links:`
  is no longer shown anywhere.
- API nothing used: `filmstrip::sequence`, `Scene`, `Timing::ticks` (`shots` takes ticks per
  scene), `gif::encode`, `video::encode` (the `Writer`s do it), `Frame::solid`;
  `flipkit::Config::read`, `config::FILE`, `Rendered::options`, the `Deck`, `Look` and
  `filmstrip` re-exports.

The wide example's GIF, MP4 and cards are the same bytes as before.

## 0.1.0 - 2026-11-21

### Added

- `flipcard <deck.md>`: a looping GIF, `--mp4`, `--docx`, `--pdf`, `--cards DIR`, `--out DIR`,
  `--preview` (WezTerm or iTerm2), timing flags `--fps --width --hold --enter --fade
  --transition --pace --colours`; `flipcard lint`; `flipcard config`.
- `flipcard.toml`: `fonts`, `logo`, `[theme]`, `[film]`; the closest file wins.
- `flipkit`, the film as a library: `Config`, `Look`, `Settings`, `Options`, `render`, `check`,
  `Rendered` with `gif`, `mp4`, `docx`, `pdf`, `cards`.
- `filmstrip` from presiterm, with its history. Word file links in a fixed order, so a deck with
  several links gives the same bytes every run.

The film was `presiterm film` up to presiterm 0.2.0; same deck, same bytes here.
