# flipcard: a deck as GIF and MP4

```sh
flipcard talk/slides.md                      # talk/slides.gif
flipcard talk/slides.md --gif --mp4         # the GIF and the MP4
flipcard talk/slides.md --out dist           # into dist/ instead of next to the deck
flipcard talk/slides.md --cards dist/cards   # every card, fully built, as PNG
flipcard talk/slides.md --format tall        # 9:16 for a phone, words rise in
flipcard talk/slides.md --preset pop         # words pop in, on a wide card too
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

That is the `flip` preset, the default on a wide card. The other presets are below.

## Motion

A preset says how the words come in. Frontmatter `preset:`, `[film] preset` or `--preset` set it
for the film, `:::{scene}` for one slide:

```markdown
# One change at a time

:::{scene} slide
:pace: 160
:beat: 0.5
:::

- easy to review
```

| preset | the words |
|:--|:--|
| `flip` | the card build-in above (wide default) |
| `rise` | word by word, up from below as they fade in (tall and square default) |
| `slide` | word by word, in from the left, settling with a little spring |
| `fade` | block by block, fading in |
| `pop` | word by word, scaled up past their size and back |
| `still` | all there at once |
| `type` | letter by letter at `pace`, an accent caret blinking on the beat |
| `karaoke` | all there in muted, each word filled with ink as it is read at `pace` |
| `move` | magic move: a word the scene before shows too glides from there, at its size then; the others fade out in the transition and rise in after |
| `stack` | line by line from below, each read at `pace`; the lines in so far move up to make room |

Time runs in beats (0.4 s, `:beat:` changes it). The heading comes in first, then each block on
the beat after the one before has landed; inside a block a word follows the last by half a beat.
`type` and `karaoke` give each word its letters at `pace` (5.5 letters a word), half a beat more
after `, ; :` and a beat after `. ! ?`. `//` between words waits one more beat and is not shown. A
scene then holds for its words read at `pace` (after `type`, `karaoke` and `stack`: they were read
coming in), at least four beats; `hold` is the floor when set. `enter` is for `flip` only.

`move` and `stack` lay the card out once more to find where the words are. `move` matches words
by their text, each to the first unused one of the scene before; the glide takes a beat and a
half, then the other words come in. A later key press of a slide brings its words as `rise` does.

Marks draw once their word has landed (`stack`: its line), from the left over 0.7 s; words marked
alike in a row draw as one sweep:

- `**bold**`: turns to the accent; a lone bold word pulses, 1 to 1.12 and back on a spring over a
  beat.
- `==marked==`: an accent band sweeps in behind the words; the ink stays.
- `__underlined__`: a line draws from the left.
- `~~struck~~`: a line draws through it and it turns muted.

- `==word==` in a heading: an accent marker draws behind it from the left once the words have
  landed. A tall or square heading without one gets a bar under it instead, drawn as a laser: a
  soft glow and a white-hot tip that fades once it is across.
- `:::{metrics}`: the values rise and count up over three beats, in their own format (`1 200`,
  `60 %`, `3.5x`), the digits tabular so nothing jumps.
- Every word sits where it ends up: the card is laid out once at rest and the words move, scale
  and fade inside their boxes. A scene's last frame is its still.

On a tall or square card the text is big and centred, one idea per screen. On the 360 pt canvas:
heading 40, text and lists 26 (lines and items 1.33 apart), a metric 64 with its label at 18, the
footer 12; on a tall card metrics stack one per row. That holds about 12 words: `flipcard lint`
warns above. A slide that doesn't fit is not shrunk: it pages on into a follow-on scene with the
same heading, cut after a block, else after a list item or a paragraph line. Only when not even
that fits is it an overflow.

## Files

* `slides.gif`: one learned palette, only changed pixels stored (14 cards at 640 px: about
  1 MB). Loops.
* `slides.mp4` (`--mp4`): H.264, pure Rust, same frames.
* `--cards DIR`: every scene's fully built card as `card03-007.png` (slide 3, scene 7).

## Timing and size

Each value is a flag, a frontmatter key or a `[film]` key in `flipcard.toml`. Flag beats
frontmatter beats file beats the format's defaults.

| value | flag | default | what |
|:--|:--|--:|:--|
| format | `--format` | wide | `wide` (16:9), `tall` (9:16) or `square` (1:1) |
| preset | `--preset` | flip | how the words come in, see Motion; `rise` for tall and square |
| safe | `--safe` | none | `phone` keeps the content clear of a phone app's bars, captions and buttons |
| pad | `--pad` | none | `square` puts a tall film on a square canvas, see below |
| fps | `--fps` | 10 | frames per second |
| width | `--width` | 640 | pixels, even (the MP4 needs it); the height follows the format |
| hold | `--hold` | 7 | seconds every scene stays at least, build-in included (motion: unset) |
| enter | `--enter` | 1.6 | seconds of `flip` build-in at the start of every scene |
| fade | `--fade` | 0.4 | seconds a transition between scenes takes |
| transition | `--transition` | fade | `fade`, `wipe`, `push`, `pixelize`, `circle`, `split` or `cut` |
| pace | `--pace` | 200 | words per minute; a scene with more words than `hold` covers stays longer |
| colours | `--colours` | 48 | colours of the GIF palette, 2 to 255 |
| gif-width | `--gif-width` | width | the GIF scaled down by a whole factor, at least this wide |
| gif-fps | `--gif-fps` | fps | the GIF takes every n-th frame |

`tall` sets width 1080 (1080x1920), fps 30, fade 0.2, safe `phone`, preset `rise`, gif-width 360
and gif-fps 15: the MP4 is the film for the phone, the GIF a preview. `square` is the same at
1080x1080 without the safe area. The card's content sits inside the safe area, centred; what
doesn't fit there is an overflow, as on a wide card.

`pad: square` (tall only) makes a square film of a tall one: the tall frame scaled to the full
height as a panel in the middle, a 2 px edge in the theme's `border` colour left and right, and
bars of the same frame, blurred until no shape is left, toned down, lit by a glow in the theme's
`accent` colour that spills from the panel's sides and fades within about 250 px, and darkened
towards the edges, black in the corners. At 1080 the film is 1080x1080 with a 608 px panel. The
MP4 has a fine grain on the bars that moves every 4th frame (one pattern, shifted, so the encoder
and the file stay small); the GIF has none, as grain changes every pixel of the bars from frame
to frame and would bloat its delta frames. A padded film keeps the whole card: the safe area
defaults to `none` (a phone app's bars sit outside the square), `--safe phone` still applies.
The cards (`--cards`) stay tall.

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
hold: 5
enter: 1.2
---
```

```toml
[film]
width = 960
fps = 12
```

## Fitting

A card that doesn't fit is never cropped: `flipcard` stops with
`slide 4 (The chain): the card overflows onto 2 pages; trim it`, and `flipcard lint` lists every
such card. Fix: fewer words, a smaller picture (`{h=5}`), or a second slide. A tall or square
slide pages on first (see Motion).

## Fonts

Text in the theme's `sans`, code and footer in `mono` (`[theme.fonts]` in flipcard.toml). Inter and
Fira Code are built in; other families come from the font files the config names
(`fonts = ["fonts"]`).
Headings and metric values are set in Inter Display Black (SIL OFL 1.1, built in), whatever the
`display`; `display` is for subheadings.
