//! A card: one page of a slide at one key press, ready to draw at any frame of its build-in.
//! Flip cards are timed by Typst (`frame` of `frames`); motion cards by a [`Timeline`], one
//! state per frame; kinetic ones by where their words are, too. A tall slide that doesn't fit
//! pages on into follow-on cards, the heading where the first page has it.

use anyhow::{Context, Result};
use deckmark::deck::{Block, Deck, Loaded, Slide, Span};
use deckmark::diagram::Assets;
use deckmark::fragments;
use serde_json::{Value, json};
use typstkit::{Engine, Input, Main};

use crate::motion::{Preset, Spot, Timeline};
use crate::script::{self, Scene};
use crate::{Format, Frame, Options, Overflow};

/// The card layout, `word()` first.
const CARD: &str = concat!(include_str!("typ/motion.typ"), include_str!("film.typ"));
/// Pixels per logical point: the card is drawn at twice its size for crisp text.
const SCALE: f32 = 2.0;
/// Seconds a flip scene stays unless `hold` says otherwise.
const HOLD: f32 = 7.0;
/// Frames each flip build-in frame stays: 2 at 10 fps is the stop-motion look.
const TICKS: usize = 2;
/// Seconds of a beat, unless a scene says otherwise.
const BEAT: f64 = 0.4;

/// A slide as it goes on screen: the whole slide, or one page of it.
#[derive(Clone, Debug)]
pub(crate) struct Page {
    slide: Slide,
    /// The slide's 0-based index in the deck.
    index: usize,
    /// A follow-on page: its heading is already there.
    more: bool,
    /// Points the heading moves down to sit where the slide's first page has it.
    lift: f64,
}

/// The pictures, diagrams and logo of a card as `sys.inputs` entries: key and bytes.
type Media = Vec<(String, Vec<u8>)>;

/// One scene of the film.
#[derive(Debug)]
pub(crate) struct Card {
    /// The slide's 0-based index in the deck.
    pub slide: usize,
    title: String,
    /// The `sys.inputs` strings, all but `frame` and `state`.
    text: Vec<(&'static str, String)>,
    bytes: Media,
    /// Every build-in frame's state (motion); empty for flip.
    pub states: Vec<String>,
    /// What the states come from (motion).
    timeline: Option<Timeline>,
    /// Build-in frames; each stays `ticks` frames, the scene `hold` seconds.
    pub frames: usize,
    pub ticks: usize,
    pub hold: f32,
}

impl Card {
    /// Draw build-in frame `frame`.
    pub(crate) fn draw(&self, engine: &Engine, frame: usize) -> Result<Frame> {
        let number = frame.to_string();
        let raster = engine
            .raster(
                Main::Source(CARD),
                &self.inputs(frame, &number),
                72.0 * SCALE,
            )
            .with_context(|| format!("slide {} ({})", self.slide + 1, self.title))?;
        Ok(Frame {
            width: raster.width,
            height: raster.height,
            rgb: raster.rgb,
        })
    }

    /// Pages the fully built card takes, a tall one with a body line to spare; 1 when it fits.
    fn pages(&self, engine: &Engine) -> Result<usize> {
        let last = self.frames - 1;
        let number = last.to_string();
        let mut inputs = self.inputs(last, &number);
        inputs.push(("fit", Input::Text("1")));
        let document = engine
            .document_with(Main::Source(CARD), &inputs)
            .with_context(|| format!("slide {} ({})", self.slide + 1, self.title))?;
        Ok(document.pages().len())
    }

    /// Where its words are at rest, by id (motion).
    fn spots(&self, engine: &Engine) -> Result<Vec<Option<Spot>>> {
        let last = self.frames - 1;
        let number = last.to_string();
        let found = engine
            .query(Main::Source(CARD), &self.inputs(last, &number), "word")
            .with_context(|| format!("slide {} ({})", self.slide + 1, self.title))?;
        let mut spots = Vec::new();
        for f in found.iter().filter(|f| f.page == 1) {
            if let Some((id, spot)) = Spot::read(&f.value, f.x, f.y) {
                spots.resize(spots.len().max(id + 1), None);
                spots[id] = Some(spot);
            }
        }
        Ok(spots)
    }

