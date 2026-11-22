// Motion, ahead of film.typ: one word at one instant. sys.inputs.state (JSON) holds a look per
// word id (o opacity, x and y offset in em, s scale, draw how far its mark is drawn, chars how
// much is typed, fill how far ink has filled it from muted, tint toward its mark's colour; a
// missing field is at rest), the counted metric values (n), their bars while one draws (m: draw,
// hot, by value), the word the caret follows (c) and the tall heading's bar (bar: draw, hot, dy
// in points); film.typ draws the bars.
// The word's box has its final size; the look moves, scales and fades the glyphs inside it, so
// nothing reflows. Every word leaves `metadata("id width height size") <word>` (points) at its
// box's top left: where the words are, for the kinetic presets.

#let motion = json(bytes(sys.inputs.at("state", default: "{}")))
#let motion-theme = json(bytes(sys.inputs.theme))
#let (motion-accent, motion-muted) = (rgb(motion-theme.accent), rgb(motion-theme.muted))

// `body` as word `w`, then its glued punctuation `tail`, unmarked; `final` is what it shows at
// rest when that differs (a counted value), `typed(n)` its first n letters as body and tail, laid
// out where they are at rest. Its marks, drawn from the left as `draw` grows: `==` a marker
// stroke just under the baseline in a `heading`, else an accent band behind the glyphs; `__` a
// line below; `~~` a line through, the word turning muted; `**` turns it to the accent. The marks
// move with the glyphs; only the glyphs scale. A mark
// on a `bridge` word runs on over the space to the next. A `dot` (a list marker) only moves with
// its word.
#let word(w, body, tail: "", final: none, typed: none, heading: false, dot: false) = context {
  let look = motion.at("w", default: ()).at(w.id, default: (:))
  let (o, x, y, s, draw, chars, fill, tint) = ("o", "x", "y", "s", "draw", "chars", "fill", "tint").zip((1, 0, 0, 1, 1, 1, 1, 1)).map(((k, d)) => look.at(k, default: d))
  let rest = if final == none { [#body#tail] } else { final }
  let (size, marked) = (measure(rest), measure(body).width)
  if w.at("bridge", default: false) { marked += measure(text(" ")).width }
  let spot = if not dot {
    [#metadata((w.id, size.width / 1pt, size.height / 1pt, text.size.to-absolute() / 1pt).map(str).join(" ")) <word>]
  }
  if o == 0 or chars == 0 { return box(width: size.width, { spot; hide(rest) }) }
  let mark(key) = not dot and w.at(key, default: false)
  let target = if mark("strike") { motion-muted } else if mark("bold") { motion-accent }
  let ink = if target == none or tint == 0 { text.fill } else if tint >= 1 { target } else {
    color.mix((text.fill, (1 - tint) * 100%), (target, tint * 100%))
  }
  let alpha(c) = if o < 1 { c.transparentize((1 - o) * 100%) } else { c }
  let letters = (w.at("text", default: "") + w.at("tail", default: "")).codepoints().len()
  let (body, tail) = if chars < 1 and typed != none { typed(int(calc.round(chars * letters))) } else { (body, tail) }
  // karaoke: ink from the left over muted, a hard edge at `fill`
  let paint = if dot or fill >= 1 { alpha(ink) } else if fill <= 0 { motion-muted } else {
    gradient.linear((ink, 0%), (ink, fill * 100%), (motion-muted, fill * 100%), (motion-muted, 100%))
  }
  let caret = if not dot and motion.at("c", default: none) == w.id {
    box(width: 0pt, place(left + bottom, dx: 0.04em, dy: 0.15em, rect(width: 0.07em, height: 0.95em, fill: motion-accent)))
  }
  let line-at(dy, stroke) = place(left + bottom, dy: dy, line(length: marked * draw, stroke: stroke))
  // the marks move with the glyphs (a stacked line still rising); only the glyphs scale
  box(width: size.width, align(left, {
    spot
    move(dx: x * 1em, dy: y * 1em, box({
      if draw > 0 and mark("highlight") {
        if heading { line-at(0.08em, (paint: motion-accent, thickness: 0.1em, cap: "round")) } else {
          // a bridged band ends where the next one starts
          let pad = if w.at("bridge", default: false) { 0em } else { 0.16em }
          place(left + bottom, dx: -0.08em, dy: 0.22em, rect(width: marked * draw + pad, height: 1.05em, radius: 0.12em, fill: motion-accent.transparentize(70%)))
        }
      }
      if draw > 0 and mark("underline") { line-at(0.2em, 0.09em + alpha(motion-accent)) }
      let plain = if fill < 1 { paint } else { alpha(text.fill) }
      scale(s * 100%, box(text(fill: paint, body) + text(fill: plain, tail) + caret))
      if draw > 0 and mark("strike") { line-at(-0.3em, 0.08em + alpha(motion-muted)) }
    }))
  }))
}
