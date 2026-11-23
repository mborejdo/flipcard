//! How words enter: a preset is a look along an entrance, for every letter of a word, and the
//! half-beats between two words. Most move a word as one (its letters alike); `type` reveals
//! the letters, `wave`, `drop` and `decode` ripple through them, each letter a little after the
//! one before.

use std::f64::consts::PI;
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
    /// Word by word, the letters hopping up and back in a ripple as they fade in.
    Wave,
    /// Word by word, the letters falling into place one after the other, landing on a spring.
    Drop,
    /// Word by word, every letter a scramble of its kind, settling from the left.
    Decode,
}

/// Half-beats a word takes to come in (a rippled one: each letter).
pub(super) const ENTER: f64 = 2.0;

impl Preset {
    /// Every preset, in the order of the docs.
    pub const ALL: [Self; 13] = [
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
        Self::Wave,
        Self::Drop,
        Self::Decode,
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
            Self::Wave => "wave",
            Self::Drop => "drop",
            Self::Decode => "decode",
        }
    }

    /// The look of every letter of a word (`letters`, its graphemes) `p` (0 to 1) through its
    /// entrance; `seed` makes a scramble the word's own. All at rest at 1.
    pub(crate) fn letters(self, p: f64, letters: &[&str], seed: u64) -> Vec<Look> {
        let n = letters.len().max(1);
        let rest = Look::default();
        if p >= 1.0 {
            return vec![rest; n];
        }
        // half-beats into the word, and letter `i`'s own progress through its entrance
        let (len, gap) = self.ripple().unwrap_or((ENTER, 0.0));
        let t = p * self.enter(n);
        let q = |i: usize| (gap.mul_add(-(i as f64), t) / len).clamp(0.0, 1.0);
        match self {
            Self::Type => {
                // whole letters; the time of its space comes after the last
                let typed = Self::typed(p, n);
                (0..n)
                    .map(|i| {
                        if i < typed {
                            rest
                        } else {
                            Look { o: 0.0, ..rest }
                        }
                    })
                    .collect()
            },
            Self::Wave => (0..n)
                .map(|i| Look {
                    o: Curve::Ease.at(2.0 * q(i)),
                    y: -0.35 * libm::sin(PI * q(i)),
                    ..rest
                })
                .collect(),
            Self::Drop => (0..n)
                .map(|i| Look {
                    o: Curve::Linear.at(3.0 * q(i)),
                    y: -0.9 * (1.0 - Curve::Spring.at(q(i))),
                    ..rest
                })
                .collect(),
            Self::Decode => (0..n)
                .map(|i| Look {
                    // all there after half a beat, each a stand-in till its turn
                    o: Curve::Linear.at(t),
                    g: (q(i) < 1.0)
                        .then(|| {
                            let tick = (t * 2.0).floor() as u64;
                            scramble(letters.get(i)?, mix(seed ^ mix((i as u64) << 32 | tick)))
                        })
                        .flatten(),
                    ..rest
                })
                .collect(),
            _ => vec![self.look(p); n],
        }
    }

    /// A word `p` (0 to 1) through its entrance, its letters alike.
    pub(crate) fn look(self, p: f64) -> Look {
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
            Self::Karaoke => Look {
                fill: p,
                s: 0.04f64.mul_add(libm::sin(PI * p), 1.0),
                ..rest
            },
            Self::Flip | Self::Still | Self::Type | Self::Wave | Self::Drop | Self::Decode => rest,
        }
    }

    /// `type`: the letters of `n` typed `p` (0 to 1) through the word, its space after the last.
    pub(crate) fn typed(p: f64, n: usize) -> usize {
        ((p * (n as f64 + 1.0)).floor().max(0.0) as usize).min(n)
    }

    /// A rippled preset's letter: the half-beats each takes and from one's start to the next's.
    const fn ripple(self) -> Option<(f64, f64)> {
        match self {
            Self::Wave | Self::Drop => Some((3.0, 0.5)),
            Self::Decode => Some((2.0, 0.6)),
            _ => None,
        }
    }

    /// Half-beats a word of `n` letters takes to come in (not paced).
    pub(crate) fn enter(self, n: usize) -> f64 {
        self.ripple()
            .map_or(ENTER, |(len, gap)| gap.mul_add(n.max(1) as f64 - 1.0, len))
    }

    /// Half-beats from one word's entrance to the next one's in a block (not paced).
    pub(crate) const fn stagger(self) -> u32 {
        match self {
            Self::Fade | Self::Flip | Self::Still | Self::Type | Self::Karaoke | Self::Stack => 0,
            Self::Rise
            | Self::Slide
            | Self::Pop
            | Self::Move
            | Self::Wave
            | Self::Drop
            | Self::Decode => 1,
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

/// splitmix64: a well-mixed number from any number, the same on every platform.
const fn mix(z: u64) -> u64 {
    let z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A stand-in for `letter` picked by `hash`: a capital for a capital, a small letter for a small
/// one, a digit for a digit; `None` for anything else (punctuation, a space) or a letter of
/// another script.
fn scramble(letter: &str, hash: u64) -> Option<char> {
    let c = letter.chars().next()?;
    let pool: &[u8] = if c.is_uppercase() {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    } else if c.is_lowercase() {
        b"abcdefghijklmnopqrstuvwxyz"
    } else if c.is_ascii_digit() {
        b"0123456789"
    } else {
        return None;
    };
    Some(char::from(pool[(hash % pool.len() as u64) as usize]))
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

    const WORD: [&str; 5] = ["w", "o", "r", "d", "s"];

    #[test]
    fn every_entrance_ends_at_rest_and_a_name_reads_back() {
        for preset in Preset::ALL {
            assert_eq!(
                preset.letters(1.0, &WORD, 7),
                vec![Look::default(); 5],
                "{preset:?}"
            );
            assert_eq!(preset.look(1.0), Look::default(), "{preset:?}");
            assert_eq!(preset.name().parse::<Preset>().unwrap(), preset);
        }
        assert_eq!(Preset::Rise.look(0.0).o, 0.0);
        assert!(Preset::Pop.look(0.7).s > 1.0);
        assert_eq!(Preset::Karaoke.look(0.5).s, 1.04);
        assert_eq!(
            "bounce".parse::<Preset>().unwrap_err().to_string(),
            "preset: expected one of flip, rise, slide, fade, pop, still, type, karaoke, move, \
             stack, wave, drop or decode, got \"bounce\""
        );
    }

    #[test]
    fn a_word_preset_moves_its_letters_alike() {
        for preset in [Preset::Rise, Preset::Slide, Preset::Pop, Preset::Fade] {
            let letters = preset.letters(0.3, &WORD, 7);
            assert!(letters.iter().all(|l| *l == preset.look(0.3)), "{preset:?}");
        }
    }

    #[test]
    fn type_reveals_whole_letters_its_space_last() {
        // 4 letters and a space: a letter every fifth
        let shown = |p: f64| {
            let letters = Preset::Type.letters(p, &WORD[..4], 0);
            letters.iter().filter(|l| **l == Look::default()).count()
        };
        assert_eq!(
            (shown(0.0), shown(0.39), shown(0.79), shown(0.9)),
            (0, 1, 3, 4)
        );
        assert!(
            Preset::Type.letters(0.5, &WORD[..4], 0)[..2]
                .iter()
                .all(|l| *l == Look::default())
        );
    }

    #[test]
    fn a_ripple_brings_each_letter_after_the_one_before() {
        for preset in [Preset::Wave, Preset::Drop, Preset::Decode] {
            // the word takes its first letter's entrance and a gap per letter after it
            let (len, gap) = preset.ripple().unwrap();
            assert_eq!(preset.enter(5), 4.0f64.mul_add(gap, len), "{preset:?}");
            let early = preset.letters(0.2, &WORD, 7);
            assert_ne!(early[0], early[4], "{preset:?}");
        }
        // a wave hops up; a drop falls from above and lands on its spring, past its place
        let hop = Preset::Wave.letters(0.3, &WORD, 0);
        assert!(hop[0].y < 0.0 && hop[0].y < hop[4].y);
        let fall = |p: f64| Preset::Drop.letters(p, &WORD, 0)[0].y;
        assert!(fall(0.0) < -0.8 && (0..100).any(|k| fall(f64::from(k) / 100.0) > 0.0));
    }

    #[test]
    fn decode_scrambles_by_kind_the_same_every_time_and_settles_from_the_left() {
        let word = ["A", "b", "7", "!"];
        let at = |p: f64| Preset::Decode.letters(p, &word, 42);
        assert_eq!(at(0.3), at(0.3));
        let first = at(0.1);
        assert!(first[0].g.is_some_and(|c| c.is_ascii_uppercase()));
        assert!(first[1].g.is_some_and(|c| c.is_ascii_lowercase()));
        assert!(first[2].g.is_some_and(|c| c.is_ascii_digit()));
        assert_eq!(first[3].g, None, "punctuation stays");
        // later on, the first letter has settled before the last
        let late = at(0.8);
        assert!(late[0].g.is_none() && late[2].g.is_some());
        // another word scrambles otherwise
        let other = Preset::Decode.letters(0.1, &word, 43);
        assert_ne!(first, other);
    }
}
