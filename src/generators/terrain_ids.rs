//! `[✓] Auto-Assign Material IDs by Slope & Altitude` for Terrain: ID 1
//! flat ground / grass, ID 2 steep cliff rock, ID 3 high snow / peaks,
//! ID 4 low sand / riverbed, matching the Alpine Terrain Pack.

use super::terrain::Biome;
use crate::mesh::EditablePoly;
use bevy::math::Vec3;

pub const ID_GRASS: u16 = 1;
pub const ID_CLIFF: u16 = 2;
pub const ID_SNOW: u16 = 3;
pub const ID_SAND: u16 = 4;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TerrainIds {
    pub enabled: bool,
    /// Slopes steeper than this (degrees) are cliff rock.
    pub cliff_angle: f32,
    /// Normalized altitude (0 = lowest, 1 = highest) above which snow lies.
    pub snow_line: f32,
    /// Normalized altitude below which sand / riverbed lies.
    pub sand_line: f32,
}

impl Default for TerrainIds {
    fn default() -> Self {
        Self::of(Biome::RollingHills)
    }
}

impl TerrainIds {
    pub fn of(biome: Biome) -> Self {
        let base = Self { enabled: true, cliff_angle: 38.0, snow_line: 0.88, sand_line: 0.1 };
        match biome {
            Biome::Plains => Self { snow_line: 1.1, sand_line: 0.12, ..base },
            Biome::RollingHills => base,
            Biome::JaggedMountains => Self { cliff_angle: 40.0, snow_line: 0.62, sand_line: 0.06, ..base },
            Biome::Mesa => Self { cliff_angle: 35.0, snow_line: 1.1, sand_line: 0.12, ..base },
            Biome::Savannah => Self { snow_line: 1.1, sand_line: 0.22, ..base },
            // Dunes are sand everywhere except the occasional rock outcrop.
            Biome::Dunes => Self { cliff_angle: 48.0, snow_line: 1.1, sand_line: 1.1, ..base },
        }
    }

    /// The ID for a polygon with `slope` (degrees from flat) at normalized
    /// altitude `t`.
    pub fn id_for(&self, slope: f32, t: f32) -> u16 {
        if t >= self.snow_line && slope < self.cliff_angle + 15.0 {
            ID_SNOW
        } else if slope >= self.cliff_angle {
            ID_CLIFF
        } else if t <= self.sand_line {
            ID_SAND
        } else {
            ID_GRASS
        }
    }

    /// Sets every polygon's Material ID from its slope and altitude.
    pub fn assign(&self, m: &mut EditablePoly) {
        let Some(b) = m.bounds() else { return };
        let (lo, span) = (b.min.z, (b.max.z - b.min.z).max(1e-6));
        let keys: Vec<_> = m.polygons.keys().collect();
        for p in keys {
            let n = m.poly_normal(p);
            let slope = n.dot(Vec3::Z).clamp(-1.0, 1.0).acos().to_degrees();
            let t = (m.poly_centroid(p).z - lo) / span;
            m.polygons[p].material_id = self.id_for(slope, t);
        }
    }
}
