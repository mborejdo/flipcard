//! `flipcard.toml`: the look of the cards and the film's defaults.
//!
//! Precedence, strongest first: a command-line flag, the deck's frontmatter, the closest
//! `flipcard.toml` (found by walking up from the deck's folder; farther ones are ignored), the
//! format's defaults, the built-in default. An unknown key or a malformed value is an error naming the file.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use deckmark::deck::{self, Deck, Loaded};
use serde::Deserialize;

/// The file name looked up in the deck's folder and every folder above it.
pub const FILE: &str = "flipcard.toml";

/// The `[film]` table, the frontmatter's film keys or the flags: every key optional, a key left
/// out falls through to the next level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// The aspect: wide, tall or square (wide).
    pub format: Option<crate::Format>,
    /// The area the content stays in: none or phone (none; phone for tall).
    pub safe: Option<crate::Safe>,
    /// Frames per second (10).
    pub fps: Option<u16>,
    /// Width in pixels, even (640).
    pub width: Option<u32>,
    /// How the words come in: flip, rise, slide, fade, pop or still (flip; rise for tall).
    pub preset: Option<crate::Preset>,
    /// Seconds every scene stays at least (7 for flip).
    pub hold: Option<f32>,
    /// Seconds of build-in (1.6).
    pub enter: Option<f32>,
    /// Seconds a transition between cards takes (0.4).
    pub fade: Option<f32>,
    /// How one card gives way to the next (fade).
    pub transition: Option<filmstrip::Transition>,
    /// Words per minute the hold of a wordy scene is sized for (200); 0 turns it off.
    pub pace: Option<f32>,
    /// Colours of the GIF palette (48).
    pub colours: Option<usize>,
    /// Width of the GIF in pixels, the frame scaled down by a whole factor (the width).
    #[serde(rename = "gif-width")]
    pub gif_width: Option<u32>,
    /// Frames per second of the GIF, every n-th frame of the film (the fps).
    #[serde(rename = "gif-fps")]
    pub gif_fps: Option<u16>,
}

impl Settings {
    /// These settings over `under`: a key set here wins.
    #[must_use]
    pub fn over(self, under: Self) -> Self {
        Self {
            format: self.format.or(under.format),
            safe: self.safe.or(under.safe),
            preset: self.preset.or(under.preset),
            fps: self.fps.or(under.fps),
            width: self.width.or(under.width),
            hold: self.hold.or(under.hold),
            enter: self.enter.or(under.enter),
            fade: self.fade.or(under.fade),
            transition: self.transition.or(under.transition),
            pace: self.pace.or(under.pace),
            colours: self.colours.or(under.colours),
            gif_width: self.gif_width.or(under.gif_width),
            gif_fps: self.gif_fps.or(under.gif_fps),
        }
    }

    /// Read the film keys of a deck's frontmatter (`format`, `safe`, `preset`, `fps`, `width`, `hold`,
    /// `enter`, `fade`, `transition`, `pace`, `colours`, `gif-width`, `gif-fps`); a value that is
    /// not a number, or not one of the key's names, is an error naming the key.
    pub fn from_frontmatter(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        fn number<T: FromStr>(key: &str, value: Option<String>) -> Result<Option<T>> {
            value
                .map(|v| {
                    v.trim().parse::<T>().ok().with_context(|| {
                        format!("frontmatter {key}: expected a number, got \"{v}\"")
                    })
                })
                .transpose()
        }
        fn named<T: FromStr<Err = anyhow::Error>>(
            key: &str,
            value: Option<String>,
        ) -> Result<Option<T>> {
            value
                .map(|v| v.parse().with_context(|| format!("frontmatter {key}")))
                .transpose()
        }
        Ok(Self {
            format: named("format", get("format"))?,
            safe: named("safe", get("safe"))?,
            preset: named("preset", get("preset"))?,
            fps: number("fps", get("fps"))?,
            width: number("width", get("width"))?,
            hold: number("hold", get("hold"))?,
            enter: number("enter", get("enter"))?,
            fade: number("fade", get("fade"))?,
            transition: named("transition", get("transition"))?,
            pace: number("pace", get("pace"))?,
            colours: number("colours", get("colours"))?,
            gif_width: number("gif-width", get("gif-width"))?,
            gif_fps: number("gif-fps", get("gif-fps"))?,
        })
    }
}

