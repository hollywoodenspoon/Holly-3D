//! GeoGen — procedural Quartz Crystal clusters, Jagged Rocks, Layered
//! Cliffs and Stalagmites / Stalactites. Deterministic per seed
//! (`rand_chacha`), shaped with Perlin FBM + Worley noise (`noise`).
//! Used as a Create -> Geometry base object and as a stack modifier that
//! grows formations on (selected) polygons or deforms the input mesh.

mod cliff;
mod rock;
mod spires;
#[cfg(test)]
mod tests;

pub use rock::{deform_rock, quad_sphere};

use crate::mesh::EditablePoly;
use bevy::math::{Mat4, Quat, Vec3};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, Worley};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GeoMode {
    #[default]
    Crystal,
    Rock,
    Cliff,
    Stalagmite,
}

impl GeoMode {
    pub const ALL: [GeoMode; 4] = [Self::Crystal, Self::Rock, Self::Cliff, Self::Stalagmite];
    pub fn label(self) -> &'static str {
        match self {
            Self::Crystal => "Quartz Crystal Cluster",
            Self::Rock => "Jagged Rock / Boulder",
            Self::Cliff => "Layered Cliff / Slate",
            Self::Stalagmite => "Stalagmite / Stalactite",
        }
    }
    /// Object name prefix (Crystal001, Rock001…).
    pub fn short(self) -> &'static str {
        match self {
            Self::Crystal => "Crystal",
            Self::Rock => "Rock",
            Self::Cliff => "Cliff",
            Self::Stalagmite => "Stalagmite",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoGenParams {
    pub mode: GeoMode,
    pub seed: u32,
    /// Spires (crystal / stalagmite), boulders (rock) or strata (cliff).
    pub count: u32,
    /// Max spire tilt from vertical, degrees.
    pub spread: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub base_radius: f32,
    pub tip_taper: f32,
    /// Planar cuts (rock, crystal base) / fracture cracks (cliff) / flutes (stalagmite).
    pub facets: u32,
    pub noise_strength: f32,
    pub noise_scale: f32,
    pub auto_smooth: bool,
    /// Crystal: small side shards branching off the spires.
    pub side_shards: bool,
    /// Crystal: flattened rock the spires grow from.
    pub cluster_base: bool,
    /// Stalagmite: hang from the ceiling (stalactite).
    pub hanging: bool,
}

impl GeoGenParams {
    pub fn default_of(mode: GeoMode) -> Self {
        let base = Self {
            mode,
            seed: 1,
            count: 7,
            spread: 35.0,
            min_height: 0.4,
            max_height: 1.2,
            base_radius: 0.5,
            tip_taper: 0.3,
            facets: 8,
            noise_strength: 0.25,
            noise_scale: 1.5,
            auto_smooth: true,
            side_shards: true,
            cluster_base: true,
            hanging: false,
        };
        match mode {
            GeoMode::Crystal => base,
            GeoMode::Rock => Self { count: 1, min_height: 0.6, max_height: 0.9, base_radius: 0.6, facets: 12, ..base },
            GeoMode::Cliff => {
                Self { count: 6, min_height: 1.2, max_height: 1.6, base_radius: 0.8, facets: 5, noise_strength: 0.15, ..base }
            }
            GeoMode::Stalagmite => {
                Self { count: 1, spread: 8.0, min_height: 1.0, max_height: 1.6, base_radius: 0.25, tip_taper: 0.5, facets: 7, ..base }
            }
        }
    }

    pub fn build(&self) -> EditablePoly {
        let mut m = match self.mode {
            GeoMode::Crystal => spires::crystal_cluster(self),
            GeoMode::Rock => rock::rock_pile(self),
            GeoMode::Cliff => cliff::cliff(self),
            GeoMode::Stalagmite => spires::stalagmites(self),
        };
        finish(&mut m, self.auto_smooth);
        m
    }
}

impl Default for GeoGenParams {
    fn default() -> Self {
        Self::default_of(GeoMode::Crystal)
    }
}

/// Auto Smooth threshold: low enough that shallow fracture planes stay crisp.
const SMOOTH_ANGLE: f32 = 20.0;

/// Facets stay sharp with Auto Smooth (rock ridges, crystal faces).
pub(crate) fn finish(m: &mut EditablePoly, auto_smooth: bool) {
    m.finalize();
    if m.signed_volume() < 0.0 {
        crate::splines::sweep::flip(m);
        m.finalize();
    }
    if auto_smooth {
        m.auto_smooth(&[], SMOOTH_ANGLE);
    } else {
        m.polygons.values_mut().for_each(|p| p.smoothing_group = 1);
    }
}

pub(crate) fn rng(seed: u32, salt: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed as u64 ^ (salt.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

/// Uniform in a..b (b may equal a).
pub(crate) fn range(r: &mut ChaCha8Rng, a: f32, b: f32) -> f32 {
    if b <= a { a } else { r.gen_range(a..b) }
}

/// Random unit vector (uniform on the sphere).
pub(crate) fn unit(r: &mut ChaCha8Rng) -> Vec3 {
    let z = range(r, -1.0, 1.0);
    let a = range(r, 0.0, std::f32::consts::TAU);
    let s = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(s * a.cos(), s * a.sin(), z)
}

/// Rotation tilting +Z by up to `spread` degrees towards a random azimuth.
pub(crate) fn tilt(r: &mut ChaCha8Rng, spread: f32) -> Quat {
    let az = range(r, 0.0, std::f32::consts::TAU);
    let t = range(r, 0.0, spread.to_radians());
    Quat::from_rotation_z(az) * Quat::from_rotation_x(t) * Quat::from_rotation_z(range(r, 0.0, std::f32::consts::TAU))
}

/// Seeded FBM + Worley field in roughly -1..1.
pub(crate) struct Field {
    fbm: Fbm<Perlin>,
    cells: Worley,
}

impl Field {
    pub fn new(seed: u32) -> Self {
        Self { fbm: Fbm::<Perlin>::new(seed).set_octaves(4), cells: Worley::new(seed.wrapping_add(17)) }
    }
    pub fn fbm(&self, p: Vec3) -> f32 {
        self.fbm.get(p.as_dvec3().to_array()) as f32
    }
    pub fn rocky(&self, p: Vec3) -> f32 {
        0.7 * self.fbm(p) + 0.3 * self.cells.get(p.as_dvec3().to_array()) as f32
    }
}

/// GeoGen as a stack modifier.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoGenMod {
    pub params: GeoGenParams,
    /// false: grow formations on polygons; true: deform the mesh like a rock.
    pub deform: bool,
    /// Grow mode: fraction of the candidate polygons that get a formation.
    pub density: f32,
    /// Grow mode: scale of each formation relative to its polygon.
    pub scale: f32,
}

impl Default for GeoGenMod {
    fn default() -> Self {
        let params = GeoGenParams { count: 3, cluster_base: false, ..GeoGenParams::default_of(GeoMode::Crystal) };
        Self { params, deform: false, density: 1.0, scale: 1.0 }
    }
}

/// Grow mode never places more formations than this.
pub const MAX_GROWN: usize = 96;

impl GeoGenMod {
    pub fn apply(&self, mesh: &mut EditablePoly) {
        if self.deform {
            deform_rock(mesh, &self.params);
            mesh.finalize();
            if self.params.auto_smooth {
                mesh.auto_smooth(&[], SMOOTH_ANGLE);
            }
            return;
        }
        let selected = mesh.selected_polygons();
        let targets = if selected.is_empty() { mesh.polygons.keys().collect() } else { selected };
        let mut r = rng(self.params.seed, 99);
        let chosen: Vec<(usize, _)> = targets.iter().copied().enumerate().filter(|_| range(&mut r, 0.0, 1.0) <= self.density).collect();
        // Past the cap, pick evenly across the candidates instead of the first N.
        let n = chosen.len().min(MAX_GROWN);
        let mut grown = EditablePoly::new();
        for j in 0..n {
            let (i, p) = chosen[j * chosen.len() / n];
            let (c, nrm) = (mesh.poly_centroid(p), mesh.poly_normal(p));
            let size = mesh.poly_area(p).sqrt().max(1e-3);
            let params = GeoGenParams { seed: self.params.seed.wrapping_add(i as u32 * 7919), ..self.params.clone() };
            let f = params.build();
            let k = self.scale * size / (2.0 * params.base_radius.max(1e-3));
            let spin = Quat::from_rotation_z(range(&mut r, 0.0, std::f32::consts::TAU));
            grown.append(&f, Mat4::from_scale_rotation_translation(Vec3::splat(k), Quat::from_rotation_arc(Vec3::Z, nrm) * spin, c));
        }
        mesh.append(&grown, Mat4::IDENTITY);
        mesh.finalize();
    }
}
