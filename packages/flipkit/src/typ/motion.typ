// Motion, ahead of film.typ: one word at one instant. sys.inputs.state (JSON) holds a look per
// word id (o opacity, x and y offset in em, s scale, draw how far its mark is drawn, chars how
// much is typed, fill how far ink has filled it from muted, tint toward its mark's colour; a
// missing field is at rest), the counted metric values (n) and the word the caret follows (c).
// The word's box has its final size; the look moves, scales and fades the glyphs inside it, so
// nothing reflows.

#let motion = json(bytes(sys.inputs.at("state", default: "{}")))
#let motion-theme = json(bytes(sys.inputs.theme))
#let (motion-accent, motion-muted) = (rgb(motion-theme.accent), rgb(motion-theme.muted))

// `body` as word `w`; `final` is what it shows at rest when that differs (a counted value),
// `typed(n)` its first n letters. Its marks, drawn from the left as `draw` grows: `==` a marker
// stroke just under the baseline in a `heading`, else an accent band behind the glyphs; `__` a
// line below; `~~` a line through, the word turning muted; `**` turns it to the accent. A `dot`
// (a list marker) only moves with its word.
#let word(w, body, final: none, typed: none, heading: false, dot: false) = context {
  let look = motion.at("w", default: ()).at(w.id, default: (:))
  let (o, x, y, s, draw, chars, fill, tint) = ("o", "x", "y", "s", "draw", "chars", "fill", "tint").zip((1, 0, 0, 1, 1, 1, 1, 1)).map(((k, d)) => look.at(k, default: d))
  let rest = if final == none { body } else { final }
  if o == 0 or chars == 0 { return hide(rest) }
  let size = measure(rest)
  let mark(key) = not dot and w.at(key, default: false)
  let target = if mark("strike") { motion-muted } else if mark("bold") { motion-accent }
  let ink = if target == none or tint == 0 { text.fill } else if tint >= 1 { target } else {
    color.mix((text.fill, (1 - tint) * 100%), (target, tint * 100%))
  }
  let alpha(c) = if o < 1 { c.transparentize((1 - o) * 100%) } else { c }
  let letters = (w.at("text", default: "") + w.at("tail", default: "")).codepoints().len()
  let shown = if chars < 1 and typed != none { typed(int(calc.round(chars * letters))) } else { body }
  // karaoke: ink from the left over muted, a hard edge at `fill`
  let paint = if dot or fill >= 1 { alpha(ink) } else if fill <= 0 { motion-muted } else {
    gradient.linear((ink, 0%), (ink, fill * 100%), (motion-muted, fill * 100%), (motion-muted, 100%))
  }
  let caret = if not dot and motion.at("c", default: none) == w.id {
    box(width: 0pt, place(left + bottom, dx: 0.04em, dy: 0.15em, rect(width: 0.07em, height: 0.95em, fill: motion-accent)))
  }
  let line-at(dy, stroke) = place(left + bottom, dy: dy, line(length: size.width * draw, stroke: stroke))
  box(width: size.width, align(left, {
    if draw > 0 and mark("highlight") {
      if heading { line-at(0.08em, (paint: motion-accent, thickness: 0.1em, cap: "round")) } else {
        place(left + bottom, dx: -0.08em, dy: 0.22em, rect(width: size.width * draw + 0.16em, height: 1.05em, radius: 0.12em, fill: motion-accent.transparentize(70%)))
      }
    }
    if draw > 0 and mark("underline") { line-at(0.2em, 0.09em + alpha(motion-accent)) }
    move(dx: x * 1em, dy: y * 1em, scale(s * 100%, box(text(fill: paint, shown) + caret)))
    if draw > 0 and mark("strike") { line-at(-0.3em, 0.08em + alpha(motion-muted)) }
  }))
}
