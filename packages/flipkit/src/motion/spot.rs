//! Where a word is at rest: what `word()` in `motion.typ` leaves as metadata.

use super::Look;

/// A word's box at rest, in points: its top left, its size, and its text's size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Spot {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub size: f64,
}

impl Spot {
    /// The word's id and spot from its metadata (`id width height size`) found at `x`, `y`.
    pub(crate) fn read(value: &str, x: f64, y: f64) -> Option<(usize, Self)> {
        let numbers: Vec<f64> = value.split(' ').filter_map(|v| v.parse().ok()).collect();
        let [id, w, h, size] = numbers[..] else {
            return None;
        };
        Some((id as usize, Self { x, y, w, h, size }))
    }

    /// The look that draws the word at this spot where and as big as it was at `was`. A look
    /// scales about the box's middle and moves in em of the word's size.
    pub(crate) fn from(self, was: Self) -> Look {
        let s = was.size / self.size;
        let (dx, dy) = (self.w / 2.0 * (1.0 - s), self.h / 2.0 * (1.0 - s));
        Look {
            x: (was.x - self.x - dx) / self.size,
            y: (was.y - self.y - dy) / self.size,
            s,
            ..Look::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_glides_from_where_it_was_at_its_size_then() {
        let (id, here) = Spot::read("3 40 10 20", 100.0, 200.0).unwrap();
        assert_eq!((id, here.w, here.size), (3, 40.0, 20.0));
        // half the size, its box's top left 20 pt right, 40 pt up
        let was = Spot {
            x: 120.0,
            y: 160.0,
            w: 20.0,
            h: 5.0,
            size: 10.0,
        };
        let look = here.from(was);
        // scaled about the middle (120, 205): the box is 20 by 5 at (110, 202.5), moved to (120, 160)
        assert_eq!((look.x, look.y, look.s), (0.5, -2.125, 0.5));
        assert_eq!(look.toward(1.0), Look::default());
        assert!(Spot::read("3 40", 0.0, 0.0).is_none());
    }
}
