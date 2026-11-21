//! flipkit: a deck as a film. Every slide is a card drawn by Typst in the deck's tokens, wide
//! (16:9), tall (9:16) or square; every key press of the talk (a `+` bullet, a lit `steps` node)
//! is a build-in step of its own, with `enter` seconds of animation (bullets appear, marked text
//! underlines itself, metrics count up, a title lands word by word) and `hold` seconds of rest.
//! The frames stream to `filmstrip`: a GIF and an MP4.
//!
//! On a tall card the words come in by a [`Preset`] instead: on a beat, block after block, timed
//! in Rust and drawn by Typst without reflow.
//!
//! The deck is the presiterm dialect, read by [`deckmark`]. This file is the spine: [`Options`],
//! [`render`] and the encoders; [`config`] reads `flipcard.toml`, [`format`](mod@format) the
//! canvas; `card` makes the scenes, `script` reads `:::{scene}` and numbers the words, `motion`
//! times them. The card layout is `typ/motion.typ` and `film.typ`, both embedded.

// Pixel arithmetic converts between usize, u32 and f32; a card is a few thousand pixels, far
// inside every range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "pixel math on a few thousand pixels, far inside every range"
)]

mod card;
pub mod config;
pub mod format;
mod motion;
mod script;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread::{Scope, ScopedJoinHandle};

use anyhow::{Context, Result, bail};
use filmstrip::Shot;
use rayon::prelude::*;
use typstkit::Engine;

use crate::card::Maker;

pub use config::{Config, Settings};
pub use deckmark;
pub use deckmark::deck::Loaded;
pub use filmstrip::{Frame, Transition};
pub use format::{Format, Safe};
pub use motion::Preset;

/// Distinct frames drawn side by side before they go to the encoders.
const CHUNK: usize = 32;

/// How the film is timed and sized.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The aspect.
    pub format: Format,
    /// The area the content stays in.
    pub safe: Safe,
    /// Frames per second.
    pub fps: u16,
    /// Width in pixels; the height follows the format. Even numbers only (the MP4 needs them).
    pub width: u32,
    /// How the words come in, unless a scene says otherwise.
    pub preset: Preset,
    /// Seconds every scene stays at least, build-in included; unset, a flip scene stays 7 and
    /// a motion scene its entrances and four beats.
    pub hold: Option<f32>,
    /// Seconds of flip build-in at the start of every step.
    pub enter: f32,
    /// Seconds a transition between cards takes.
    pub fade: f32,
    /// How one card gives way to the next.
    pub transition: filmstrip::Transition,
    /// Words per minute: a scene with more words than `hold` seconds cover at this pace stays
    /// longer, so a wordy card can be read; 0 holds every scene for `hold`.
    pub pace: f32,
    /// Colours of the GIF palette (2 to 255).
    pub colours: usize,
    /// Width of the GIF: the frame scaled down by the whole factor that keeps at least this.
    pub gif_width: u32,
    /// Frames per second of the GIF: every n-th frame of the film, as close to this as whole
    /// steps allow.
    pub gif_fps: u16,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            format: Format::Wide,
            safe: Safe::None,
            fps: 10,
            width: 640,
            preset: Preset::Flip,
            hold: None,
            enter: 1.6,
            fade: 0.4,
            transition: filmstrip::Transition::Fade,
            pace: 200.0,
            colours: 48,
            gif_width: 640,
            gif_fps: 10,
        }
    }
}

impl Options {
    /// Resolve the options: `flags` beat the deck's frontmatter (`source`), which beats the
    /// config's `[film]` table, which beats the format's defaults, which beat the built-in ones.
    pub fn resolve(flags: Settings, source: &str, config: &Config) -> Result<Self> {
        let front = colonmark::Frontmatter::read(source);
        let deck = Settings::from_frontmatter(|key| front.get(key).map(str::to_string))?;
        let set = flags.over(deck).over(config.film);
        let format = set.format.unwrap_or_default();
        let set = set.over(format.defaults());
        let base = Self::default();
        let (fps, width) = (set.fps.unwrap_or(base.fps), set.width.unwrap_or(base.width));
        Ok(Self {
            format,
            safe: set.safe.unwrap_or(base.safe),
            fps,
            width,
            preset: set.preset.unwrap_or(base.preset),
            hold: set.hold,
            enter: set.enter.unwrap_or(base.enter),
            fade: set.fade.unwrap_or(base.fade),
            transition: set.transition.unwrap_or(base.transition),
            pace: set.pace.unwrap_or(base.pace),
            colours: set.colours.unwrap_or(base.colours),
            gif_width: set.gif_width.unwrap_or(width),
            gif_fps: set.gif_fps.unwrap_or(fps),
        })
    }

