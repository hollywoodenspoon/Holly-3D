//! CAD hard-surface tools after SolidCraft (MIT / Apache-2.0): the
//! multi-profile Loft and the Hole / Counterbore / Countersink cutter.

pub mod hole;
pub mod loft;
#[cfg(test)]
mod tests;

pub use hole::{HoleKind, HoleSpec};
pub use loft::{LoftMod, LoftSection};
