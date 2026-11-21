# flipcard: a deck as GIF, MP4, DOCX and PDF

```sh
flipcard talk/slides.md                      # talk/slides.gif
flipcard talk/slides.md --mp4 --docx --pdf   # the MP4, a Word file, a PDF
flipcard talk/slides.md --out dist           # into dist/ instead of next to the deck
flipcard talk/slides.md --cards dist/cards   # every card, fully built, as PNG
flipcard talk/slides.md --format tall        # 9:16 for a phone
flipcard talk/slides.md --preview            # play the GIF inline in WezTerm or iTerm2
flipcard lint talk/slides.md                 # which cards do not fit
```

A presiterm deck renders as a film: every slide a card in the deck's tokens (`[theme]`,
`theme: light`), drawn in process by Typst. Every key press is a build-in:

| in the deck | in the film |
|:--|:--|
| a key press (a `+` bullet, a lit `steps` node) | one scene: `enter` seconds of build-in, then `hold` seconds of rest |
| `-` bullets shown at once | appear one after the other during the build-in |
| `==marked==` and `__marked__` | an accent underline that grows from the left |
| the first slide, or a heading alone | the heading lands word by word |
| a `steps` diagram, mermaid or Typst | the step's node lights up |
| `:::{metrics}` | the numbers count up from zero |
| a picture, a table, a code block | drawn in the card |
| `:::{notes}` | never in the film |
| `{only}` blocks | follow the frontmatter's `tags:` |

## Files

* `slides.gif`: one learned palette, only changed pixels stored (14 cards at 640 px: about
  1 MB). Loops.
* `slides.mp4` (`--mp4`): H.264, pure Rust, same frames.
* `slides.docx` (`--docx`): A4 landscape (portrait for tall), the GIF and a line of links. Word for Mac and current
  Word for Windows play the GIF in place.
* `slides.pdf` (`--pdf`): A4 landscape (portrait for tall), the first card as poster, title line and links. Acrobat
  plays and loops the MP4 over it on open; other viewers show the poster.
* `--cards DIR`: every scene's fully built card as `card03-007.png` (slide 3, scene 7).

## Timing and size

Each value is a flag, a frontmatter key or a `[film]` key in `flipcard.toml`. Flag beats
frontmatter beats file beats the format's defaults.

| value | flag | default | what |
|:--|:--|--:|:--|
| format | `--format` | wide | `wide` (16:9), `tall` (9:16) or `square` (1:1) |
| safe | `--safe` | none | `phone` keeps the content clear of a phone app's bars, captions and buttons |
| fps | `--fps` | 10 | frames per second |
| width | `--width` | 640 | pixels, even (the MP4 needs it); the height follows the format |
| hold | `--hold` | 7 | seconds every scene stays, build-in included |
| enter | `--enter` | 1.6 | seconds of build-in at the start of every scene |
| fade | `--fade` | 0.4 | seconds a transition between scenes takes |
| transition | `--transition` | fade | `fade`, `wipe`, `push`, `pixelize`, `circle`, `split` or `cut` |
| pace | `--pace` | 200 | words per minute; a scene with more words than `hold` covers stays longer |
| colours | `--colours` | 48 | colours of the GIF palette, 2 to 255 |
| gif-width | `--gif-width` | width | the GIF scaled down by a whole factor, at least this wide |
| gif-fps | `--gif-fps` | fps | the GIF takes every n-th frame |

`tall` sets width 1080 (1080x1920), fps 30, fade 0.2, safe `phone`, gif-width 360 and gif-fps
15: the MP4 is the film for the phone, the GIF a preview. `square` is the same at 1080x1080
without the safe area. The card's content sits inside the safe area, centred; what doesn't fit
there is an overflow, as on a wide card.

`hold` is the minimum: a scene with more words than `hold` covers at `pace` (first step: the
heading and all shown at once; then each `+` bullet) stays for its reading time. `pace: 0` holds
every scene for exactly `hold`.

Transitions are CPU-drawn to read at 10 fps (a 0.4 s `fade` is four frames). `wipe` comes in
from the left, `push` shoves the current scene out to the left, `pixelize` breaks it into blocks
that sharpen into the next, `circle` opens from the centre, `split` from the centre line like
barn doors, `cut` has no frames between. One transition per film; `fade` sets its length.

```markdown
---
title: My talk
links: https://example.org/talk, mailto:me@example.org
hold: 5
enter: 1.2
---
```

```toml
[film]
width = 960
fps = 12
```

`links:` are comma-separated URLs, shown under the picture in the Word and PDF editions with the
scheme (`https://`, `mailto:`) stripped from the label.

## Fitting

A card that doesn't fit is never cropped: `flipcard` stops with
`slide 4 (The chain): the card overflows onto 2 pages; trim it`, and `flipcard lint` lists every
such card. Fix: fewer words, a smaller picture (`{h=5}`), or a second slide.

## Fonts

Text in the theme's `sans`, headings in `display` (its heaviest weight, so a Black face lands
there if you bring one), code and footer in `mono` (`[theme.fonts]` in flipcard.toml). Inter and
Fira Code are built in; other families come from the font files the config names
(`fonts = ["fonts"]`).
