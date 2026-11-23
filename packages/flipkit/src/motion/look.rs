//! How one word, or one letter of it, looks at one instant: what `word()` in `motion.typ`
//! applies inside the word's box.

use serde_json::{Map, Value};

/// A word's look, or a letter's; the default is at rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Look {
    /// Opacity, 0 to 1.
    pub o: f64,
    /// Offset in em, right and down.
    pub x: f64,
    pub y: f64,
    /// Scale.
    pub s: f64,
    /// How far its mark is drawn, 0 to 1.
    pub draw: f64,
    /// How far it is filled with ink from the left, from muted, 0 to 1.
    pub fill: f64,
    /// How far its colour has turned to its mark's (bold: the accent, struck: muted), 0 to 1.
    pub tint: f64,
    /// A letter drawn in its place (`decode`).
    pub g: Option<char>,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            o: 1.0,
            x: 0.0,
            y: 0.0,
            s: 1.0,
            draw: 1.0,
            fill: 1.0,
            tint: 1.0,
            g: None,
        }
    }
}

impl Look {
    /// From this look to rest, `e` (0 to 1) of the way: where it is and its size.
    pub(crate) fn toward(self, e: f64) -> Self {
        Self {
            x: self.x * (1.0 - e),
            y: self.y * (1.0 - e),
            s: (1.0 - self.s).mul_add(e, self.s),
            ..Self::default()
        }
    }

    /// The look as JSON: values rounded to 1e-4, a field at its default left out.
    pub(crate) fn json(self) -> Value {
        let rest = Self::default();
        let fields = [
            ("o", self.o, rest.o),
            ("x", self.x, rest.x),
            ("y", self.y, rest.y),
            ("s", self.s, rest.s),
            ("draw", self.draw, rest.draw),
            ("fill", self.fill, rest.fill),
            ("tint", self.tint, rest.tint),
        ];
        let mut map = Map::new();
        for (key, value, default) in fields {
            let value = (value * 1e4).round() / 1e4;
            if (value - default).abs() > 5e-5 {
                map.insert(key.into(), value.into());
            }
        }
        if let Some(g) = self.g {
            map.insert("g".into(), g.to_string().into());
        }
        Value::Object(map)
    }
}

/// A word's letters as one look when they are alike (the word moves as one, as a word look
/// does), else each its own: `(word, letters)`, the letters empty when they are alike.
pub(crate) fn shape(letters: Vec<Look>) -> (Look, Vec<Look>) {
    match letters.split_first() {
        Some((first, rest)) if rest.iter().all(|l| l == first) => (*first, Vec::new()),
        Some(_) => (Look::default(), letters),
        None => (Look::default(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_at_rest_is_empty_and_values_are_rounded() {
        assert_eq!(Look::default().json().to_string(), "{}");
        let look = Look {
            o: 0.123_456,
            y: 0.999_99,
            draw: 0.999_99,
            g: Some('Q'),
            ..Look::default()
        };
        assert_eq!(look.json().to_string(), r#"{"g":"Q","o":0.1235,"y":1.0}"#);
    }

    #[test]
    fn letters_alike_are_one_look_for_the_word() {
        let faded = Look {
            o: 0.5,
            ..Look::default()
        };
        assert_eq!(shape(vec![faded; 3]), (faded, vec![]));
        let typed = vec![Look::default(), faded];
        assert_eq!(shape(typed.clone()), (Look::default(), typed));
        assert_eq!(shape(vec![]), (Look::default(), vec![]));
    }
}
