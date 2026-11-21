# The film: a deck as GIF and MP4

Design note, 06.11.2026, built the same day as `presiterm film`. Since 21.11.2026 the film is
flipcard: `flipkit` the film (on `deckmark`, presiterm's deck model as a package), `flipcard` the
command line. Names below are as written then. The Word and PDF editions of the first version
are gone: flipcard writes the GIF and the MP4, other tools embed them.

The film renders a deck to files that play on their own: a looping GIF for a README or a chat,
an MP4 for a post. Every key press of the talk is a build-in.

## Why in presiterm

presiterm had everything but the encoders:

| need | presiterm has |
|---|---|
| slides from Markdown with steps | `deck`: `+` bullets (`reveal`), `steps` diagrams, `{only}` tags |
| design tokens and fonts | `swatchbook`: palettes, Inter, Fira Code |
| drawing without a browser | `typstkit`: in-process Typst to PNG |
| lint before export | `presiterm lint` |

One deck, two outputs, same rules: pure Rust, no program started, no fallbacks (an overflowing
card is a lint error, not a cropped frame), help with examples, the same tokens.

## Shape

### `filmstrip` (`packages/filmstrip`)

Frames in, files out; knows nothing about Markdown or Typst.

- `Frame { width, height, rgb }`, `Frame::png()`.
- `sequence(scenes, timing) -> Vec<Frame>` with `Timing { fps, fade, transition, ticks }` and
  `Scene { frames, hold }`: each scene's build-in, its own hold on the last frame, a
  `Transition` into the next (fade, wipe, push, pixelize, circle, split, cut; CPU-drawn,
  readable at 10 fps).
- `gif::encode(frames, keyframes, fps, colours)`: one NeuQuant palette from the keyframes plus a
  transparent slot, delta frames (changed rectangle only), long holds split into legal delays.
  Keeps a 14-card film near 1 MB.
- `video::encode(frames, fps)` (feature `mp4`): H.264 via `rusty_h264-encoder`, boxed by `mp4`,
  no B-frames.

Features `gif` (default) and `mp4`, each pulls only its encoder. Tests per container (GIF frames
and delays, MP4 sample table), read back with the crate's own readers.

### `typstkit::Engine::raster`

The film needs the pixels of a compile with `sys.inputs` (slide, step, frame): a new method
returns width, height and RGB bytes, `png()` wraps it. Pictures reach Typst by path under the
engine's root, as for diagrams.

### `presiterm film` (feature `film`, needs `typst`)

    presiterm film talk/slides.md                 talk/slides.gif
    presiterm film talk/slides.md --mp4
    presiterm film talk/slides.md --cards out/    every card and build-in sample as PNG
    presiterm film talk/slides.md --preview       play the GIF inline in this WezTerm

An embedded Typst layout draws one slide at one build-in instant from the slide as JSON
(`deck::Slide`), `index`/`total`, `step`/`steps`, `frame`/`frames`, the token colours and the
width. It mirrors the terminal: accent heading with its rule, body from the left margin, footer
with title and `n / N`, corner logo; `theme:` picks the palette, `[theme]` applies.

| in the terminal | in the film |
|---|---|
| a key press (a `+` bullet, a lit `steps` node) | one build-in step: `enter` seconds of animation, then `hold` |
| `- ` bullets shown at once | appear one by one along the step's ease curve |
| `==marked==` / `__marked__` | the underline grows from left to right |
| a title slide | the heading lands word by word |
| a mermaid diagram with `steps` | drawn per step by the existing renderer, placed as a picture |
| a Typst diagram with `steps` | the same `stepnode` preamble with `step` set per frame |
| a table | the existing Typst table, rows appearing |
| `:::{notes}` | never in the film |
| `{only}` tags | the same `--tags` as the window |

Timing: `[film] fps = 10, width = 640, hold = 7, enter = 1.6, fade = 0.4, transition = "fade",
pace = 200, colours = 48` in `presiterm.toml`, same keys in the frontmatter, a flag beats both.
`hold` is the minimum; a scene with more words than that covers at `pace` words per minute stays
for its reading time. `presiterm lint --film` checks every slide at the film's geometry and
names the rows that overflow.

Planned directives, as talks want them and the film gives them motion: `:::{metrics}` (`value |
label` lines, big numbers in a row, counted up in the film) and `:::{columns}` (a picture beside
text). Only `metrics` was built.

## What was built

All of the above, plus `Input::Bytes`, the card layout (`apps/presiterm/src/film/film.typ`),
`--out` and the eight timing flags, `presiterm handbook film`, `metrics`
in dialect, window and film, and `examples/film`, rendered by `cargo xtask demo`.

Left out on purpose: `columns` (a picture above its text does the job), per-element timing in
the Markdown, audio, a transition per slide, GPU transitions, any browser.
