//! Emissive polygons as area lights: every triangle whose material has an
//! Emission Multiplier above 0 joins a power-weighted table that next-event
//! estimation samples (a point on a triangle chosen by its emitted power).

use super::bvh::Bvh;
use super::trace::Rng;
use crate::materials::Material;
use crate::materials::shading::surface;
use bevy::math::{Vec2, Vec3, Vec4};

#[derive(Clone, Copy, Debug)]
pub struct Emitter {
    /// Triangle index in the BVH.
    pub tri: usize,
    pub area: f32,
    /// Emitted linear radiance.
    pub radiance: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct Emitters {
    pub list: Vec<Emitter>,
    /// Running sum of emitted power (radiance x area), for sampling.
    cdf: Vec<f32>,
}

impl Emitters {
    pub fn build(bvh: &Bvh, materials: &[Material]) -> Self {
        let mut e = Self::default();
        for (i, t) in bvh.tris.iter().enumerate() {
            let m = &materials[t.mat];
            if m.emissive_strength <= 0.0 {
                continue;
            }
            let area = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).length() * 0.5;
            let uv: Vec2 = (t.uv[0] + t.uv[1] + t.uv[2]) / 3.0;
            let radiance = surface(m, uv, (t.p[0] + t.p[1] + t.p[2]) / 3.0, Vec4::ONE).emissive;
            let power = radiance.max_element() * area;
            if power > 1e-8 {
                e.list.push(Emitter { tri: i, area, radiance });
                e.cdf.push(e.cdf.last().copied().unwrap_or(0.0) + power);
            }
        }
        e
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn total_power(&self) -> f32 {
        self.cdf.last().copied().unwrap_or(0.0)
    }

    /// Probability [`Emitters::pick`] chooses triangle `tri` (0 if it does
    /// not emit): the light-sampling density MIS weighs BSDF hits against.
    pub fn prob_of(&self, tri: usize) -> f32 {
        self.lookup(tri).map_or(0.0, |(_, p)| p)
    }

    /// The emitter on triangle `tri` and its pick probability.
    pub fn lookup(&self, tri: usize) -> Option<(&Emitter, f32)> {
        let i = self.list.binary_search_by_key(&tri, |e| e.tri).ok()?;
        let prev = if i == 0 { 0.0 } else { self.cdf[i - 1] };
        Some((&self.list[i], (self.cdf[i] - prev) / self.total_power().max(1e-12)))
    }

    /// Picks an emitter in proportion to its power: (emitter, probability).
    pub fn pick(&self, rng: &mut Rng) -> Option<(&Emitter, f32)> {
        let total = self.total_power();
        if total <= 0.0 {
            return None;
        }
        let x = rng.f() * total;
        let i = self.cdf.partition_point(|&c| c <= x).min(self.list.len() - 1);
        let prev = if i == 0 { 0.0 } else { self.cdf[i - 1] };
        Some((&self.list[i], (self.cdf[i] - prev) / total))
    }
}

/// Uniform random point on a triangle.
pub fn point_on(p: &[Vec3; 3], rng: &mut Rng) -> Vec3 {
    let (mut a, mut b) = (rng.f(), rng.f());
    if a + b > 1.0 {
        (a, b) = (1.0 - a, 1.0 - b);
    }
    p[0] + (p[1] - p[0]) * a + (p[2] - p[0]) * b
}
