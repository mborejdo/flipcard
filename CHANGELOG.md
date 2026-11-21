# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## Unreleased

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

### Changed

- Frames stream to the encoders in chunks, drawn side by side on one Typst engine per worker; a
  frame that stays is drawn once. A long film no longer sits in memory.
- `flipkit::render` takes the `Outputs` to encode; `Rendered` holds the stills, the GIF and the
  MP4.
- The H.264 level follows the macroblocks per frame and second (3.0 for 640x360, 4.0 for
  1080x1920 at 30 fps).
- The Word and PDF pages of a tall film are A4 portrait.
- Tall defaults to `rise` and big centred text (on the 360 pt canvas: heading 40, text 26, metric
  64 with its label at 18, footer 12; metrics one per row); wide stays `flip`. `hold` is a floor, unset by default (7 s for `flip`).
- `gif::Writer::push` and `video::Writer::push` take how often the frame repeats; `shots` takes
  ticks per scene. The GIF and MP4 encode on threads of their own while frames are drawn; a frame
  whose state didn't change is not drawn again.
- The phone example shows every preset, a marker, a pause, metrics and a slide that pages on.

The wide example is the same bytes as before.

## 0.1.0 - unreleased

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
