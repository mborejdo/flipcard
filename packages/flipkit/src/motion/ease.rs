//! Easing curves: progress 0..1 in, eased progress out, both ends exact.

use std::f64::consts::PI;

/// How an entrance moves through its time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Curve {
    /// Even speed.
    Linear,
    /// Fast, then settling (power3.out).
    Ease,
    /// Past the end and back (peak about 1.10).
    Back,
    /// A damped bounce (first peak about 1.09).
    Spring,
}

impl Curve {
    /// The curve at `x`: 0 at or before 0, 1 at or after 1.
    pub(crate) fn at(self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        match self {
            Self::Linear => x,
            Self::Ease => 1.0 - libm::pow(1.0 - x, 3.0),
            Self::Back => {
                let (c, u) = (1.701_58_f64, x - 1.0);
                (c + 1.0).mul_add(u * u * u, c * u * u) + 1.0
            },
            Self::Spring => libm::exp(-6.9 * x).mul_add(-libm::cos(2.5 * PI * x), 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Curve; 4] = [Curve::Linear, Curve::Ease, Curve::Back, Curve::Spring];

    fn samples(curve: Curve) -> Vec<f64> {
        (0..=1000)
            .map(|i| curve.at(f64::from(i) / 1000.0))
            .collect()
    }

    #[test]
    fn every_curve_starts_at_0_and_ends_at_1_exactly() {
        for curve in ALL {
            assert_eq!((curve.at(0.0), curve.at(1.0)), (0.0, 1.0), "{curve:?}");
            assert_eq!((curve.at(-1.0), curve.at(2.0)), (0.0, 1.0), "{curve:?}");
        }
    }

    #[test]
    fn ease_only_rises_back_and_spring_overshoot() {
        assert!(samples(Curve::Ease).windows(2).all(|w| w[1] >= w[0]));
        let peak = |curve| samples(curve).into_iter().fold(0.0, f64::max);
        assert!(
            (peak(Curve::Back) - 1.10).abs() < 0.005,
            "{}",
            peak(Curve::Back)
        );
        assert!(
            (peak(Curve::Spring) - 1.09).abs() < 0.01,
            "{}",
            peak(Curve::Spring)
        );
    }
}
