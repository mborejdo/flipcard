# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## Unreleased

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
