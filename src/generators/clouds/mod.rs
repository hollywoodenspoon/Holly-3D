//! Clouds PR 1: placeable 3D cloud objects and the Cloud Field (Create ->
//! Geometry -> Worlds / FX, and the Demiurge tab).
//!
//! Every cloud is a `CloudShape`: a seeded list of puffs (spheres squashed
//! vertically) laid out by per-type rules, plus smaller puffs grown on the
//! bigger ones (cauliflower detail). The puffs are the one source of truth:
//! today they become a light preview mesh, and the volume renderer turns
//! the same puffs into density. Z-up, base of the cloud at z = 0.

mod field;
mod look;
pub(crate) mod mesh;
mod puffs;
#[cfg(test)]
mod tests;

pub use field::{CloudFieldParams, Placement};
pub use look::{CloudLook, RenderAs};
pub use puffs::{CloudShape, Puff};

use crate::mesh::EditablePoly;

/// The cloud families (`PR 1` set; more types arrive later).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CloudType {
    /// Small, puffy, flat-based fair-weather cloud.
    #[default]
    Cumulus,
    /// Tall, lumpy tower.
    Congestus,
    /// Huge storm tower with a flat anvil and a dark base.
    Cumulonimbus,
    /// A wide sheet of rolls with gaps between them.
    Stratocumulus,
}

impl CloudType {
    pub const ALL: [CloudType; 4] = [Self::Cumulus, Self::Congestus, Self::Cumulonimbus, Self::Stratocumulus];
    pub fn label(self) -> &'static str {
        match self {
            Self::Cumulus => "Cumulus",
            Self::Congestus => "Cumulus Congestus",
            Self::Cumulonimbus => "Cumulonimbus",
            Self::Stratocumulus => "Stratocumulus",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Self::Cumulus => "Small puffy cloud with a flat base: scattered cover to weave between",
            Self::Congestus => "Tall lumpy tower: a wall to hide behind",
            Self::Cumulonimbus => "Huge storm tower with a flat anvil on top",
            Self::Stratocumulus => "Wide sheet of rolls with gaps: a deck to dive through",
        }
    }
    /// Typical footprint width and height (metres).
    pub fn default_size(self) -> (f32, f32) {
        match self {
            Self::Cumulus => (300.0, 180.0),
            Self::Congestus => (420.0, 650.0),
            Self::Cumulonimbus => (1400.0, 2200.0),
            Self::Stratocumulus => (1200.0, 160.0),
        }
    }
}

/// One cloud object's parameters (the Modify panel "Cloud" rollout).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CloudParams {
    pub kind: CloudType,
    pub seed: u32,
    /// Footprint width (m).
    pub size: f32,
    /// Height from base to top (m).
    pub height: f32,
    /// Puffiness 0..=1: how big the detail puffs bulge out of their parents.
    pub puffiness: f32,
    /// Lumpiness 0..=1: random variation of puff sizes and positions.
    pub lumpiness: f32,
    /// Base Flatness 0..=1: how flat the underside is cut.
    pub base_flatness: f32,
    /// Wind Shear 0..=1: how far the top leans downwind.
    pub shear: f32,
    /// Wind direction (compass degrees, 0 = +X).
    pub shear_dir: f32,
    /// Detail Levels 1..=3 (puff-on-puff depth).
    pub detail: u32,
    /// How it renders (Clouds V2: volume by default).
    pub look: CloudLook,
}

impl Default for CloudParams {
    fn default() -> Self {
        Self::of(CloudType::Cumulus)
    }
}

impl CloudParams {
    /// The preset for `kind` (Preset dropdown).
    pub fn of(kind: CloudType) -> Self {
        let (size, height) = kind.default_size();
        let (puffiness, lumpiness, base_flatness, shear) = match kind {
            CloudType::Cumulus => (0.55, 0.4, 0.85, 0.05),
            CloudType::Congestus => (0.6, 0.6, 0.8, 0.1),
            CloudType::Cumulonimbus => (0.5, 0.5, 0.9, 0.15),
            CloudType::Stratocumulus => (0.4, 0.35, 0.6, 0.0),
        };
        Self { kind, seed: 1, size, height, puffiness, lumpiness, base_flatness, shear, shear_dir: 0.0, detail: 3, look: CloudLook::default() }
    }

    /// The puff layout.
    pub fn shape(&self) -> CloudShape {
        puffs::layout(self)
    }

    /// The preview mesh (Material ID 1, Clouds pack).
    pub fn build(&self) -> EditablePoly {
        mesh::build(&self.shape(), self.base_flatness)
    }
}
