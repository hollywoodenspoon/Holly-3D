//! Procedural world generators (Create -> Geometry -> Worlds, and as stack
//! modifiers): biome Terrain with erosion and slope / altitude Material IDs,
//! a Gerstner trochoidal Ocean with crest foam, and the layered Planet
//! Studio (surface, cloud shell, atmosphere halo, rings). Deterministic per
//! seed; Z-up like the rest of the scene.

pub mod buildings;
mod erosion;
pub mod kit;
mod ocean;
mod planet;
mod planet_layers;
mod planet_surface;
pub mod ruin;
pub mod settlement;
pub mod smoke;
mod sphere;
mod terrain;
mod terrain_ids;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_planet;

pub use erosion::Heightmap;
pub use ocean::{OceanMod, OceanParams};
pub use planet::{PlanetParams, PlanetPart};
pub use planet_layers::{Atmosphere, CloudShell, Rings};
pub use planet_surface::SurfacePreset;
pub use settlement::{Era, PropStyle, SettlementMod, SettlementParams};
pub use sphere::equal_angle_sphere;
pub use terrain::{Biome, TerrainMod, TerrainParams};
pub use terrain_ids::TerrainIds;

use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Vec2, Vec3};

/// The procedural world objects (Create -> Geometry -> Worlds).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WorldGen {
    Terrain(TerrainParams),
    Ocean(OceanParams),
    Planet(Box<PlanetParams>),
    /// GhostTown settlement (Create -> Geometry -> Settlement).
    Settlement(Box<SettlementParams>),
    /// One building / prop (Create -> Geometry -> Buildings & Props).
    Building(Box<buildings::BuildingParams>),
    /// PR 8 Smoke & Fire Plume (Create -> Geometry -> Worlds / FX).
    Smoke(Box<smoke::SmokeParams>),
}

/// Create-tab buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldKind {
    Terrain,
    Ocean,
    Planet,
    Settlement(Era),
    Building(buildings::BuildingKind),
    Smoke,
}

impl WorldKind {
    pub const ALL: [WorldKind; 4] = [Self::Terrain, Self::Ocean, Self::Planet, Self::Smoke];
    /// Create -> Geometry -> Settlement buttons.
    pub const SETTLEMENTS: [WorldKind; 14] = {
        let mut out = [Self::Terrain; 14];
        let mut i = 0;
        while i < 14 {
            out[i] = Self::Settlement(Era::ALL[i]);
            i += 1;
        }
        out
    };
    pub fn label(self) -> &'static str {
        match self {
            Self::Terrain => "Terrain",
            Self::Ocean => "Ocean",
            Self::Planet => "Planet",
            Self::Settlement(e) => e.short(),
            Self::Building(k) => k.label(),
            Self::Smoke => "Smoke Plume",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Self::Terrain => "Biome terrain: hills, mountains, mesas, savannah, dunes",
            Self::Ocean => "Gerstner wave surface with crest foam",
            Self::Planet => "Planet Studio: surface, clouds, atmosphere and rings",
            Self::Settlement(e) => e.label(),
            Self::Building(k) => k.category().label(),
            Self::Smoke => "Billowing smoke & fire plume: wind lean, mushroom cap, rise phase",
        }
    }
}

impl WorldGen {
    pub fn default_of(kind: WorldKind) -> Self {
        match kind {
            WorldKind::Terrain => Self::Terrain(TerrainParams::default()),
            WorldKind::Ocean => Self::Ocean(OceanParams::default()),
            WorldKind::Planet => Self::Planet(Box::default()),
            WorldKind::Settlement(e) => Self::Settlement(Box::new(SettlementParams::of(e))),
            WorldKind::Building(k) => Self::Building(Box::new(buildings::BuildingParams::of(k))),
            WorldKind::Smoke => Self::Smoke(Box::default()),
        }
    }
    pub fn kind(&self) -> WorldKind {
        match self {
            Self::Terrain(_) => WorldKind::Terrain,
            Self::Ocean(_) => WorldKind::Ocean,
            Self::Planet(_) => WorldKind::Planet,
            Self::Settlement(p) => WorldKind::Settlement(p.era),
            Self::Building(p) => WorldKind::Building(p.kind),
            Self::Smoke(_) => WorldKind::Smoke,
        }
    }
    pub fn build(&self) -> EditablePoly {
        match self {
            Self::Terrain(t) => t.build(),
            Self::Ocean(o) => o.build(),
            Self::Planet(p) => p.build(),
            Self::Settlement(p) => p.build(),
            Self::Building(p) => p.build(),
            Self::Smoke(p) => p.build(),
        }
    }
    /// Object name prefix (Terrain001…).
    pub fn short(&self) -> &'static str {
        self.kind().label()
    }
    /// The Multi/Sub-Object pack whose IDs this generator assigns.
    pub fn material_pack(&self) -> crate::materials::MultiPreset {
        use crate::materials::MultiPreset;
        match self {
            Self::Terrain(_) => MultiPreset::AlpineTerrain,
            Self::Ocean(_) => MultiPreset::OceanWater,
            Self::Planet(_) => MultiPreset::PlanetStudio,
            Self::Settlement(_) => MultiPreset::GhostTown,
            Self::Building(p) => p.style.pack(),
            Self::Smoke(_) => MultiPreset::SmokeFire,
        }
    }
}

/// A flat `segs` x `segs` quad grid (`w` along X, `l` along Y, centered),
/// UVs 0..1, heights from `h(x, y)`. Returns the mesh and the vertex ids in
/// row-major order (row = Y).
pub fn grid(w: f32, l: f32, segs: u32, h: impl Fn(f32, f32) -> f32) -> (EditablePoly, Vec<VertexId>) {
    let n = segs.max(1) as usize;
    let mut m = EditablePoly::new();
    let mut ids = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let (u, v) = (i as f32 / n as f32, j as f32 / n as f32);
            let (x, y) = ((u - 0.5) * w, (v - 0.5) * l);
            ids.push(m.add_vertex(Vec3::new(x, y, h(x, y))));
        }
    }
    let w1 = n + 1;
    let uv = |i: usize, j: usize| Vec2::new(i as f32 / n as f32, j as f32 / n as f32);
    for j in 0..n {
        for i in 0..n {
            let q = [ids[j * w1 + i], ids[j * w1 + i + 1], ids[(j + 1) * w1 + i + 1], ids[(j + 1) * w1 + i]];
            m.add_polygon_uv(&q, &[uv(i, j), uv(i + 1, j), uv(i + 1, j + 1), uv(i, j + 1)]);
        }
    }
    (m, ids)
}

/// Hermite smoothstep from `a` to `b`.
pub(crate) fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Seeded deterministic hash in 0..1.
pub(crate) fn hash(seed: u32, i: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9) ^ i.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// sRGB vertex color with alpha.
pub(crate) fn rgba(c: Vec3, a: f32) -> bevy::math::Vec4 {
    c.clamp(Vec3::ZERO, Vec3::ONE).extend(a.clamp(0.0, 1.0))
}