/// What the cards are drawn in: the design tokens, the font files Typst draws with (next to the
/// vendored Inter and Fira Code) and a logo for the corner.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    /// The `[theme]` table over the defaults.
    pub theme: swatchbook::Theme,
    /// `fonts = [...]`, resolved against the config file.
    pub faces: Vec<PathBuf>,
    /// `logo = "..."`, resolved against the config file.
    pub logo: Option<PathBuf>,
}

impl Look {
    /// Give `deck` this look.
    pub fn dress(&self, deck: &mut Deck) {
        deck.tokens = self.theme.clone();
        deck.faces.clone_from(&self.faces);
        deck.logo.clone_from(&self.logo);
    }
}

/// The config of a deck: its look and its `[film]` table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    /// The look of the cards.
    pub look: Look,
    /// The `[film]` table.
    pub film: Settings,
    /// The `flipcard.toml` it came from; `None` for the defaults.
    pub file: Option<PathBuf>,
}

/// The keys of a `flipcard.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default)]
    fonts: Vec<PathBuf>,
    logo: Option<PathBuf>,
    theme: Option<swatchbook::Theme>,
    #[serde(default)]
    film: Settings,
}

impl Config {
    /// Load the closest `flipcard.toml` from `deck_dir` up to the filesystem root; the defaults
    /// when there is none.
    ///
    /// # Examples
    ///
    /// ```
    /// let folder = tempfile::tempdir().expect("tempdir");
    /// std::fs::write(folder.path().join("flipcard.toml"), "[film]\nhold = 5\n").expect("write");
    /// let config = flipkit::Config::load(folder.path()).expect("load");
    /// assert_eq!(config.film.hold, Some(5.0));
    /// ```
    pub fn load(deck_dir: &Path) -> Result<Self> {
        let start = std::path::absolute(deck_dir)
            .with_context(|| format!("resolve the deck folder {}", deck_dir.display()))?;
        let Some(file) = start
            .ancestors()
            .map(|folder| folder.join(FILE))
            .find(|file| file.is_file())
        else {
            return Ok(Self::default());
        };
        Self::read(&file)
    }

    /// Read one config file; paths in it are relative to its folder.
    pub fn read(file: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(file).with_context(|| format!("read {}", file.display()))?;
        let raw: Raw =
            toml::from_str(&text).with_context(|| format!("parse {}", file.display()))?;
        let dir = file.parent().unwrap_or_else(|| Path::new("."));
        Ok(Self {
            look: Look {
                theme: raw.theme.unwrap_or_default(),
                faces: raw.fonts.iter().map(|f| dir.join(f)).collect(),
                logo: raw.logo.map(|l| dir.join(l)),
            },
            film: raw.film,
            file: Some(file.to_path_buf()),
        })
    }

    /// Read and parse the deck in `file` in this look; a deck without slides is an error.
    pub fn deck(&self, file: &Path) -> Result<Loaded> {
        deck::read(file, false, |d| self.look.dress(d))
    }