    /// Height in pixels: the width in the format's aspect, rounded to an even number.
    #[must_use]
    pub const fn height(&self) -> u32 {
        let (w, h) = self.format.aspect();
        (self.width * h + w) / (2 * w) * 2
    }
}

/// Which films [`render`] encodes while the frames stream by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outputs {
    /// The animated GIF.
    pub gif: bool,
    /// The MP4.
    pub mp4: bool,
}

/// A rendered deck: the stills of its scenes and the films asked for.
#[derive(Clone, Debug)]
pub struct Rendered {
    /// The fully built last frame of every scene (a slide at a step).
    pub stills: Vec<Frame>,
    /// Which slide each scene shows.
    pub slide_of: Vec<usize>,
    /// Seconds each scene stays.
    pub holds: Vec<f32>,
    /// Frames in the film.
    pub frames: usize,
    /// The animated GIF, if asked for.
    pub gif: Option<Vec<u8>>,
    /// The MP4, if asked for.
    pub mp4: Option<Vec<u8>>,
}

impl Rendered {
    /// Every scene's fully built card as PNG, with the 1-based slide it shows.
    pub fn cards(&self) -> Result<Vec<(usize, Vec<u8>)>> {
        self.stills
            .iter()
            .zip(&self.slide_of)
            .map(|(still, slide)| Ok((slide + 1, still.png()?)))
            .collect()
    }
}

/// A card that does not fit: slide index, its title, pages it would take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overflow {
    /// 0-based slide index.
    pub slide: usize,
    /// The slide's heading, or empty.
    pub title: String,
    /// Pages the card spilled onto (at least 2).
    pub pages: usize,
}

impl std::fmt::Display for Overflow {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "slide {} ({}): the card overflows onto {} pages; trim it",
            self.slide + 1,
            self.title,
            self.pages
        )
    }
}

/// Render the film and encode the `outputs` asked for. A card that overflows is an error naming
/// the slide: nothing is cropped.
///
/// The stills (the last frame of every scene) and the first frames are drawn first: they are the
/// edges of the transitions, the cards and the GIF's palette. The rest streams to the encoders
/// in chunks of distinct frames, drawn side by side on one engine; a frame that stays, or whose
/// words didn't move, is drawn once.
pub fn render(loaded: &Loaded, options: &Options, outputs: Outputs) -> Result<Rendered> {
    if !options.width.is_multiple_of(2) || options.width < 64 {
        bail!("film width must be an even number of at least 64 pixels");
    }
    if options.fps == 0 || options.gif_fps == 0 || options.gif_width == 0 {
        bail!("film fps, gif-fps and gif-width must be at least 1");
    }
    let engine = engine(&loaded.deck.faces)?;
    let cards = Maker::new(loaded, options)?.cards(&engine)?;
    let stills = run(&engine, &cards, |engine, card| {
        Ok(Arc::new(card.draw(engine, card.frames - 1)?))
    })?;
    let edges: Vec<_> = cards.iter().zip(&stills).collect();
    let firsts = run(&engine, &edges, |engine, (card, still)| {
        Ok(if card.key(0) + 1 == card.frames {
            Arc::clone(still)
        } else {
            Arc::new(card.draw(engine, 0)?)
        })
    })?;

    let timing = filmstrip::Timing {
        fps: options.fps,
        fade: options.fade,
        transition: options.transition,
    };
    let plan: Vec<_> = cards.iter().map(|c| (c.frames, c.ticks, c.hold)).collect();
    let shots: Vec<Shot> = filmstrip::shots(&plan, timing)?
        .into_iter()
        .map(|shot| match shot {
            Shot::Card { scene, frame } => Shot::Card {
                scene,
                frame: cards[scene].key(frame),
            },
            between @ Shot::Between { .. } => between,
        })
        .collect();
    let runs: Vec<&[Shot]> = shots.chunk_by(PartialEq::eq).collect();
    let [gif, mp4] = std::thread::scope(|scope| {
        let film = Film::new(scope, options, outputs, &stills)?;
        for chunk in runs.chunks(CHUNK) {
            let frames = run(&engine, chunk, |engine, run| {
                Ok(match run[0] {
                    Shot::Card { scene, frame } if frame + 1 == cards[scene].frames => {
                        Arc::clone(&stills[scene])
                    },
                    Shot::Card { scene, frame: 0 } => Arc::clone(&firsts[scene]),
                    Shot::Card { scene, frame } => Arc::new(cards[scene].draw(engine, frame)?),
                    Shot::Between { scene, t } => {
                        let next = &firsts[(scene + 1) % firsts.len()];
                        Arc::new(options.transition.frame(&stills[scene], next, t))
                    },
                })
            })?;
            // Typst's cache keeps what the last chunk drew: a long film stays small
            Engine::evict(1);
            for (frame, run) in frames.iter().zip(chunk) {
                film.push(frame, run.len());
            }
        }
        film.finish()
    })?;
    drop(firsts);
    Ok(Rendered {
        stills: stills.into_iter().map(Arc::unwrap_or_clone).collect(),
        slide_of: cards.iter().map(|c| c.slide).collect(),
        holds: cards.iter().map(|c| c.hold).collect(),
        frames: shots.len(),
        gif,
        mp4,
    })
}

