//! Procedural world generators (Create -> Geometry -> Worlds, and as stack
//! modifiers): biome Terrain with erosion and slope / altitude Material IDs,
//! a Gerstner trochoidal Ocean with crest foam, and the layered Planet
//! Studio (surface, cloud shell, atmosphere halo, rings). Deterministic per
//! seed; Z-up like the rest of the scene.

pub mod buildings;
pub mod clouds;
mod erosion;
pub mod kit;
mod ocean;
mod planet;
mod planet_layers;
mod planet_surface;
pub mod ruin;
pub mod settlement;
pub mod pyro;
pub mod smoke;
pub mod smoke_sim;
pub mod trails;
pub mod vdb;
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
    /// Procedural settlement (Create -> Geometry -> Settlement).
    Settlement(Box<SettlementParams>),
    /// One building / prop (Create -> Geometry -> Buildings & Props).
    Building(Box<buildings::BuildingParams>),
    /// PR 8 Smoke & Fire Plume (Create -> Geometry -> Worlds / FX).
    Smoke(Box<smoke::SmokeParams>),
    /// Clouds PR 1: one 3D cloud (Create -> Geometry -> Worlds / FX).
    Cloud(Box<clouds::CloudParams>),
    /// Clouds PR 1: a layer of scattered clouds.
    CloudField(Box<clouds::CloudFieldParams>),
    /// Clouds PR 3: a contrail, smoke or flame trail.
    Trail(Box<trails::TrailParams>),
    /// Clouds PR 4: an explosion or mushroom cloud.
    Pyro(Box<pyro::PyroParams>),
    /// Step C: an imported OpenVDB smoke / fire simulation.
    Vdb(Box<vdb::VdbParams>),
    /// Step D: a grid smoke / fire simulation baked in the background.
    SmokeSim(Box<smoke_sim::SmokeSimParams>),
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
    Cloud(clouds::CloudType),
    CloudField,
    Trail(trails::TrailKind),
    Pyro(pyro::PyroKind),
    Vdb,
    SmokeSim,
    FireSim,
}

impl WorldKind {
    /// Create -> Geometry -> Clouds buttons.
    pub const CLOUDS: [WorldKind; 9] = {
        let mut out = [Self::CloudField; 9];
        let mut i = 0;
        while i < 8 {
            out[i] = Self::Cloud(clouds::CloudType::ALL[i]);
            i += 1;
        }
        out
    };
    /// Create -> Geometry -> Trails buttons.
    pub const TRAILS: [WorldKind; 6] = {
        let mut out = [Self::Terrain; 6];
        let mut i = 0;
        while i < 6 {
            out[i] = Self::Trail(trails::TrailKind::ALL[i]);
            i += 1;
        }
        out
    };
    /// Create -> Geometry -> Explosions buttons.
    pub const PYRO: [WorldKind; 6] = {
        let mut out = [Self::Terrain; 6];
        let mut i = 0;
        while i < 6 {
            out[i] = Self::Pyro(pyro::PyroKind::ALL[i]);
            i += 1;
        }
        out
    };
    pub const ALL: [WorldKind; 4] = [Self::Terrain, Self::Ocean, Self::Planet, Self::Smoke];
    /// Create -> Geometry -> Simulations buttons.
    pub const SIMS: [WorldKind; 2] = [Self::SmokeSim, Self::FireSim];
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
            Self::Cloud(k) => k.label(),
            Self::CloudField => "Cloud Field",
            Self::Trail(k) => k.label(),
            Self::Pyro(k) => k.label(),
            Self::Vdb => "VDB Volume",
            Self::SmokeSim => "Smoke Simulation",
            Self::FireSim => "Fire Simulation",
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
            Self::Cloud(k) => k.tip(),
            Self::CloudField => "A whole layer of sky: scatters a mix of cloud types",
            Self::Trail(k) => k.tip(),
            Self::Pyro(k) => k.tip(),
            Self::Vdb => "Simulated smoke and fire imported from an OpenVDB file",
            Self::SmokeSim => "Rising, swirling smoke simulated on a grid, frame by frame",
            Self::FireSim => "Burning fuel simulated on a grid: flames below, soot above",
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
            WorldKind::Cloud(k) => Self::Cloud(Box::new(clouds::CloudParams::of(k))),
            WorldKind::CloudField => Self::CloudField(Box::default()),
            WorldKind::Trail(k) => Self::Trail(Box::new(trails::TrailParams::of(k))),
            WorldKind::Pyro(k) => Self::Pyro(Box::new(pyro::PyroParams::of(k))),
            WorldKind::Vdb => Self::Vdb(Box::default()),
            WorldKind::SmokeSim => Self::SmokeSim(Box::default()),
            WorldKind::FireSim => Self::SmokeSim(Box::new(smoke_sim::SmokeSimParams::fire_preset())),
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
            Self::Cloud(p) => WorldKind::Cloud(p.kind),
            Self::CloudField(_) => WorldKind::CloudField,
            Self::Trail(p) => WorldKind::Trail(p.kind),
            Self::Pyro(p) => WorldKind::Pyro(p.kind),
            Self::Vdb(_) => WorldKind::Vdb,
            Self::SmokeSim(p) => if p.sim.fuel > 0.0 { WorldKind::FireSim } else { WorldKind::SmokeSim },
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
            Self::Cloud(p) => p.build(),
            Self::CloudField(p) => p.build(),
            Self::Trail(p) => p.build(),
            Self::Pyro(p) => p.build(),
            Self::Vdb(p) => p.build(),
            Self::SmokeSim(p) => p.build(),
        }
    }
    /// Object name prefix (Terrain001…).
    pub fn short(&self) -> &'static str {
        match self {
            Self::Cloud(_) => "Cloud",
            Self::Trail(_) => "Trail",
            Self::Pyro(_) => "Explosion",
            Self::Vdb(_) => "VDB",
            Self::SmokeSim(_) => "SmokeSim",
            _ => self.kind().label(),
        }
    }
    /// The Multi/Sub-Object pack whose IDs this generator assigns.
    pub fn material_pack(&self) -> crate::materials::MultiPreset {
        use crate::materials::MultiPreset;
        match self {
            Self::Terrain(_) => MultiPreset::AlpineTerrain,
            Self::Ocean(_) => MultiPreset::OceanWater,
            Self::Planet(_) => MultiPreset::PlanetStudio,
            Self::Settlement(_) => MultiPreset::Settlement,
            Self::Building(p) => p.style.pack(),
            Self::Smoke(_) | Self::Trail(_) | Self::Pyro(_) | Self::Vdb(_) | Self::SmokeSim(_) => MultiPreset::SmokeFire,
            Self::Cloud(_) | Self::CloudField(_) => MultiPreset::Clouds,
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
