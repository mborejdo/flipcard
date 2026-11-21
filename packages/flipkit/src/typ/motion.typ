// Motion, ahead of film.typ: one word at one instant. sys.inputs.state (JSON) holds a look per
// word id (o opacity, x and y offset in em, s scale, draw how far its mark is drawn, tint toward
// the accent; a missing field is at rest) and the counted metric values (n). The word's box has
// its final size; the look moves, scales and fades the glyphs inside it, so nothing reflows.

#let motion = json(bytes(sys.inputs.at("state", default: "{}")))
#let motion-accent = rgb(json(bytes(sys.inputs.theme)).accent)

// `body` as word `w`; `final` is what it shows at rest when that differs (a counted value);
// `band` draws a marker stroke behind the glyphs, `under` a line under them.
#let word(w, body, final: none, band: false, under: false) = context {
  let look = motion.at("w", default: ()).at(w.id, default: (:))
  let (o, x, y, s, draw, tint) = ("o", "x", "y", "s", "draw", "tint").zip((1, 0, 0, 1, 1, 0)).map(((k, d)) => look.at(k, default: d))
  if o == 0 { return hide(if final == none { body } else { final }) }
  let size = measure(if final == none { body } else { final })
  let fill = if tint > 0 { color.mix((text.fill, (1 - tint) * 100%), (motion-accent, tint * 100%)) } else { text.fill }
  let fill = if o < 1 { fill.transparentize((1 - o) * 100%) } else { fill }
  box(width: size.width, align(left, {
    if band and draw > 0 { place(left + bottom, dy: -0.08em, rect(width: size.width * draw, height: 0.42em, fill: motion-accent.transparentize(70%))) }
    if under and draw > 0 { place(left + bottom, dy: 0.2em, line(length: size.width * draw, stroke: 0.09em + motion-accent.transparentize((1 - o) * 100%))) }
    move(dx: x * 1em, dy: y * 1em, scale(s * 100%, text(fill: fill, body)))
  }))
}