/// An encoder on a thread of its own: runs of frames in, the file out.
type Encoder<'s> = (
    SyncSender<(Arc<Frame>, usize)>,
    ScopedJoinHandle<'s, Result<Vec<u8>>>,
);

/// The encoders the film streams into, side by side with the drawing.
struct Film<'s> {
    gif: Option<Encoder<'s>>,
    mp4: Option<Encoder<'s>>,
}

impl<'s> Film<'s> {
    fn new(
        scope: &'s Scope<'s, '_>,
        options: &Options,
        outputs: Outputs,
        stills: &[Arc<Frame>],
    ) -> Result<Self> {
        let Some(first) = stills.first() else {
            bail!("the deck has no slides");
        };
        // the GIF takes every `every`-th frame, `scale` times smaller
        let scale = (options.width / options.gif_width).max(1);
        let every = (options.fps / options.gif_fps).max(1);
        let gif = outputs
            .gif
            .then(|| {
                let keys: Vec<Frame> = stills.iter().map(|f| f.downscale(scale)).collect();
                let (w, h) = (first.width / scale, first.height / scale);
                let writer =
                    filmstrip::gif::Writer::new(w, h, &keys, options.fps / every, options.colours)?;
                let every = usize::from(every);
                Ok::<_, anyhow::Error>(encoder(
                    scope,
                    (writer, 0),
                    move |(gif, at): &mut (filmstrip::gif::Writer, usize), frame, count| {
                        let takes = (*at + count).div_ceil(every) - at.div_ceil(every);
                        *at += count;
                        if takes > 0 {
                            gif.push(&frame.downscale(scale), takes)?;
                        }
                        Ok(())
                    },
                    |(gif, _)| gif.finish(),
                ))
            })
            .transpose()?;
        let mp4 = outputs
            .mp4
            .then(|| {
                let writer = filmstrip::video::Writer::new(first.width, first.height, options.fps)?;
                Ok::<_, anyhow::Error>(encoder(
                    scope,
                    writer,
                    filmstrip::video::Writer::push,
                    filmstrip::video::Writer::finish,
                ))
            })
            .transpose()?;
        Ok(Self { gif, mp4 })
    }

    /// Send `frame` as the next `count` frames; an encoder that stopped reports why in `finish`.
    fn push(&self, frame: &Arc<Frame>, count: usize) {
        for (sender, _) in self.gif.iter().chain(&self.mp4) {
            let _ = sender.send((Arc::clone(frame), count));
        }
    }