    /// The words it shows at rest, by their text, and where they are.
    fn shown(&self, engine: &Engine) -> Result<Vec<(String, Spot)>> {
        let Some(timeline) = &self.timeline else {
            return Ok(Vec::new());
        };
        let spots = self.spots(engine)?;
        let spot = |id: usize| spots.get(id).copied().flatten();
        Ok(timeline
            .shown()
            .filter_map(|c| Some((c.word.run.text.clone(), spot(c.word.id)?)))
            .collect())
    }

    /// The `sys.inputs` of build-in frame `frame`, `number` its digits.
    fn inputs<'a>(&'a self, frame: usize, number: &'a str) -> Vec<(&'a str, Input<'a>)> {
        let state = self.states.get(frame).map(|s| ("state", Input::Text(s)));
        self.text
            .iter()
            .map(|(key, value)| (*key, Input::Text(value)))
            .chain([("frame", Input::Text(number))])
            .chain(state)
            .chain(
                self.bytes
                    .iter()
                    .map(|(k, b)| (k.as_str(), Input::Bytes(b))),
            )
            .collect()
    }

    /// The build-in frame that looks like `frame`: the last one once all is at rest, else the
    /// first of a run of equal states.
    pub(crate) fn key(&self, frame: usize) -> usize {
        let (states, last) = (&self.states, self.frames - 1);
        if states.is_empty() {
            return frame;
        }
        if states[frame] == states[last] {
            return last;
        }
        let mut key = frame;
        while key > 0 && states[key - 1] == states[frame] {
            key -= 1;
        }
        key
    }
}

/// What every card of a deck is made from.
pub(crate) struct Maker<'a> {
    deck: &'a Deck,
    options: &'a Options,
    assets: Assets,
    logo: Option<Vec<u8>>,
    scenes: Vec<Scene>,
    theme: String,
}

impl<'a> Maker<'a> {
    pub(crate) fn new(loaded: &'a Loaded, options: &'a Options) -> Result<Self> {
        let deck = &loaded.deck;
        let logo = deck
            .logo
            .as_ref()
            .map(|path| {
                std::fs::read(path)
                    .with_context(|| format!("logo {}: missing image", path.display()))
            })
            .transpose()?;
        Ok(Self {
            deck,
            options,
            assets: Assets::new(&loaded.base, deck),
            logo,
            scenes: script::read(&loaded.source)?,
            theme: theme(deck),
        })
    }

    /// Every scene of the film: every page at every key press. A slide that doesn't fit is an
    /// error naming it (all of them). A kinetic card is timed by where its words are, and by
    /// where the card before has them.
    pub(crate) fn cards(&mut self, engine: &Engine) -> Result<Vec<Card>> {
        let (mut pages, overflows) = self.paged(engine)?;
        if !overflows.is_empty() {
            let lines: Vec<String> = overflows.iter().map(ToString::to_string).collect();
            anyhow::bail!("{}", lines.join("\n"));
        }
        self.pin(engine, &mut pages)?;
        let mut cards: Vec<Card> = Vec::new();
        for page in &pages {
            for step in 0..=fragments(&page.slide) {
                let mut card = self.card(page, step)?;
                if card.timeline.as_ref().is_some_and(Timeline::kinetic) {
                    let before = match cards.last() {
                        Some(last) if step == 0 => last.shown(engine)?,
                        _ => Vec::new(),
                    };
                    let spots = card.spots(engine)?;
                    if let Some(timeline) = card.timeline.as_mut() {
                        timeline.arrange(&spots, &before);
                        (card.states, card.frames, card.hold) = film(timeline, self.options);
                    }
                }
                cards.push(card);
            }
        }
        Ok(cards)
    }

    /// Lift the heading of every follow-on page to where its slide's first page has it.
    fn pin(&mut self, engine: &Engine, pages: &mut [Page]) -> Result<()> {
        let mut first = None;
        for i in 0..pages.len() {
            let split = pages[i].more || pages.get(i + 1).is_some_and(|p| p.more);
            if !split || pages[i].slide.heading.is_none() {
                continue;
            }
            // the heading's first word is word 0
            let spots = self.card(&pages[i], 0)?.spots(engine)?;
            let y = spots.first().copied().flatten().map(|s| s.y);
            match (pages[i].more, first, y) {
                (false, _, _) => first = y,
                (true, Some(first), Some(y)) => pages[i].lift = first - y,
                _ => {},
            }
        }
        Ok(())
    }

