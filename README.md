# flipcard

A Markdown script in, a flip-card film out. Every slide is a 16:9 card drawn by Typst; every key
press of the deck (a `+` bullet, a lit `steps` node) is a build-in of its own. Out come a looping
GIF, an MP4, a Word file that plays the GIF and a PDF that plays the MP4. Pure Rust: no ffmpeg,
no browser, nothing started.

The script is a [presiterm](https://github.com/mborejdo/presiterm) deck: the same Markdown
presents in WezTerm with presiterm and renders as a film here.

## Install

```sh
cargo install --git https://github.com/mborejdo/flipcard flipcard
```

## Usage

```sh
flipcard talk/slides.md                          # talk/slides.gif
flipcard talk/slides.md --gif --mp4 --docx --pdf # all four
flipcard talk/slides.md --out dist --cards dist/cards
flipcard talk/slides.md --hold 5 --transition wipe
flipcard lint talk/slides.md                     # does every card fit
flipcard config talk/slides.md                   # which flipcard.toml applies
```

`flipcard --help` has every flag with an example and the exit codes. [docs/guide.md](docs/guide.md)
has what each part of a deck becomes, the files, the timing and the fonts.

## Config

`flipcard.toml` next to the deck (or in a folder above it; the closest wins). Every key is
optional; a frontmatter key beats it, a flag beats both.

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

## Examples

```sh
flipcard examples/film/slides.md --gif --mp4 --docx --pdf --out target/demo
```

`examples/film` shows the build-ins: bullets, marked text, metrics, a stepped diagram, code.

## How it works

`deckmark` (from presiterm) reads the deck into slides of blocks. `flipkit` draws every slide at
every build-in instant as a Typst card (`film.typ`, in the deck's tokens) and times the scenes.
`filmstrip` turns the frames into the containers: a GIF with one learned palette and delta
frames, H.264 in an MP4, a Word file whose picture is the GIF, a PDF page with a Screen annotation
that plays the MP4 on open. The same deck and the same flags give the same bytes.

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
cargo deny check
```

CI runs these, then lints and renders the example. A parity job renders the example with
presiterm v0.2.0 too and compares every file byte for byte.
[docs/design.md](docs/design.md) is the design note.

## License

MIT OR Apache-2.0, at your option: [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE).

H.264 is covered by patent pools in some countries; the MP4 and the PDF are opt-in for that reason.
