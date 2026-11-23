// Motion, ahead of film.typ: one word at one instant. sys.inputs.state (JSON) holds a look per
// word id (o opacity, x and y offset in em, s scale, draw how far its mark is drawn, fill how far
// ink has filled it from muted, tint toward its mark's colour; a missing field is at rest) and,
// while its letters differ, a look per letter (l: o, x, y, s, g a letter drawn in its place), the
// counted metric values (n), their bars while one draws (m: draw, hot, by value), where the caret
// is (c: word id, letters before it) and the tall heading's bar (bar: draw, hot, dy in points);
// film.typ draws the bars.
// The word's box has its final size; the look moves, scales and fades the glyphs inside it, so
// nothing reflows. Every word leaves `metadata("id width height size") <word>` (points) at its
// box's top left: where the words are, for the kinetic presets.

#let motion = json(bytes(sys.inputs.at("state", default: "{}")))
#let motion-theme = json(bytes(sys.inputs.theme))
#let (motion-accent, motion-muted) = (rgb(motion-theme.accent), rgb(motion-theme.muted))

// `body` as word `w`, then its glued punctuation `tail`, unmarked; `final` is what it shows at
// rest when that differs (a counted value), `glyph(c)` a letter `c` of body in its style. Letters
// with looks of their own are each drawn where the word's shaping puts it at rest (its kerning
// kept), the word's box unchanged. Its marks, drawn from the left as `draw` grows: `==` a marker
// stroke just under the baseline in a `heading`, else an accent band behind the glyphs; `__` a
// line below; `~~` a line through, the word turning muted; `**` turns it to the accent. The marks
// move with the glyphs; only the glyphs scale. A mark
// on a `bridge` word runs on over the space to the next. A `dot` (a list marker) only moves with
// its word.
#let word(w, body, tail: "", final: none, glyph: none, heading: false, dot: false) = context {
  let look = motion.at("w", default: ()).at(w.id, default: (:))
  let (o, x, y, s, draw, fill, tint) = ("o", "x", "y", "s", "draw", "fill", "tint").zip((1, 0, 0, 1, 1, 1, 1)).map(((k, d)) => look.at(k, default: d))
  let letters = look.at("l", default: none)
  // a list marker moves with its word's first letter
  if dot and letters != none {
    let first = letters.at(0, default: (:))
    (o, x, y, s) = (o * first.at("o", default: 1), x + first.at("x", default: 0), y + first.at("y", default: 0), s * first.at("s", default: 1))
    letters = none
  }
  let rest = if final == none { [#body#tail] } else { final }
  let (size, marked) = (measure(rest), measure(body).width)
  if w.at("bridge", default: false) { marked += measure(text(" ")).width }
  let spot = if not dot {
    [#metadata((w.id, size.width / 1pt, size.height / 1pt, text.size.to-absolute() / 1pt).map(str).join(" ")) <word>]
  }
  // the word's letters: its own in its style, then its tail's; `upto(k)` its first k as one run,
  // shaped (kerned) as the word is
  let (own, after) = (w.at("text", default: "").clusters(), w.at("tail", default: "").clusters())
  let glyphs = if glyph == none { () } else { own.map(glyph) + after.map(text) }
  let upto(k) = glyph(own.slice(0, calc.min(k, own.len())).sum(default: "")) + text(after.slice(0, calc.max(0, k - own.len())).sum(default: ""))
  // the caret is placed after the letters typed so far, never laid out with them: inline, it
  // could break onto a line of its own in the word's box, the box growing a line taller
  let typed = motion.at("c", default: none)
  let caret = if not dot and glyph != none and typed != none and typed.at(0) == w.id {
    place(left + bottom, dx: measure(upto(typed.at(1))).width + 0.04em, dy: 0.15em, rect(width: 0.07em, height: 0.95em, fill: motion-accent))
  }
  if o == 0 { return box(width: size.width, { spot; hide(rest); caret }) }
  let mark(key) = not dot and w.at(key, default: false)
  let target = if mark("strike") { motion-muted } else if mark("bold") { motion-accent }
  let ink = if target == none or tint == 0 { text.fill } else if tint >= 1 { target } else {
    color.mix((text.fill, (1 - tint) * 100%), (target, tint * 100%))
  }
  let alpha(c, o: o) = if o < 1 { c.transparentize((1 - o) * 100%) } else { c }
  // karaoke: ink from the left over muted, a hard edge at `fill`
  let paint = if dot or fill >= 1 { alpha(ink) } else if fill <= 0 { motion-muted } else {
    gradient.linear((ink, 0%), (ink, fill * 100%), (motion-muted, fill * 100%), (motion-muted, 100%))
  }
  let line-at(dy, stroke) = place(left + bottom, dy: dy, line(length: marked * draw, stroke: stroke))
  let plain = if fill < 1 { paint } else { alpha(text.fill) }
  let glyphs-at-rest = if letters == none or glyphs.len() == 0 {
    box(text(fill: paint, body) + text(fill: plain, tail))
  } else {
    // each letter where the word's shaping puts it: the letters through it, less its own width
    let body-letters = own.len()
    box({
      hide(text(body) + text(tail))
      for (i, g) in glyphs.enumerate() {
        let l = letters.at(i, default: (:))
        let lo = o * l.at("o", default: 1)
        if lo > 0 {
          let at = measure(upto(i + 1)).width - measure(g).width
          // a stand-in sits centred on the letter it stands in for
          let (at, g) = if "g" in l {
            let by = if i < body-letters { glyph(l.g) } else { text(l.g) }
            (at + (measure(g).width - measure(by).width) / 2, by)
          } else { (at, g) }
          let fill = if i < body-letters { if fill >= 1 { alpha(ink, o: lo) } else { paint } } else { alpha(text.fill, o: lo) }
          place(left + bottom, dx: at, move(dx: l.at("x", default: 0) * 1em, dy: l.at("y", default: 0) * 1em, scale(l.at("s", default: 1) * 100%, text(fill: fill, g))))
        }
      }
    })
  }
  // the marks move with the glyphs (a stacked line still rising); only the glyphs scale
  box(width: size.width, align(left, {
    spot
    move(dx: x * 1em, dy: y * 1em, box({
      if draw > 0 and mark("highlight") {
        if heading { line-at(0.08em, (paint: motion-accent, thickness: 0.1em, cap: "round")) } else {
          // a run of words is one band: a bridged band ends square where the next starts square
          let (from, on) = (w.at("bridged", default: false), w.at("bridge", default: false))
          let (lpad, rpad) = (if from { 0em } else { 0.08em }, if on { 0em } else { 0.08em })
          let (l, r) = (if from { 0em } else { 0.12em }, if on { 0em } else { 0.12em })
          let radius = (top-left: l, bottom-left: l, top-right: r, bottom-right: r)
          place(left + bottom, dx: -lpad, dy: 0.22em, rect(width: marked * draw + lpad + rpad, height: 1.05em, radius: radius, fill: motion-accent.transparentize(70%)))
        }
      }
      if draw > 0 and mark("underline") { line-at(0.2em, 0.09em + alpha(motion-accent)) }
      scale(s * 100%, glyphs-at-rest)
      caret
      if draw > 0 and mark("strike") { line-at(-0.3em, 0.08em + alpha(motion-muted)) }
    }))
  }))
}
