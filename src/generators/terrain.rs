//! Procedural Terrain (Create -> Geometry -> Worlds -> Terrain, and the
//! Terrain modifier): six biome presets built from Perlin FBM, Ridged
//! Multifractal and Worley noise, terrace quantization, thermal / hydraulic
//! erosion, and auto Material IDs by slope and altitude.

use super::erosion::Heightmap;
use super::terrain_ids::TerrainIds;
use super::{grid, smooth};
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, RidgedMulti, Worley};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Biome {
    Plains,
    #[default]
    RollingHills,
    JaggedMountains,
    Mesa,
    Savannah,
    Dunes,
}

impl Biome {
    pub const ALL: [Biome; 6] = [Self::Plains, Self::RollingHills, Self::JaggedMountains, Self::Mesa, Self::Savannah, Self::Dunes];
    pub fn label(self) -> &'static str {
        match self {
            Self::Plains => "Plains",
            Self::RollingHills => "Rolling Hills",
            Self::JaggedMountains => "Jagged Mountains",
            Self::Mesa => "Mesa / Plateau Canyon",
            Self::Savannah => "Savannah",
            Self::Dunes => "Desert Sand Dunes",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TerrainParams {
    pub biome: Biome,
    pub width: f32,
    pub length: f32,
    pub segments: u32,
    pub seed: u32,
    pub max_elevation: f32,
    /// Noise octaves (1..=10).
    pub octaves: u32,
    /// Octave persistence (0.2..=0.8): higher = rougher.
    pub roughness: f32,
    /// Plateau terrace steps (0 = off).
    pub terraces: u32,
    /// Hydraulic + thermal erosion (0..1).
    pub erosion: f32,
    /// Dune wind direction (degrees about Z).
    pub wind_angle: f32,
    pub ids: TerrainIds,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self::of(Biome::RollingHills)
    }
}

impl TerrainParams {
    /// Biome defaults (Biome dropdown resets the shape parameters).
    pub fn of(biome: Biome) -> Self {
        let base = Self {
            biome,
            width: 200.0,
            length: 200.0,
            segments: 96,
            seed: 1,
            max_elevation: 25.0,
            octaves: 5,
            roughness: 0.5,
            terraces: 0,
            erosion: 0.0,
            wind_angle: 30.0,
            ids: TerrainIds::of(biome),
        };
        match biome {
            Biome::Plains => Self { max_elevation: 6.0, octaves: 4, roughness: 0.4, ..base },
            Biome::RollingHills => base,
            Biome::JaggedMountains => Self { max_elevation: 70.0, octaves: 7, roughness: 0.55, erosion: 0.6, segments: 128, ..base },
            Biome::Mesa => Self { max_elevation: 40.0, octaves: 5, roughness: 0.45, terraces: 5, erosion: 0.15, ..base },
            Biome::Savannah => Self { max_elevation: 8.0, octaves: 4, roughness: 0.45, ..base },
            Biome::Dunes => Self { max_elevation: 12.0, octaves: 3, roughness: 0.35, segments: 128, ..base },
        }
    }

    /// Normalized heights (about 0..1) for u, v in 0..1, after erosion.
    pub fn heightmap(&self) -> Heightmap {
        let n = self.segments.clamp(2, 512) as usize + 1;
        let f = Noises::new(self);
        let aspect = Vec2::new(self.width.max(1e-3), self.length.max(1e-3));
        let scale = aspect / aspect.max_element();
        let mut hm = Heightmap::from_fn(n, |u, v| f.height(Vec2::new(u - 0.5, v - 0.5) * scale * 4.0, self));
        hm.erode(self.seed, self.erosion);
        hm
    }

    pub fn build(&self) -> EditablePoly {
        let hm = self.heightmap();
        let segs = (hm.n - 1) as u32;
        let (w, l, e) = (self.width.max(1e-3), self.length.max(1e-3), self.max_elevation);
        let (mut m, _) = grid(w, l, segs, |x, y| hm.sample(x / w + 0.5, y / l + 0.5) * e);
        m.finalize();
        finish(&mut m, &self.ids, self.biome);
        m
    }
}

/// Smoothing and Material IDs for a finished terrain mesh.
pub(crate) fn finish(m: &mut EditablePoly, ids: &TerrainIds, biome: Biome) {
    if biome == Biome::Mesa {
        m.auto_smooth(&[], 50.0);
    } else {
        m.polygons.values_mut().for_each(|p| p.smoothing_group = 1);
    }
    if ids.enabled {
        ids.assign(m);
    }
}

struct Noises {
    fbm: Fbm<Perlin>,
    ridged: RidgedMulti<Perlin>,
    warp: Fbm<Perlin>,
    cells: Worley,
}

fn get2(n: &impl NoiseFn<f64, 2>, p: Vec2) -> f32 {
    n.get(p.as_dvec2().to_array()) as f32
}

impl Noises {
    fn new(t: &TerrainParams) -> Self {
        let oct = t.octaves.clamp(1, 10) as usize;
        let pers = t.roughness.clamp(0.1, 0.9) as f64;
        Self {
            fbm: Fbm::<Perlin>::new(t.seed).set_octaves(oct).set_persistence(pers),
            ridged: RidgedMulti::<Perlin>::new(t.seed.wrapping_add(3)).set_octaves(oct),
            warp: Fbm::<Perlin>::new(t.seed.wrapping_add(7)).set_octaves(3),
            cells: Worley::new(t.seed.wrapping_add(11)),
        }
    }

