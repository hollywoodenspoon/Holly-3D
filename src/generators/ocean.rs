//! Gerstner (trochoidal) Ocean (Create -> Geometry -> Worlds -> Ocean, and
//! the Ocean modifier): a spectrum of six waves around the wind direction.
//! Choppiness pulls vertices toward each crest, so crests are peaked and
//! troughs wide. `[✓] Auto-Crest Foam` whitens high, pinched crests in
//! `Vertex.color` and gives those polygons Material ID 2.

use super::{grid, rgba, smooth};
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};
use std::f32::consts::TAU;

pub const ID_WATER: u16 = 1;
pub const ID_FOAM: u16 = 2;
/// Deep water vertex tint (sRGB).
pub const WATER_TINT: Vec3 = Vec3::new(0.05, 0.24, 0.33);

/// (wavelength ratio, angle offset from the wind in radians, phase).
const SPECTRUM: [(f32, f32, f32); 6] =
    [(1.0, 0.0, 0.0), (0.61, 0.38, 1.7), (0.43, -0.42, 4.1), (0.29, 0.71, 2.6), (0.19, -0.66, 5.3), (0.13, 0.2, 0.9)];

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OceanParams {
    pub width: f32,
    pub length: f32,
    pub segments: u32,
    /// Crest-to-trough height of the main wave.
    pub wave_height: f32,
    /// 0 = round sine swell, 1 = sharp trochoid crests.
    pub choppiness: f32,
    /// Main wavelength (scene units).
    pub wavelength: f32,
    /// Wind direction (degrees about Z, 0 = +X).
    pub wind_angle: f32,
    /// Phase / time (seconds).
    pub time: f32,
    pub auto_foam: bool,
    /// Foam starts at this fraction of the crest height (0..1).
    pub foam_threshold: f32,
}

impl Default for OceanParams {
    fn default() -> Self {
        Self {
            width: 200.0,
            length: 200.0,
            segments: 128,
            wave_height: 2.5,
            choppiness: 0.7,
            wavelength: 40.0,
            wind_angle: 0.0,
            time: 0.0,
            auto_foam: true,
            foam_threshold: 0.6,
        }
    }
}

/// One evaluated point: displaced position offset and foam amount 0..1.
#[derive(Clone, Copy, Debug)]
pub struct WaveSample {
    pub offset: Vec3,
    pub foam: f32,
}

impl OceanParams {
    fn waves(&self) -> impl Iterator<Item = (Vec2, f32, f32, f32, f32)> + '_ {
        let base = self.wavelength.max(0.1);
        let wind = self.wind_angle.to_radians();
        SPECTRUM.iter().map(move |&(ratio, da, ph)| {
            let lambda = base * ratio;
            let k = TAU / lambda;
            // Amplitude scales with wavelength (constant steepness): the
            // main wave rises and falls by half the Wave Height.
            let a = self.wave_height.max(0.0) * 0.5 * ratio;
            let omega = (9.81 * k).sqrt();
            (Vec2::new((wind + da).cos(), (wind + da).sin()), k, a, omega, ph)
        })
    }

    /// Largest possible crest height (sum of amplitudes).
    pub fn crest(&self) -> f32 {
        self.waves().map(|w| w.2).sum::<f32>().max(1e-6)
    }

    /// Gerstner displacement and foam at rest position `p` (XY).
    pub fn sample(&self, p: Vec2) -> WaveSample {
        let n = SPECTRUM.len() as f32;
        let (mut off, mut jxx, mut jyy, mut jxy) = (Vec3::ZERO, 1.0, 1.0, 0.0);
        for (d, k, a, w, ph) in self.waves() {
            // Per-wave steepness so the summed choppiness never loops.
            let q = self.choppiness.clamp(0.0, 1.0) / (k * a * n).max(1e-6);
            let th = k * d.dot(p) - w * self.time + ph;
            let (s, c) = th.sin_cos();
            off += Vec3::new(q * a * d.x * c, q * a * d.y * c, a * s);
            jxx -= q * a * k * d.x * d.x * s;
            jyy -= q * a * k * d.y * d.y * s;
            jxy -= q * a * k * d.x * d.y * s;
        }
        let height = off.z / self.crest();
        let pinch = 1.0 - (jxx * jyy - jxy * jxy);
        let th = self.foam_threshold.clamp(0.0, 0.99);
        let foam = smooth(th, th + 0.3, height).max(smooth(0.6, 0.95, pinch) * smooth(th * 0.5, th, height));
        WaveSample { offset: off, foam }
    }

    pub fn build(&self) -> EditablePoly {
        let (w, l) = (self.width.max(1e-3), self.length.max(1e-3));
        let (mut m, _) = grid(w, l, self.segments.clamp(1, 512), |_, _| 0.0);
        displace(&mut m, self);
        m
    }
}

/// Displaces every vertex from its rest XY, writes foam into the vertex
/// colors and sets Material IDs (water 1, foam 2).
pub(crate) fn displace(m: &mut EditablePoly, o: &OceanParams) {
    let mut foam = std::collections::HashMap::new();
    for (id, v) in m.vertices.iter_mut() {
        let s = o.sample(v.position.truncate());
        v.position += s.offset;
        let f = if o.auto_foam { s.foam } else { 0.0 };
        v.color = rgba(WATER_TINT.lerp(Vec3::ONE, f), 1.0);
        foam.insert(id, f);
    }
    for p in m.polygons.values_mut() {
        let avg = p.corners.iter().map(|c| foam[&c.vertex_id]).sum::<f32>() / p.corners.len() as f32;
        p.material_id = if o.auto_foam && avg > 0.5 { ID_FOAM } else { ID_WATER };
        p.smoothing_group = 1;
    }
    m.finalize();
}

/// Ocean as a stack modifier: Gerstner waves on any mesh (local XY = rest
/// position, waves rise along local Z).
#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OceanMod {
    pub params: OceanParams,
}

impl OceanMod {
    pub fn apply(&self, mesh: &mut EditablePoly) {
        displace(mesh, &self.params);
    }
}