    /// The pages of every slide, and the slides that don't fit: a tall one pages on after as
    /// many blocks as fit, else after as many items or lines of its first block; a follow-on
    /// page carries the heading.
    pub(crate) fn paged(&mut self, engine: &Engine) -> Result<(Vec<Page>, Vec<Overflow>)> {
        let (mut pages, mut overflows) = (Vec::new(), Vec::new());
        for (index, slide) in self.deck.slides.iter().enumerate() {
            let mut page = Page {
                slide: slide.clone(),
                index,
                more: false,
                lift: 0.0,
            };
            loop {
                let spill = self.spill(engine, &page)?;
                if spill == 1 {
                    break;
                }
                let split = if self.options.format == Format::Tall {
                    self.split(engine, &page)?
                } else {
                    None
                };
                let Some((head, rest)) = split else {
                    overflows.push(Overflow {
                        slide: index,
                        title: slide.title().unwrap_or_default(),
                        pages: spill,
                    });
                    break;
                };
                pages.push(head);
                page = rest;
            }
            pages.push(page);
        }
        Ok((pages, overflows))
    }

    /// Pages `page` takes fully built.
    fn spill(&mut self, engine: &Engine, page: &Page) -> Result<usize> {
        self.card(page, fragments(&page.slide))?.pages(engine)
    }

    /// `page` in two, the first part fitting; `None` when not even one item or line fits.
    fn split(&mut self, engine: &Engine, page: &Page) -> Result<Option<(Page, Page)>> {
        let blocks = &page.slide.blocks;
        let with = |blocks: Vec<Block>, more: bool| Page {
            slide: Slide {
                blocks,
                ..page.slide.clone()
            },
            more,
            ..*page
        };
        let mut fit = 0;
        while fit + 1 < blocks.len()
            && self.spill(engine, &with(blocks[..=fit].to_vec(), page.more))? == 1
        {
            fit += 1;
        }
        if fit > 0 {
            return Ok(Some((
                with(blocks[..fit].to_vec(), page.more),
                with(blocks[fit..].to_vec(), true),
            )));
        }
        let Some(first) = blocks.first() else {
            return Ok(None);
        };
        let mut parts = 0;
        while let Some((head, _)) = cut(first, parts + 1) {
            if self.spill(engine, &with(vec![head], page.more))? != 1 {
                break;
            }
            parts += 1;
        }
        // as many pages as needed, filled evenly
        if parts > 0 {
            let total = len(first);
            parts = total.div_ceil(total.div_ceil(parts));
        }
        Ok(cut(first, parts).map(|(head, tail)| {
            let rest = std::iter::once(tail).chain(blocks[1..].iter().cloned());
            (with(vec![head], page.more), with(rest.collect(), true))
        }))
    }

    /// Page `page` at key press `step`.
    fn card(&mut self, page: &Page, step: usize) -> Result<Card> {
        let (o, slide) = (self.options, &page.slide);
        let scene = self.scenes.get(page.index).cloned().unwrap_or_default();
        let preset = scene.preset.unwrap_or(o.preset);
        let (mut card, mut bytes) = card_json(slide, page.index == 0, step, &mut self.assets, o)?;
        if let Some(logo) = &self.logo {
            bytes.push(("logo".into(), logo.clone()));
        }
        if page.lift != 0.0 {
            card["lift"] = page.lift.into();
        }
        let (states, frames, ticks, hold, timeline) = if preset == Preset::Flip {
            let frames = ((o.enter * f32::from(o.fps)).round() as usize).max(1);
            // the hold, or longer when reading the words at `pace` takes more
            let hold = o.hold.unwrap_or(HOLD);
            let read = words(slide, step) as f32 * 60.0 / o.pace;
            let hold = if o.pace > 0.0 { hold.max(read) } else { hold };
            (Vec::new(), frames, TICKS, hold, None)
        } else {
            let timeline = timeline(&mut card, page, step, &scene, preset, o);
            let (states, frames, hold) = film(&timeline, o);
            (states, frames, 1, hold, Some(timeline))
        };
        let [top, right, bottom, left] = o.safe.insets();
        let logical = |px: u32| (px as f32 / SCALE).round().to_string();
        let motion = !states.is_empty();
        let text = vec![
            ("card", card.to_string()),
            ("theme", self.theme.clone()),
            ("title", self.deck.title.clone()),
            ("index", page.index.to_string()),
            ("total", self.deck.slides.len().to_string()),
            ("step", step.to_string()),
            ("frames", if motion { 1 } else { frames }.to_string()),
            ("width", logical(o.width)),
            ("height", logical(o.height())),
            ("safe", format!("{top} {right} {bottom} {left}")),
        ];
        Ok(Card {
            slide: page.index,
            title: slide.title().unwrap_or_default(),
            text,
            bytes,
            states,
            timeline,
            frames,
            ticks,
            hold,
        })
    }
}

