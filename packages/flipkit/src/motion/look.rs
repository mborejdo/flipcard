//! How one word looks at one instant: what `word()` in `motion.typ` applies inside the word's box.

use serde_json::{Map, Value};

/// A word's look; the default is the word at rest.
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
    /// How far its colour has turned to the accent, 0 to 1.
    pub tint: f64,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            o: 1.0,
            x: 0.0,
            y: 0.0,
            s: 1.0,
            draw: 1.0,
            tint: 0.0,
        }
    }
}

impl Look {
    /// The look as JSON: values rounded to 1e-4, a field at its default left out.
    pub(crate) fn json(self) -> Value {
        let rest = Self::default();
        let fields = [
            ("o", self.o, rest.o),
            ("x", self.x, rest.x),
            ("y", self.y, rest.y),
            ("s", self.s, rest.s),
            ("draw", self.draw, rest.draw),
            ("tint", self.tint, rest.tint),
        ];
        let mut map = Map::new();
        for (key, value, default) in fields {
            let value = (value * 1e4).round() / 1e4;
            if (value - default).abs() > 5e-5 {
                map.insert(key.into(), value.into());
            }
        }
        Value::Object(map)
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
            ..Look::default()
        };
        assert_eq!(look.json().to_string(), r#"{"o":0.1235,"y":1.0}"#);
    }
}