    /// The GIF and the MP4.
    fn finish(self) -> Result<[Option<Vec<u8>>; 2]> {
        let join = |encoder: Option<Encoder<'s>>| {
            encoder
                .map(|(sender, thread)| {
                    drop(sender);
                    thread
                        .join()
                        .map_err(|_| anyhow::anyhow!("an encoder failed"))?
                })
                .transpose()
        };
        Ok([join(self.gif)?, join(self.mp4)?])
    }
}

/// An encoder thread: `push` every run of frames into `state`, then `finish` it.
fn encoder<'s, S: Send + 's>(
    scope: &'s Scope<'s, '_>,
    mut state: S,
    push: impl Fn(&mut S, &Frame, usize) -> Result<()> + Send + 's,
    finish: impl FnOnce(S) -> Result<Vec<u8>> + Send + 's,
) -> Encoder<'s> {
    let (sender, frames) = sync_channel::<(Arc<Frame>, usize)>(CHUNK);
    let thread = scope.spawn(move || {
        for (frame, count) in frames {
            push(&mut state, &frame, count)?;
        }
        finish(state)
    });
    (sender, thread)
}

/// `job` over `items` side by side on one engine, the results in order.
fn run<T: Sync, R: Send>(
    engine: &Engine,
    items: &[T],
    job: impl Fn(&Engine, &T) -> Result<R> + Sync,
) -> Result<Vec<R>> {
    items.par_iter().map(|item| job(engine, item)).collect()
}

/// A new engine with Inter Display Black (headings and figures) and the deck's fonts, loaded
/// as `deckmark::faces::engine` loads them: a file as it is, a folder by its font files in name
/// order.
fn engine(faces: &[PathBuf]) -> Result<Engine> {
    let is_font =
        |path: &Path| deckmark::deck::extension_is(&path.to_string_lossy(), &["otf", "ttf", "ttc"]);
    let mut builder =
        Engine::builder().font(include_bytes!("../fonts/InterDisplay-Black.otf").to_vec());
    for face in faces {
        let mut files = vec![face.clone()];
        if face.is_dir() {
            files = std::fs::read_dir(face)
                .with_context(|| format!("fonts: {}", face.display()))?
                .filter_map(|entry| Some(entry.ok()?.path()))
                .filter(|path| is_font(path))
                .collect();
            files.sort();
        }
        for file in files {
            let bytes =
                std::fs::read(&file).with_context(|| format!("fonts: {}", file.display()))?;
            builder = builder.font(bytes);
        }
    }
    Ok(builder.build())
}

/// Words a tall screen holds before `flipcard lint` warns; more read badly on a phone.
pub const TALL_WORDS: usize = 12;

/// The tall slides with more than [`TALL_WORDS`] words on screen, fully built: 0-based index,
/// title, words. Empty for wide and square films.
#[must_use]
pub fn crowded(loaded: &Loaded, options: &Options) -> Vec<(usize, String, usize)> {
    if options.format != Format::Tall {
        return Vec::new();
    }
    let slides = loaded.deck.slides.iter().enumerate();
    slides
        .map(|(i, slide)| {
            let words = (0..=deckmark::fragments(slide)).map(|step| card::words(slide, step));
            (i, slide.title().unwrap_or_default(), words.sum())
        })
        .filter(|&(_, _, words)| words > TALL_WORDS)
        .collect()
}

/// Lay out every slide fully built (a tall one paged on) and report the ones that overflow;
/// what `flipcard lint` runs.
pub fn check(loaded: &Loaded, options: &Options) -> Result<Vec<Overflow>> {
    let engine = engine(&loaded.deck.faces)?;
    Ok(Maker::new(loaded, options)?.paged(&engine)?.1)
}