/// A motion card's timeline, its words numbered in `card`.
fn timeline(
    card: &mut Value,
    page: &Page,
    step: usize,
    scene: &Scene,
    preset: Preset,
    o: &Options,
) -> Timeline {
    card["motion"] = true.into();
    let mut cues = script::number(card);
    for cue in cues.iter_mut().filter(|c| page.more && c.block == 0) {
        cue.step = usize::MAX; // a follow-on page's heading is already there
    }
    let heading = page.slide.heading.as_deref();
    let marked = heading.is_some_and(|h| o.format == Format::Tall || h.iter().any(|s| s.highlight));
    let wpm = scene.pace.unwrap_or_else(|| f64::from(o.pace));
    let beat = scene.beat.unwrap_or(BEAT);
    Timeline::new(preset, beat, wpm, step, cues, marked)
}

/// The state of every build-in frame of `timeline`, how many there are and the seconds the scene
/// stays, on whole frames.
fn film(timeline: &Timeline, o: &Options) -> (Vec<String>, usize, f32) {
    let fps = f64::from(o.fps);
    let frames = timeline.entrances().mul_add(fps, -1e-6).ceil().max(0.0) as usize + 1;
    let states = (0..frames)
        .map(|k| timeline.state(k as f64 / fps))
        .collect();
    let length = timeline.length().max(o.hold.map_or(0.0, f64::from));
    (states, frames, ((length * fps).round() / fps) as f32)
}

/// Items or lines in `block`.
const fn len(block: &Block) -> usize {
    match block {
        Block::List(items) => items.len(),
        Block::Paragraph(lines) | Block::Quote(lines) => lines.len(),
        _ => 0,
    }
}

/// `block` cut after `n` of its items or lines, both parts non-empty.
fn cut(block: &Block, n: usize) -> Option<(Block, Block)> {
    let halves = |len: usize| (1..len).contains(&n);
    match block {
        Block::List(items) if halves(items.len()) => Some((
            Block::List(items[..n].to_vec()),
            Block::List(items[n..].to_vec()),
        )),
        Block::Paragraph(lines) if halves(lines.len()) => Some((
            Block::Paragraph(lines[..n].to_vec()),
            Block::Paragraph(lines[n..].to_vec()),
        )),
        Block::Quote(lines) if halves(lines.len()) => Some((
            Block::Quote(lines[..n].to_vec()),
            Block::Quote(lines[n..].to_vec()),
        )),
        _ => None,
    }
}

/// The words that come in at `step` of `slide`: at step 0 the heading and everything shown at
/// once, at step k the k-th `+` bullet; a lit diagram node brings none.
pub(crate) fn words(slide: &Slide, step: usize) -> usize {
    fn count(text: &str) -> usize {
        text.split_whitespace().count()
    }
    fn spans(spans: &[Span]) -> usize {
        count(&deckmark::deck::plain(spans))
    }
    let mut reveal = 0;
    let mut total = if step == 0 {
        slide.heading.as_deref().map_or(0, spans)
    } else {
        0
    };
    for block in &slide.blocks {
        total += match block {
            Block::List(items) => items
                .iter()
                .filter(|item| {
                    if item.reveal {
                        reveal += 1;
                        reveal == step
                    } else {
                        step == 0
                    }
                })
                .map(|item| spans(&item.spans))
                .sum(),
            _ if step != 0 => 0,
            Block::Paragraph(lines) | Block::Quote(lines) => lines.iter().map(|l| spans(l)).sum(),
            Block::Code { lines, .. } => lines.iter().map(|l| count(l)).sum(),
            Block::Table { head, rows, .. } => head
                .iter()
                .chain(rows.iter().flatten())
                .map(|c| count(c))
                .sum(),
            Block::SubHeading(text) | Block::Error(text) => count(text),
            Block::Metrics(items) => items.iter().map(|m| count(&m.label) + 1).sum(),
            Block::Picture(_) | Block::Diagram(_) => 0,
        };
    }
    total
}