    /// Parse a deck from `source` in this look; `name` is its title when the frontmatter has
    /// none, `base` the folder its pictures and diagrams are relative to.
    #[must_use]
    pub fn parse(&self, source: &str, name: &str, base: &Path) -> Loaded {
        let mut deck = deck::parse(source, name, base, false);
        self.look.dress(&mut deck);
        Loaded {
            deck,
            base: base.to_path_buf(),
            source: source.to_string(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let from = self
            .file
            .as_deref()
            .map_or_else(|| "default".to_string(), |p| p.display().to_string());
        writeln!(out, "config: {from}")?;
        let fonts: Vec<String> = self
            .look
            .faces
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        writeln!(
            out,
            "  fonts = {}",
            if fonts.is_empty() {
                "built-in".to_string()
            } else {
                fonts.join(", ")
            }
        )?;
        writeln!(
            out,
            "  logo  = {}",
            self.look
                .logo
                .as_deref()
                .map_or_else(|| "none".to_string(), |p| p.display().to_string())
        )?;
        let f = &self.film;
        let film: Vec<String> = [
            ("format", f.format.map(|v| format!("{v:?}").to_lowercase())),
            ("safe", f.safe.map(|v| format!("{v:?}").to_lowercase())),
            ("preset", f.preset.map(|v| v.name().to_string())),
            ("fps", f.fps.map(|v| v.to_string())),
            ("width", f.width.map(|v| v.to_string())),
            ("hold", f.hold.map(|v| v.to_string())),
            ("enter", f.enter.map(|v| v.to_string())),
            ("fade", f.fade.map(|v| v.to_string())),
            (
                "transition",
                f.transition.map(|v| format!("{v:?}").to_lowercase()),
            ),
            ("pace", f.pace.map(|v| v.to_string())),
            ("colours", f.colours.map(|v| v.to_string())),
            ("gif-width", f.gif_width.map(|v| v.to_string())),
            ("gif-fps", f.gif_fps.map(|v| v.to_string())),
        ]
        .into_iter()
        .filter_map(|(key, value)| Some(format!("{key} = {}", value?)))
        .collect();
        write!(
            out,
            "  film  = {}",
            if film.is_empty() {
                "built-in".to_string()
            } else {
                film.join(", ")
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closest_file_wins_and_its_paths_are_next_to_it() {
        let root = tempfile::tempdir().expect("tempdir");
        let talk = root.path().join("talk");
        std::fs::create_dir_all(&talk).expect("mkdir");
        std::fs::write(root.path().join(FILE), "[film]\nfps = 12\n").expect("write");
        std::fs::write(
            talk.join(FILE),
            "fonts = [\"fonts\"]\nlogo = \"logo.png\"\n[theme.light]\naccent = \"#2b2459\"\n[film]\nhold = 5\ntransition = \"wipe\"\nformat = \"9:16\"\ngif-fps = 15\n",
        )
        .expect("write");
        let config = Config::load(&talk).expect("load");
        assert_eq!(config.film.hold, Some(5.0));
        assert_eq!(config.film.fps, None, "the farther file is ignored");
        assert_eq!(config.film.transition, Some(filmstrip::Transition::Wipe));
        assert_eq!(config.film.format, Some(crate::Format::Tall));
        assert_eq!(config.film.gif_fps, Some(15));
        assert_eq!(config.look.faces, [talk.join("fonts")]);
        assert_eq!(config.look.logo, Some(talk.join("logo.png")));
        assert_ne!(config.look.theme, swatchbook::Theme::default());
    }

    #[test]
    fn an_unknown_key_is_an_error_naming_the_file() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join(FILE), "font_size = 42\n").expect("write");
        let error = format!("{:#}", Config::load(root.path()).expect_err("unknown key"));
        assert!(
            error.contains(FILE) && error.contains("font_size"),
            "{error}"
        );
    }

    #[test]
    fn flags_beat_the_frontmatter_which_beats_the_file() {
        let flags = Settings {
            fps: Some(12),
            ..Settings::default()
        };
        let front = Settings::from_frontmatter(|key| {
            matches!(key, "fps" | "hold").then(|| "4".to_string())
        })
        .expect("frontmatter");
        let file = Settings {
            hold: Some(9.0),
            pace: Some(0.0),
            ..Settings::default()
        };
        let set = flags.over(front).over(file);
        assert_eq!(
            (set.fps, set.hold, set.pace),
            (Some(12), Some(4.0), Some(0.0))
        );
        assert!(Settings::from_frontmatter(|_| Some("x".into())).is_err());
        let tall = Settings::from_frontmatter(|key| (key == "format").then(|| "tall".into()));
        assert_eq!(tall.unwrap().format, Some(crate::Format::Tall));
    }
}