/// Where the film's files go: `<out or the deck's folder>/<deck stem>.<ext>`.
#[must_use]
pub fn output_path(deck_file: &Path, out: Option<&Path>, ext: &str) -> std::path::PathBuf {
    let stem = deck_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("film");
    let dir = out.map_or_else(
        || deckmark::deck_dir(deck_file),
        std::path::Path::to_path_buf,
    );
    dir.join(format!("{stem}.{ext}"))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::path::Path;

    use deckmark::deck;
    use serde_json::Value;

    use super::*;

    fn load(source: &str) -> Loaded {
        Loaded {
            deck: deck::parse(source, "test", Path::new("."), false),
            base: Path::new(".").to_path_buf(),
            source: source.to_string(),
        }
    }

    fn quick() -> Options {
        Options {
            fps: 4,
            width: 128,
            hold: Some(0.5),
            enter: 0.5,
            fade: 0.25,
            pace: 0.0,
            colours: 16,
            gif_width: 128,
            gif_fps: 4,
            ..Options::default()
        }
    }

    const BOTH: Outputs = Outputs {
        gif: true,
        mp4: true,
    };

    #[test]
    fn a_wordy_scene_is_held_for_its_reading_time_and_a_short_one_for_the_hold() {
        let loaded = load(
            "# Short

---
# Long

one two three four five six seven eight nine ten

- eleven twelve
+ thirteen fourteen fifteen
",
        );
        let options = Options {
            hold: Some(2.0),
            pace: 60.0, // one word a second
            ..quick()
        };
        let rendered = render(&loaded, &options, Outputs::default()).unwrap();
        // "Short" alone: one word, so the hold; "Long" at step 0: 13 words; step 1: 3 words
        assert_eq!(rendered.holds, vec![2.0, 13.0, 3.0]);
        assert_eq!(card::words(&loaded.deck.slides[1], 0), 13);
        assert_eq!(card::words(&loaded.deck.slides[1], 1), 3);
        assert_eq!(card::words(&loaded.deck.slides[1], 2), 0);
    }

    #[test]
    fn every_key_press_is_a_scene_and_the_film_loops_through_them() {
        let loaded = load(
            "---\ntitle: T\n---\n# One\n\n**bold** and ==marked==\n\n---\n# Two\n\n- a\n+ b\n+ c\n\n:::{metrics}\n1 200 | things\n60 % | share\n:::\n",
        );
        let rendered = render(&loaded, &quick(), BOTH).unwrap();
        // slide 1: one scene; slide 2: steps 0, 1, 2
        assert_eq!(rendered.stills.len(), 4);
        assert_eq!(rendered.slide_of, vec![0, 1, 1, 1]);
        let first = &rendered.stills[0];
        assert_eq!((first.width, first.height), (128, 72));
        // per scene: 1 build-in frame for 2 ticks, the still for at least 2, 1 transition frame
        assert_eq!(rendered.frames, 4 * (2 + 2 + 1));
        assert!(rendered.gif.unwrap().starts_with(b"GIF89a"));
        assert_eq!(&rendered.mp4.unwrap()[4..8], b"ftyp");
    }

    #[test]
    fn a_card_that_overflows_is_an_error_that_names_the_slide() {
        let mut source = String::from("# Too much\n\n");
        for i in 0..40 {
            let _ = writeln!(source, "- bullet number {i} with some words on it");
        }
        let loaded = load(&source);
        let error = render(&loaded, &quick(), BOTH).unwrap_err().to_string();
        assert!(error.contains("slide 1 (Too much)"), "{error}");
        let found = check(&loaded, &quick()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].slide, 0);
    }

    #[test]
    fn a_tall_film_is_portrait_and_its_gif_a_smaller_slower_preview() {
        let loaded = load("---\nformat: tall\n---\n# Tall\n\n- one\n+ two\n");
        let flags = Settings {
            width: Some(128),
            fps: Some(4),
            gif_width: Some(64),
            gif_fps: Some(2),
            ..Settings::default()
        };
        let options = Options::resolve(flags, &loaded.source, &Config::default()).unwrap();
        assert_eq!((options.format, options.safe), (Format::Tall, Safe::Phone));
        assert_eq!((options.height(), options.fade), (228, 0.2));
        let rendered = render(&loaded, &options, BOTH).unwrap();
        assert_eq!(
            (rendered.stills[0].width, rendered.stills[0].height),
            (128, 228)
        );
        let gif = rendered.gif.unwrap();
        let size = |at: usize| u16::from_le_bytes([gif[at], gif[at + 1]]);
        assert_eq!((size(6), size(8)), (64, 114));
        assert_eq!(check(&loaded, &options).unwrap(), []);
        assert_eq!(crowded(&loaded, &options), []);
        let wordy = load(
            "---\nformat: tall\n---\n# Too // many\n\none two three four five six seven eight nine ten\n",
        );
        assert_eq!(crowded(&wordy, &options), [], "12 words, the pause not one");
        let more = load(
            "# A\n\none two three four five six seven eight nine ten eleven twelve thirteen\n",
        );
        assert_eq!(crowded(&more, &options)[0].2, 14);
        assert_eq!(crowded(&more, &quick()), []);
    }

    fn tall(source: &str, fps: u16) -> Rendered {
        let loaded = load(source);
        let flags = Settings {
            format: Some(Format::Tall),
            width: Some(128),
            fps: Some(fps),
            ..Settings::default()
        };
        let options = Options::resolve(flags, &loaded.source, &Config::default()).unwrap();
        render(&loaded, &options, Outputs::default()).unwrap()
    }

    #[test]
    fn a_scene_ends_as_its_still_and_a_pause_adds_a_beat_cut_on_whole_frames() {
        let body = "# Ship ==small==\n\none **two** three, ==four== __five__ ~~six~~\n\n:::{metrics}\n1 200 | things\n:::\n";
        let rise = tall(body, 12);
        let still = tall(&format!("{body}\n:::{{scene}} still\n:::\n"), 12);
        assert_eq!(rise.stills, still.stills);
        for preset in ["type", "karaoke"] {
            let paced = tall(&format!("{body}\n:::{{scene}} {preset}\n:::\n"), 12);
            assert_eq!(paced.stills, still.stills, "{preset}");
        }
        let paused = tall(&body.replace("**two** three", "**two** // three"), 12);
        assert!(
            (paused.holds[0] - rise.holds[0] - 0.4).abs() < 0.05,
            "{:?}",
            (paused.holds, rise.holds)
        );
        for hold in [rise.holds[0], paused.holds[0], still.holds[0]] {
            let frames = hold * 12.0;
            assert!((frames - frames.round()).abs() < 1e-3, "{hold}");
        }
    }

    #[test]
    fn the_phone_example_moves_as_its_golden_states() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        let config = Config::default();
        let loaded = config
            .deck(&here.join("../../examples/phone/script.md"))
            .unwrap();
        let options = Options::resolve(Settings::default(), &loaded.source, &config).unwrap();
        let engine = deckmark::faces::engine(&[]).unwrap();
        let cards = Maker::new(&loaded, &options)
            .unwrap()
            .cards(&engine)
            .unwrap();
        let mut actual = String::new();
        for (scene, card) in cards.iter().enumerate() {
            let last: Value = serde_json::from_str(&card.states[card.frames - 1]).unwrap();
            let at_rest = last["w"]
                .as_array()
                .unwrap()
                .iter()
                .all(|w| w == &Value::Object(serde_json::Map::new()));
            assert!(at_rest, "scene {scene} ends at rest");
            for (frame, state) in card.states.iter().enumerate() {
                if frame == 0 || card.states[frame - 1] != *state {
                    let _ = writeln!(
                        actual,
                        r#"{{"scene":{scene},"frame":{frame},"state":{state}}}"#
                    );
                }
            }
        }
        let golden = here.join("tests/golden/phone.jsonl");
        if std::env::var_os("FLIPCARD_BLESS").is_some() {
            std::fs::write(&golden, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(&golden).unwrap();
        assert!(
            actual == expected,
            "the states changed; FLIPCARD_BLESS=1 rewrites them"
        );
    }

    #[test]
    fn the_same_deck_gives_the_same_bytes() {
        let loaded = load("# A\n\n- one\n+ two\n\n---\n# B\n\n**bold**\n");
        let a = render(&loaded, &quick(), BOTH).unwrap();
        let b = render(&loaded, &quick(), BOTH).unwrap();
        assert_eq!((a.gif, a.mp4), (b.gif, b.mp4));
    }

    #[test]
    fn output_paths_sit_next_to_the_deck_or_in_out() {
        assert_eq!(
            output_path(Path::new("talk/slides.md"), None, "gif"),
            Path::new("talk/slides.gif")
        );
        assert_eq!(
            output_path(Path::new("talk/slides.md"), Some(Path::new("dist")), "mp4"),
            Path::new("dist/slides.mp4")
        );
    }
}