/// The slide as the JSON the layout reads, with pictures and diagrams drawn for `step` as bytes.
fn card_json(
    slide: &Slide,
    first: bool,
    step: usize,
    assets: &mut Assets,
    options: &Options,
) -> Result<(Value, Media)> {
    let mut blocks = Vec::new();
    let mut media = Vec::new();
    let mut base = 0usize;
    // Diagrams are drawn about half a card high.
    let px_height = (options.height() as f32 * 0.5).round() as u32;
    for block in &slide.blocks {
        let value = match block {
            Block::Paragraph(lines) => json!({ "kind": "paragraph", "lines": lines }),
            Block::List(items) => {
                base += items.iter().filter(|i| i.reveal).count();
                json!({ "kind": "list", "items": items })
            },
            Block::Code { lang, lines } => json!({ "kind": "code", "lang": lang, "lines": lines }),
            Block::Table { head, rows, .. } => {
                // without the typst feature a table is text; the film draws it as lines
                let mut lines = vec![head.join("  ")];
                lines.extend(rows.iter().map(|r| r.join("  ")));
                json!({ "kind": "code", "lang": "", "lines": lines })
            },
            Block::Quote(lines) => json!({ "kind": "quote", "lines": lines }),
            Block::SubHeading(text) => json!({ "kind": "subheading", "text": text }),
            Block::Metrics(items) => json!({ "kind": "metrics", "items": items }),
            Block::Picture(picture) => {
                let key = format!("img{}", media.len());
                let loaded = assets.picture(picture, Some(px_height))?;
                media.push((key.clone(), loaded.bytes.as_ref().clone()));
                json!({ "kind": "image", "key": key, "height": picture.height, "steps": 0 })
            },
            Block::Diagram(diagram) => {
                let key = format!("img{}", media.len());
                let steps = Assets::steps(diagram);
                let lit = step.saturating_sub(base).min(steps);
                base += steps;
                let png = assets.diagram_png(diagram, lit, px_height)?;
                media.push((key.clone(), png.as_ref().clone()));
                json!({ "kind": "image", "key": key, "height": diagram.height, "steps": steps })
            },
            Block::Error(text) => json!({ "kind": "error", "text": text }),
        };
        blocks.push(value);
    }
    let card = json!({
        "title": slide.heading,
        "first": slide.is_title(first),
        "blocks": blocks,
    });
    Ok((card, media))
}

