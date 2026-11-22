//! How words enter: a preset is a look along an entrance, and the half-beats between two words.

use std::str::FromStr;

use serde::Deserialize;

use super::{Curve, Look};
use crate::format::ParseNameError;

/// How the words of a scene come in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum Preset {
    /// The flip-card build-in: bullets appear, marked text underlines itself (the wide default).
    #[default]
    Flip,
    /// Word by word, up from below as they fade in (the tall default).
    Rise,
    /// Word by word, in from the left.
    Slide,
    /// Block by block, fading in.
    Fade,
    /// Word by word, scaled up past their size and back.
    Pop,
    /// Everything there at once.
    Still,
    /// Letter by letter at the pace, an accent caret blinking on the beat.
    Type,
    /// Every word there in muted, filled with ink as it is read at the pace.
    Karaoke,
    /// Magic move: a word the card before has too glides from where it was; the others rise.
    Move,
    /// Line by line from below at the pace, the lines before moving up to make room.
    Stack,
}

impl Preset {
    /// Every preset, in the order of the docs.
    pub const ALL: [Self; 10] = [
        Self::Flip,
        Self::Rise,
        Self::Slide,
        Self::Fade,
        Self::Pop,
        Self::Still,
        Self::Type,
        Self::Karaoke,
        Self::Move,
        Self::Stack,
    ];

    /// The name the frontmatter, the config, the flag and `:::{scene}` use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Flip => "flip",
            Self::Rise => "rise",
            Self::Slide => "slide",
            Self::Fade => "fade",
            Self::Pop => "pop",
            Self::Still => "still",
            Self::Type => "type",
            Self::Karaoke => "karaoke",
            Self::Move => "move",
            Self::Stack => "stack",
        }
    }

    /// A word of `letters` letters `p` (0 to 1) through its entrance.
    pub(crate) fn look(self, p: f64, letters: usize) -> Look {
        let (ease, rest) = (Curve::Ease.at(p), Look::default());
        match self {
            Self::Rise | Self::Move => Look {
                o: ease,
                y: 0.5 * (1.0 - ease),
                ..rest
            },
            Self::Slide => Look {
                o: ease,
                x: -0.6 * (1.0 - Curve::Spring.at(p)),
                ..rest
            },
            // a stacked line fades in; the timeline moves it and the lines before up
            Self::Fade | Self::Stack => Look { o: ease, ..rest },
            Self::Pop => Look {
                o: Curve::Linear.at(2.0 * p),
                s: Curve::Back.at(p),
                ..rest
            },
            Self::Type => {
                // whole letters; the time of its space comes after the last
                let n = letters.max(1) as f64;
                Look {
                    chars: (p * (n + 1.0)).floor().min(n) / n,
                    ..rest
                }
            },
            Self::Karaoke => Look {
                fill: p,
                s: 0.04f64.mul_add(libm::sin(std::f64::consts::PI * p), 1.0),
                ..rest
            },
            Self::Flip | Self::Still => rest,
        }
    }

    /// Half-beats from one word's entrance to the next one's in a block (not paced).
    pub(crate) const fn stagger(self) -> u32 {
        match self {
            Self::Fade | Self::Flip | Self::Still | Self::Type | Self::Karaoke | Self::Stack => 0,
            Self::Rise | Self::Slide | Self::Pop | Self::Move => 1,
        }
    }

    /// Each word takes its letters at the pace.
    pub(crate) const fn paced(self) -> bool {
        matches!(self, Self::Type | Self::Karaoke)
    }

    /// The words are read as they come in, so the scene holds only four beats after.
    pub(crate) const fn read_in(self) -> bool {
        matches!(self, Self::Type | Self::Karaoke | Self::Stack)
    }

    /// It times the words by where they are on the card.
    pub(crate) const fn kinetic(self) -> bool {
        matches!(self, Self::Move | Self::Stack)
    }
}

impl FromStr for Preset {
    type Err = ParseNameError;

    fn from_str(name: &str) -> Result<Self, ParseNameError> {
        Self::ALL
            .into_iter()
            .find(|p| p.name().eq_ignore_ascii_case(name.trim()))
            .ok_or_else(|| {
                let names: Vec<_> = Self::ALL.iter().map(|p| p.name()).collect();
                let (last, rest) = names.split_last().unwrap_or((&"", &[]));
                let expected = format!("one of {} or {last}", rest.join(", "));
                ParseNameError::new("preset", expected, name)
            })
    }
}

impl TryFrom<String> for Preset {
    type Error = ParseNameError;

    fn try_from(name: String) -> Result<Self, ParseNameError> {
        name.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entrance_ends_at_rest_and_a_name_reads_back() {
        for preset in Preset::ALL {
            assert_eq!(preset.look(1.0, 5), Look::default(), "{preset:?}");
            assert_eq!(preset.name().parse::<Preset>().unwrap(), preset);
        }
        assert_eq!(Preset::Rise.look(0.0, 5).o, 0.0);
        assert!(Preset::Pop.look(0.7, 5).s > 1.0);
        // 4 letters and a space: a letter every fifth
        assert_eq!(Preset::Type.look(0.39, 4).chars, 0.25);
        assert_eq!(Preset::Type.look(0.9, 4).chars, 1.0);
        assert_eq!(Preset::Karaoke.look(0.5, 4).s, 1.04);
        assert_eq!(
            "bounce".parse::<Preset>().unwrap_err().to_string(),
            "preset: expected one of flip, rise, slide, fade, pop, still, type, karaoke, move or \
             stack, got \"bounce\""
        );
    }
}
