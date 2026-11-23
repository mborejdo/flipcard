//! A spring: a damped harmonic oscillator of unit mass, solved in closed form with `libm`, so
//! every platform computes the same bits. Set as `SwiftUI` sets one
//! (`spring(response:dampingFraction:)`): `response` is the period of the undamped spring,
//! `damping` the damping fraction (1 settles fastest without passing its rest, below it bounces,
//! above it creeps). The three regimes as framer-motion and popmotion solve them; time in
//! half-beats.

use std::f64::consts::PI;

/// Displacement (and velocity per [`Spring::omega`]) below which a spring is at rest, of how
/// far it started: the 1e-4 the looks are rounded to.
const REST: f64 = 1e-4;
/// Half-beats between two looks while finding when a critically damped spring rests.
const STEP: f64 = 0.05;

/// A damped spring; see the module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Spring {
    /// Half-beats of one swing of the undamped spring.
    pub response: f64,
    /// The damping fraction: 1 critical, below it bounces, above it creeps.
    pub damping: f64,
}

/// The letters of `drop` land on this one: a quick fall, one small bounce.
pub(crate) const DROP: Spring = Spring {
    response: 1.6,
    damping: 0.55,
};

/// The lines of `stack` move up on this one, a new line taking over their momentum.
pub(crate) const STACK: Spring = Spring {
    response: 2.5,
    damping: 0.85,
};

#[allow(
    clippy::many_single_char_names,
    reason = "the oscillator's textbook names: w, z, a, b, c, e, t"
)]
impl Spring {
    /// The undamped angular frequency, per half-beat.
    fn omega(self) -> f64 {
        2.0 * PI / self.response
    }

    /// Displacement from rest and velocity `t` half-beats after it was let go at `x0`, moving at
    /// `v0`.
    pub(crate) fn at(self, x0: f64, v0: f64, t: f64) -> (f64, f64) {
        if t <= 0.0 {
            return (x0, v0);
        }
        let (w, z) = (self.omega(), self.damping);
        if (z - 1.0).abs() < 1e-9 {
            // critical: e^-wt (x0 + (v0 + w x0) t)
            let (c, e) = (w.mul_add(x0, v0), libm::exp(-w * t));
            (e * c.mul_add(t, x0), e * (w * c).mul_add(-t, v0))
        } else if z < 1.0 {
            // under: a decaying swing at the damped frequency
            let (a, b) = (z * w, w * libm::sqrt(z.mul_add(-z, 1.0)));
            let c = a.mul_add(x0, v0) / b;
            let (e, sin, cos) = (libm::exp(-a * t), libm::sin(b * t), libm::cos(b * t));
            let dv = (w * w).mul_add(x0, a * v0) / b;
            (e * x0.mul_add(cos, c * sin), e * v0.mul_add(cos, -dv * sin))
        } else {
            // over: two decays, the slow one last
            let r = w * libm::sqrt(z.mul_add(z, -1.0));
            let (r1, r2) = ((-z).mul_add(w, r), (-z).mul_add(w, -r));
            let c1 = r2.mul_add(-x0, v0) / (r1 - r2);
            let c2 = x0 - c1;
            let (e1, e2) = (libm::exp(r1 * t), libm::exp(r2 * t));
            (c1.mul_add(e1, c2 * e2), (c1 * r1).mul_add(e1, c2 * r2 * e2))
        }
    }

    /// Half-beats till it rests, let go at `x0` moving at `v0`: from when its envelope, the bound
    /// of its displacement and of its velocity, stays within [`REST`] of rest, scaled by how far
    /// it starts.
    pub(crate) fn settle(self, x0: f64, v0: f64) -> f64 {
        let (w, z) = (self.omega(), self.damping);
        let scale = x0.abs().max(v0.abs() / w);
        if scale == 0.0 {
            return 0.0;
        }
        let rest = REST * scale;
        // displacement and velocity (per w) bounded by `m` e^(-rate t): t = ln(m / rest) / rate
        let decay = |m: f64, rate: f64| (libm::log(m / rest) / rate).max(0.0);
        if (z - 1.0).abs() < 1e-9 {
            // critical: e^-wt (|x0| + |c| t), and the like for the velocity; on a grid
            let c = w.mul_add(x0, v0).abs();
            let bound = |t: f64| {
                let e = libm::exp(-w * t);
                (e * c.mul_add(t, x0.abs())).max(e * (w * c).mul_add(t, v0.abs()) / w)
            };
            let mut k = 0_u32;
            while bound(f64::from(k) * STEP) > rest && k < 1_000_000 {
                k += 1;
            }
            f64::from(k) * STEP
        } else if z < 1.0 {
            let (a, b) = (z * w, w * libm::sqrt(z.mul_add(-z, 1.0)));
            let c = a.mul_add(x0, v0) / b;
            let dv = (w * w).mul_add(x0, a * v0) / b;
            let m = libm::sqrt(x0.mul_add(x0, c * c)).max(libm::sqrt(v0.mul_add(v0, dv * dv)) / w);
            decay(m, a)
        } else {
            let r = w * libm::sqrt(z.mul_add(z, -1.0));
            let (r1, r2) = ((-z).mul_add(w, r), (-z).mul_add(w, -r));
            let c1 = r2.mul_add(-x0, v0) / (r1 - r2);
            let c2 = x0 - c1;
            let m = (c1.abs() + c2.abs()).max(((c1 * r1).abs() + (c2 * r2).abs()) / w);
            decay(m, -r1)
        }
    }