/// The deck's colours and font families as the JSON the layout reads.
fn theme(deck: &Deck) -> String {
    let pal = deck.palette();
    let mut map = serde_json::Map::new();
    for (name, colour) in pal.named() {
        map.insert(name.to_string(), Value::String(colour.to_string()));
    }
    map.insert("sans".into(), Value::String(deck.tokens.fonts.sans.clone()));
    map.insert("mono".into(), Value::String(deck.tokens.fonts.mono.clone()));
    map.insert(
        "display".into(),
        Value::String(deck.tokens.fonts.display.clone()),
    );
    Value::Object(map).to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::{Config, Settings};

    fn tall(source: &str) -> (Loaded, Options) {
        let loaded = Config::default().parse(source, "test", Path::new("."));
        let flags = Settings {
            format: Some(Format::Tall),
            width: Some(180),
            fps: Some(10),
            ..Settings::default()
        };
        let options = Options::resolve(flags, &loaded.source, &Config::default()).unwrap();
        (loaded, options)
    }

    /// The rows where `a` and `b` differ.
    fn rows(a: &Frame, b: &Frame) -> Vec<usize> {
        let row = a.width as usize * 3;
        (0..a.height as usize)
            .filter(|y| a.rgb[y * row..(y + 1) * row] != b.rgb[y * row..(y + 1) * row])
            .collect()
    }

    #[test]
    fn a_moving_word_leaves_the_other_lines_where_they_are() {
        let (loaded, options) = tall("# Heading\n\nalpha beta\ngamma delta\n");
        let engine = deckmark::faces::engine(&[]).unwrap();
        let mut maker = Maker::new(&loaded, &options).unwrap();
        let (pages, _) = maker.paged(&engine).unwrap();
        let mut card = maker.card(&pages[0], 0).unwrap();
        let mut draw = |state: &str| {
            card.states = vec![state.to_string()];
            card.draw(&engine, 0).unwrap()
        };
        let rest = draw(r#"{"w":[]}"#);
        let moved = draw(r#"{"w":[{"o":0.4,"x":0.2,"y":0.3,"s":1.2}]}"#);
        let bodiless = draw(r#"{"w":[{},{"o":0},{"o":0},{"o":0},{"o":0}]}"#);
        let body = rows(&rest, &bodiless);
        let changed = rows(&rest, &moved);
        assert!(!body.is_empty() && !changed.is_empty());
        assert!(
            changed.iter().all(|y| y < &body[0]),
            "{changed:?} vs {body:?}"
        );
    }

    #[test]
    fn typed_letters_sit_where_they_are_at_rest() {
        let (loaded, options) =
            tall("# Type\n\n:::{scene} type\n:::\n\nWrite the test **first**, then the code.\n");
        let engine = deckmark::faces::engine(&[]).unwrap();
        let mut maker = Maker::new(&loaded, &options).unwrap();
        let (pages, _) = maker.paged(&engine).unwrap();
        let mut card = maker.card(&pages[0], 0).unwrap();
        let still = card.draw(&engine, card.frames - 1).unwrap();
        let ground = &still.rgb[..3];
        let states = std::mem::take(&mut card.states);
        for state in states.iter().step_by(3) {
            // the letters only, no caret or marks: every pixel is the still's or the ground
            let mut state: Value = serde_json::from_str(state).unwrap();
            state.as_object_mut().unwrap().remove("c");
            for look in state["w"].as_array_mut().unwrap() {
                look.as_object_mut()
                    .unwrap()
                    .retain(|key, _| key == "chars");
            }
            card.states = vec![state.to_string()];
            let frame = card.draw(&engine, 0).unwrap();
            let moved = frame.rgb.chunks(3).zip(still.rgb.chunks(3));
            let off = moved.filter(|(a, b)| a != b && *a != ground).count();
            assert_eq!(off, 0, "{state}");
        }
    }

    #[test]
    fn a_tall_slide_that_does_not_fit_pages_on_with_its_heading() {
        let items = (1..=30).map(|i| format!("- point number {i}"));
        let (loaded, options) = tall(&format!(
            "# Many\n\n{}\n",
            items.collect::<Vec<_>>().join("\n")
        ));
        let engine = deckmark::faces::engine(&[]).unwrap();
        let mut maker = Maker::new(&loaded, &options).unwrap();
        let (mut pages, overflows) = maker.paged(&engine).unwrap();
        assert!(overflows.is_empty(), "{overflows:?}");
        maker.pin(&engine, &mut pages).unwrap();
        let mut heading = |page: &Page| maker.card(page, 0).unwrap().spots(&engine).unwrap()[0];
        let at: Vec<f64> = pages.iter().map(|p| heading(p).unwrap().y).collect();
        assert!(at.iter().all(|y| (y - at[0]).abs() < 0.01), "{at:?}");
        assert!(pages.len() > 1);
        assert!(
            pages
                .iter()
                .all(|p| p.slide.heading == loaded.deck.slides[0].heading)
        );
        assert_eq!(pages.iter().filter(|p| p.more).count(), pages.len() - 1);
        let sizes: Vec<usize> = pages.iter().map(|p| len(&p.slide.blocks[0])).collect();
        assert!(
            sizes.iter().max().unwrap() - sizes.iter().min().unwrap() <= 1,
            "{sizes:?}"
        );
        let lines: Vec<String> = (1..=60).map(|i| format!("let x{i} = {i};")).collect();
        let (loaded, options) = tall(&format!("# Code\n\n```rust\n{}\n```\n", lines.join("\n")));
        let overflows = crate::check(&loaded, &options).unwrap();
        assert_eq!(overflows.len(), 1, "nothing fits: still an overflow");
    }
}
