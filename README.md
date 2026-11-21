# flipcard

A Markdown deck in, a flip-card film out: every slide a 16:9 card drawn by Typst, every key press
(a `+` bullet, a lit `steps` node) a build-in of its own. As a looping GIF, an MP4, a Word file
that plays the GIF and a PDF that plays the MP4. Pure Rust, nothing started: no ffmpeg, no
browser.

The deck is a [presiterm](https://github.com/mborejdo/presiterm) deck, so the same Markdown also
presents in WezTerm.

## Install

```sh
cargo install --git https://github.com/mborejdo/flipcard flipcard
```

## Usage

```sh
flipcard talk/slides.md                          # talk/slides.gif
flipcard talk/slides.md --gif --mp4 --docx --pdf # all four
flipcard talk/slides.md --out dist --cards dist/cards
flipcard lint talk/slides.md                     # does every card fit
flipcard config talk/slides.md                   # which flipcard.toml applies
```

`flipcard --help` lists every flag (`--hold 5`, `--transition wipe`, ...) with an example and the
exit codes. [docs/guide.md](docs/guide.md): what each part of a deck becomes, the files, timing,
fonts. [`examples/film`](examples/film) shows the build-ins: bullets, marked text, metrics, a
stepped diagram, code.

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
hold = 5                  # also fps, width, enter, fade, transition, pace, colours
```

## How it works

- `deckmark` (from presiterm) reads the deck into slides of blocks.
- `flipkit` draws every slide at every build-in instant as a Typst card (`film.typ`, in the
  deck's tokens) and times the scenes.
- `filmstrip` writes the GIF (one learned palette, delta frames), H.264 in an MP4, a Word file
  whose picture is the GIF, and a PDF page whose Screen annotation plays the MP4 on open.
- Same deck, same flags, same bytes. CI also renders the example with presiterm v0.2.0 and
  compares every file byte for byte.
- Design note: [docs/design.md](docs/design.md).

## Workspace

| crate | what |
|---|---|
| [`apps/flipcard`](apps/flipcard) | the command line |
| [`packages/flipkit`](packages/flipkit) | the film as a library: `Config`, `Options`, `render`, `check`, `Rendered` |
| [`packages/filmstrip`](packages/filmstrip) | frames to GIF (delta frames), MP4 (pure-Rust H.264), DOCX and PDF; scene transitions |

`deckmark`, `colonmark`, `swatchbook` and `typstkit` come from presiterm by git tag.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
cargo deny check
cargo run -p flipcard -- lint examples/film/slides.md
cargo run -p flipcard -- examples/film/slides.md --gif --mp4 --docx --pdf --out target/demo
```

## License

MIT OR Apache-2.0, at your option: [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE).
H.264 is covered by patent pools in some countries, so the MP4 and the PDF are opt-in.
