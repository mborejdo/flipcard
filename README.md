# flipcard

A Markdown deck in, a flip-card film out: every slide a card drawn by Typst, wide (16:9) or tall
(9:16) for a phone, every key press (a `+` bullet, a lit `steps` node) a build-in of its own. On a
phone the words come in on a beat: they rise, slide, fade or pop, metrics count up, a marker draws
under the heading. Pure Rust, nothing started: no ffmpeg, no browser.

flipcard writes GIF and MP4; other tools embed them.

The deck is a [presiterm](https://github.com/mborejdo/presiterm) deck, so the same Markdown also
presents in WezTerm.

## Install

```sh
cargo install --git https://github.com/mborejdo/flipcard flipcard
```

## Usage

```sh
flipcard talk/slides.md                          # talk/slides.gif
flipcard talk/slides.md --gif --mp4              # both
flipcard talk/slides.md --out dist --cards dist/cards
flipcard talk/slides.md --format tall --mp4      # 1080x1920 at 30 fps, for a phone
flipcard talk/slides.md --format tall --preset slide
flipcard lint talk/slides.md                     # does every card fit
flipcard config talk/slides.md                   # which flipcard.toml applies
```

`flipcard --help` lists every flag (`--hold 5`, `--transition wipe`, ...) with an example and the
exit codes. [docs/guide.md](docs/guide.md): what each part of a deck becomes, the files, timing,
fonts. [`examples/film`](examples/film) shows the build-ins: bullets, marked text, metrics, a
stepped diagram, code; [`examples/phone`](examples/phone) a short tall film: the
motion presets and every mark.

## Config

`flipcard.toml` next to the deck or in a folder above it; the closest wins. Every key is
optional; frontmatter beats it, a flag beats both.

```toml
fonts = ["fonts"]         # font files or folders Typst draws with, relative to this file
logo  = "logo.svg"        # top left corner of every card

[theme.light]             # design tokens; the rest keep their defaults
accent = "#2b2459"

[theme.fonts]
display = "Inter"         # headings, in the heaviest weight the family has

[film]
hold = 5                  # also format, safe, preset, fps, width, enter, fade, transition,
                          # pace, colours, gif-width, gif-fps
```

## How it works

- `deckmark` (from presiterm) reads the deck into slides of blocks.
- `flipkit` draws every slide at every build-in instant as a Typst card (`film.typ`, in the
  deck's tokens) and times the scenes. With a motion preset Rust owns the time: the card is laid
  out once at rest, and per frame a small JSON of word looks moves, scales and fades the words
  inside their boxes (`typ/motion.typ`), so nothing reflows. Frames are drawn side by side on
  one Typst engine and stream to the GIF and MP4 encoders on threads of their own; a
  frame that stays is drawn once.
- `filmstrip` writes the GIF (one learned palette, delta frames) and H.264 in an MP4.
- Same deck, same flags, same bytes: `tests/golden.rs` keeps the SHA-256 of every file of both
  examples, `tests/golden/phone.jsonl` every state of the phone film (`FLIPCARD_BLESS=1` rewrites
  them).
- Design note: [docs/design.md](docs/design.md).

## Workspace

| crate | what |
|---|---|
| [`apps/flipcard`](apps/flipcard) | the command line |
| [`packages/flipkit`](packages/flipkit) | the film as a library: `Config`, `Options`, `render`, `check`, `Rendered` |
| [`packages/filmstrip`](packages/filmstrip) | frames to GIF (delta frames), MP4 (pure-Rust H.264), streamed; scene transitions |

`deckmark`, `colonmark`, `swatchbook` and `typstkit` come from presiterm by git tag.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
cargo deny check
cargo run -p flipcard -- lint examples/film/slides.md
cargo run -p flipcard -- examples/film/slides.md --gif --mp4 --out target/demo
cargo test --release -p flipkit -- --ignored bench  # under 10 s per 30 s of tall film
```

## License

MIT OR Apache-2.0, at your option: [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE).
H.264 is covered by patent pools in some countries, so the MP4 is opt-in.
