//! The canvas of a film: its aspect ([`Format`]), the area kept clear of a phone's buttons and
//! captions ([`Safe`]), and the defaults each format brings.

use std::str::FromStr;

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::Settings;

/// The aspect of the film.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Format {
    /// 16:9, for a README, a chat or a slide (the default).
    #[default]
    Wide,
    /// 9:16, for a phone.
    Tall,
    /// 1:1.
    Square,
}

impl Format {
    /// Width and height of the aspect.
    #[must_use]
    pub const fn aspect(self) -> (u32, u32) {
        match self {
            Self::Wide => (16, 9),
            Self::Tall => (9, 16),
            Self::Square => (1, 1),
        }
    }

    /// What the format sets below the config: wide is the built-in, tall and square are
    /// 1080 px at 30 fps with a 0.2 s fade and a 360 px GIF at 15 fps, tall in the phone's safe
    /// area.
    #[must_use]
    pub fn defaults(self) -> Settings {
        let phone = Settings {
            width: Some(1080),
            fps: Some(30),
            fade: Some(0.2),
            transition: Some(filmstrip::Transition::Fade),
            gif_width: Some(360),
            gif_fps: Some(15),
            ..Settings::default()
        };
        match self {
            Self::Wide => Settings::default(),
            Self::Tall => Settings {
                safe: Some(Safe::Phone),
                ..phone
            },
            Self::Square => phone,
        }
    }
}

impl FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self> {
        Ok(match name.trim().to_ascii_lowercase().as_str() {
            "wide" | "16:9" => Self::Wide,
            "tall" | "9:16" => Self::Tall,
            "square" | "1:1" => Self::Square,
            _ => bail!("format: expected wide, tall or square (16:9, 9:16, 1:1), got \"{name}\""),
        })
    }
}

impl TryFrom<String> for Format {
    type Error = anyhow::Error;

    fn try_from(name: String) -> Result<Self> {
        name.parse()
    }
}

/// The area the content stays in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Safe {
    /// The whole card.
    #[default]
    None,
    /// Clear of a phone app's top bar, captions and side buttons.
    Phone,
}

impl Safe {
    /// The insets as fractions of the card: top, right, bottom, left.
    #[must_use]
    pub const fn insets(self) -> [f32; 4] {
        match self {
            Self::None => [0.0; 4],
            Self::Phone => [0.1125, 128.0 / 1080.0, 0.2, 64.0 / 1080.0],
        }
    }
}

impl FromStr for Safe {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self> {
        Ok(match name.trim().to_ascii_lowercase().as_str() {
            "none" => Self::None,
            "phone" => Self::Phone,
            _ => bail!("safe: expected none or phone, got \"{name}\""),
        })
    }
}

impl TryFrom<String> for Safe {
    type Error = anyhow::Error;

    fn try_from(name: String) -> Result<Self> {
        name.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_reads_by_name_or_aspect() {
        assert_eq!("tall".parse::<Format>().unwrap(), Format::Tall);
        assert_eq!(" 9:16 ".parse::<Format>().unwrap(), Format::Tall);
        assert_eq!("1:1".parse::<Format>().unwrap(), Format::Square);
        assert_eq!("Wide".parse::<Format>().unwrap(), Format::Wide);
        assert!("4:3".parse::<Format>().is_err());
        assert_eq!("phone".parse::<Safe>().unwrap(), Safe::Phone);
        assert!("tv".parse::<Safe>().is_err());
    }

    #[test]
    fn tall_is_a_phone_film_and_wide_the_built_in() {
        let tall = Format::Tall.defaults();
        assert_eq!(
            (tall.width, tall.fps, tall.safe),
            (Some(1080), Some(30), Some(Safe::Phone))
        );
        assert_eq!(Format::Square.defaults().safe, None);
        assert_eq!(Format::Wide.defaults(), Settings::default());
    }
}