    fn fbm01(&self, p: Vec2) -> f32 {
        (get2(&self.fbm, p) * 0.5 + 0.5).clamp(0.0, 1.0)
    }

    /// Height in about 0..1 at plane point `p` (map spans about 4 units).
    fn height(&self, p: Vec2, t: &TerrainParams) -> f32 {
        let h = match t.biome {
            Biome::Plains => 0.15 + 0.5 * self.fbm01(p * 0.6) * 0.6,
            Biome::RollingHills => smooth(0.05, 0.95, self.fbm01(p * 0.7)),
            Biome::JaggedMountains => {
                let r = (get2(&self.ridged, p * 0.55) * 0.5 + 0.5).clamp(0.0, 1.0);
                let mass = self.fbm01(p * 0.3);
                (r.powf(1.6) * (0.35 + 0.65 * mass)).clamp(0.0, 1.0)
            }
            Biome::Mesa => {
                let base = smooth(0.3, 0.7, self.fbm01(p * 0.45));
                // Winding canyon: a narrow band where a warp field crosses 0.
                let river = get2(&self.warp, p * 0.5).abs();
                base * (0.25 + 0.75 * smooth(0.02, 0.12, river))
            }
            Biome::Savannah => {
                let flat = 0.3 + 0.08 * get2(&self.fbm, p * 0.8);
                let mound = smooth(0.82, 1.0, 1.0 - get2(&self.cells, p * 2.5).abs()) * 0.18;
                let channel = 1.0 - smooth(0.0, 0.05, get2(&self.warp, p * 0.6).abs());
                flat + mound - channel * 0.22
            }
            Biome::Dunes => self.dune(p, t.wind_angle),
        };
        if t.terraces > 0 { terrace(h, t.terraces) } else { h }
    }

    /// Crescent dunes: an asymmetric sawtooth across the wind (gentle
    /// windward rise, steep leeward slip face), sheared by Worley cells.
    fn dune(&self, p: Vec2, wind: f32) -> f32 {
        let a = wind.to_radians();
        let (dir, side) = (Vec2::new(a.cos(), a.sin()), Vec2::new(-a.sin(), a.cos()));
        let (s, c) = (p.dot(dir), p.dot(side));
        let cell = get2(&self.cells, Vec2::new(s * 0.6, c * 1.6));
        let phase = (s * 1.6 + 0.35 * cell + 0.25 * get2(&self.warp, p * 0.7) + 0.4 * (c * 0.9).sin().powi(2)).rem_euclid(1.0);
        let profile = if phase < 0.78 { smooth(0.0, 0.78, phase) } else { 1.0 - smooth(0.78, 1.0, phase) };
        let amp = 0.55 + 0.45 * self.fbm01(p * 0.35);
        0.1 + profile * amp * 0.85
    }
}

/// Stepped terraces with steep risers (stratified canyon walls).
pub fn terrace(h: f32, steps: u32) -> f32 {
    let k = h.clamp(0.0, 1.0) * steps as f32;
    let (fl, fr) = (k.floor(), k.fract());
    (fl + smooth(0.75, 1.0, fr)) / steps as f32
}

/// Terrain as a stack modifier: displaces the input mesh along local Z by
/// the terrain height across its XY bounds, then assigns Material IDs.
#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TerrainMod {
    pub params: TerrainParams,
}

impl TerrainMod {
    pub fn apply(&self, mesh: &mut EditablePoly) {
        let Some(b) = mesh.bounds() else { return };
        let size = (b.max - b.min).truncate().max(Vec2::splat(1e-3));
        let p = TerrainParams { width: size.x, length: size.y, ..self.params.clone() };
        let hm = p.heightmap();
        for v in mesh.vertices.values_mut() {
            let uv = (v.position.truncate() - b.min.truncate()) / size;
            v.position += Vec3::Z * hm.sample(uv.x, uv.y) * p.max_elevation;
        }
        mesh.finalize();
        finish(mesh, &p.ids, p.biome);
    }
}