    /// An entrance from 0 to 1 on the spring, let go at rest: `p` (0 to 1) of the way to its
    /// settling. Both ends exact.
    pub(crate) fn ease(self, p: f64) -> f64 {
        if p <= 0.0 {
            return 0.0;
        }
        if p >= 1.0 {
            return 1.0;
        }
        1.0 - self.at(1.0, 0.0, p * self.settle(1.0, 0.0)).0
    }

    /// Where a spring that follows a moving target is at `t`, and how fast: it starts at rest at
    /// `from`; at each `(at, to)` of `moves` (in order) its target jumps to `to`, and it carries
    /// on from where it is, at the speed it has.
    pub(crate) fn follow(self, from: f64, moves: &[(f64, f64)], t: f64) -> (f64, f64) {
        let (mut x, mut v, mut since, mut target) = (from, 0.0, 0.0_f64, from);
        for &(at, to) in moves.iter().take_while(|(at, _)| *at <= t) {
            let (d, speed) = self.at(x - target, v, at - since);
            (x, v, since, target) = (target + d, speed, at, to);
        }
        let (d, speed) = self.at(x - target, v, t - since);
        (target + d, speed)
    }

    /// The half-beat a spring following `moves` from `from` comes to rest after the last.
    pub(crate) fn rests(self, from: f64, moves: &[(f64, f64)]) -> f64 {
        let Some(&(last, to)) = moves.last() else {
            return 0.0;
        };
        let (x, v) = self.follow(from, moves, last);
        last + self.settle(x - to, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNDER: Spring = Spring {
        response: 2.0,
        damping: 0.4,
    };
    const CRITICAL: Spring = Spring {
        response: 2.0,
        damping: 1.0,
    };
    const OVER: Spring = Spring {
        response: 2.0,
        damping: 2.0,
    };

    #[test]
    fn every_spring_starts_where_it_was_let_go_and_settles() {
        for spring in [UNDER, CRITICAL, OVER, DROP, STACK] {
            assert_eq!(spring.at(1.0, 0.5, 0.0), (1.0, 0.5));
            let rest = spring.settle(1.0, 0.0);
            assert!(rest > 0.0 && rest < 40.0, "{spring:?}: {rest}");
            let (x, v) = spring.at(1.0, 0.0, rest);
            assert!(
                x.abs() <= REST && v.abs() <= REST * spring.omega(),
                "{spring:?}"
            );
            assert_eq!((spring.ease(0.0), spring.ease(1.0)), (0.0, 1.0));
            assert_eq!((spring.ease(-1.0), spring.ease(2.0)), (0.0, 1.0));
        }
        assert_eq!(UNDER.settle(0.0, 0.0), 0.0);
    }

    #[test]
    fn the_velocity_is_the_slope_of_the_position() {
        for spring in [UNDER, CRITICAL, OVER] {
            for t in [0.1, 0.7, 2.3] {
                let h = 1e-6;
                let slope =
                    (spring.at(1.0, -2.0, t + h).0 - spring.at(1.0, -2.0, t - h).0) / (2.0 * h);
                let (_, v) = spring.at(1.0, -2.0, t);
                assert!(
                    (slope - v).abs() < 1e-5,
                    "{spring:?} at {t}: {slope} vs {v}"
                );
            }
        }
    }

    #[test]
    fn under_damped_passes_its_rest_critical_and_over_damped_never_do() {
        let samples = |spring: Spring| -> Vec<f64> {
            (0..=1000)
                .map(|i| spring.ease(f64::from(i) / 1000.0))
                .collect()
        };
        let peak = |spring| samples(spring).into_iter().fold(0.0, f64::max);
        assert!(peak(UNDER) > 1.2 && peak(DROP) > 1.05);
        for spring in [CRITICAL, OVER] {
            assert!(peak(spring) <= 1.0, "{spring:?}");
            assert!(
                samples(spring).windows(2).all(|w| w[1] >= w[0]),
                "{spring:?}"
            );
        }
        // the more damping, the slower it settles past critical
        assert!(OVER.settle(1.0, 0.0) > CRITICAL.settle(1.0, 0.0));
    }

    #[test]
    fn a_retarget_keeps_where_it_is_and_how_fast_it_moves() {
        // 10 down to 4 at 0, then to 0 at 1: no jump in place or speed at the retarget
        let moves = [(0.0, 4.0), (1.0, 0.0)];
        let (before, after) = (
            STACK.follow(10.0, &moves, 1.0 - 1e-9),
            STACK.follow(10.0, &moves, 1.0),
        );
        assert!((before.0 - after.0).abs() < 1e-6 && (before.1 - after.1).abs() < 1e-6);
        assert!(after.1 < 0.0, "still moving down when the target moves on");
        // it carries its momentum: ahead of a spring let go at rest from the same place
        let fresh = STACK.at(after.0, 0.0, 0.5).0;
        assert!(STACK.follow(10.0, &moves, 1.5).0 < fresh);
        // at rest once it settles, on the last target
        let rest = STACK.rests(10.0, &moves);
        assert!(STACK.follow(10.0, &moves, rest).0.abs() < 1e-3);
        assert_eq!(STACK.follow(10.0, &[], 5.0), (10.0, 0.0));
    }
}
