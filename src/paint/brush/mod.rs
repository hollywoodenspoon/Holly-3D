//! Brush alphas and battle / decal stamps, and the stroke engine that
//! projects them onto the surface (TextureCraft's `paint` design, MIT /
//! Apache-2.0).

pub mod alphas;
pub mod stroke;
#[cfg(test)]
mod tests;

pub use stroke::{LayerStroke, Stamp};

use super::layers::Flat;

/// Built-in procedural brush alphas and damage stamps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Alpha {
    #[default]
    SoftRound,
    HardRound,
    BulletHole,
    BlastStar,
    BloodSplatter,
    BurnPatch,
    EmberCracks,
    Scratch,
    Rivet,
}

impl Alpha {
    pub const ALL: [Alpha; 9] = [
        Alpha::SoftRound,
        Alpha::HardRound,
        Alpha::BulletHole,
        Alpha::BlastStar,
        Alpha::BloodSplatter,
        Alpha::BurnPatch,
        Alpha::EmberCracks,
        Alpha::Scratch,
        Alpha::Rivet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Alpha::SoftRound => "Soft Round",
            Alpha::HardRound => "Hard Round",
            Alpha::BulletHole => "Bullet Hole / Crater Impact",
            Alpha::BlastStar => "Shell Scorch / Blast Star",
            Alpha::BloodSplatter => "Blood Smear / Splatter",
            Alpha::BurnPatch => "Charred Burn Patch",
            Alpha::EmberCracks => "Glowing Ember Cracks",
            Alpha::Scratch => "Scratch / Slash",
            Alpha::Rivet => "Rivet / Weld Seam",
        }
    }

    /// Damage and decal stamps bring their own colours, roughness, depth
    /// and glow; the round brushes paint with the brush colour or material.
    pub fn is_stamp(self) -> bool {
        !matches!(self, Alpha::SoftRound | Alpha::HardRound)
    }

    /// Distance between stamps along a stroke, in brush radii.
    pub fn spacing(self) -> f32 {
        match self {
            Alpha::SoftRound | Alpha::HardRound => 0.3,
            Alpha::Scratch | Alpha::BloodSplatter => 0.5,
            Alpha::Rivet => 2.6,
            _ => 1.6,
        }
    }

    /// Stamps that turn to a random angle each time (the rest follow the stroke).
    pub fn spins(self) -> bool {
        matches!(self, Alpha::BulletHole | Alpha::BlastStar | Alpha::BloodSplatter | Alpha::BurnPatch | Alpha::EmberCracks)
    }
}

/// One brush alpha evaluated at a point of the brush disc.
#[derive(Clone, Debug, PartialEq)]
pub struct Dab {
    /// 0..1 strength at this point.
    pub cov: f32,
    /// What a stamp paints here (`None` = the brush's own source).
    pub look: Option<Flat>,
}
