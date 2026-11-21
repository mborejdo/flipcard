//! Motion: Rust owns time, Typst draws. A scene's layout is its final state; per frame a small
//! JSON of per-word looks moves, scales and fades the words inside their boxes, so nothing
//! reflows.

mod count;
mod ease;
mod look;
mod preset;
mod timeline;

pub(crate) use count::count;
pub(crate) use ease::Curve;
pub(crate) use look::Look;
pub use preset::Preset;
pub(crate) use timeline::{Cue, Timeline};
