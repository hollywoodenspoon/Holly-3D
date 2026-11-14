//! Volume objects inside the two CPU renderers: shadow transmittance,
//! next-event estimation at a scatter point (with the Cloud Bounce Boost
//! octaves), phase sampling and the raytracer's single-scatter estimate.

use super::{Collision, VolumeObj};
use crate::materials::volume::VolumeMat;
use crate::render::trace::{Rng, Tracer};
use crate::render::volumetrics::{hg_phase, incident, sample_hg};
use bevy::math::Vec3;

/// Boost octaves k = 1, 2: weight a^k, phase g * b^k, transmittance T^(c^k)
/// with a = b = c = 1/2 (thick clouds look brighter and softer).
const OCTAVES: [(f32, f32, f32); 2] = [(0.5, 0.5, 0.5), (0.25, 0.25, 0.25)];

fn phase_scaled(m: &VolumeMat, cos: f32, s: f32) -> f32 {
    let b = m.blend.clamp(0.0, 1.0);
    hg_phase(cos, m.g_forward * s) * (1.0 - b) + hg_phase(cos, m.g_back * s) * b
}

/// Samples the next direction from the two-lobe phase (weight 1).
pub fn sample_phase(m: &VolumeMat, d: Vec3, rng: &mut Rng) -> Vec3 {
    let g = if rng.f() < m.blend.clamp(0.0, 1.0) { m.g_back } else { m.g_forward };
    sample_hg(d, g, rng.f(), rng.f())
}

fn seed(p: Vec3, d: Vec3) -> u64 {
    let b = |v: f32| v.to_bits() as u64;
    b(p.x) ^ b(p.y).rotate_left(21) ^ b(p.z).rotate_left(42) ^ b(d.x).rotate_left(7) ^ b(d.y).rotate_left(29) ^ b(d.z).rotate_left(51)
}

impl Tracer<'_> {
    /// Light passing along a shadow ray: see-through surfaces and volumes.
    pub(crate) fn transmittance(&self, p: Vec3, dir: Vec3, tmax: f32) -> f32 {
        let s = self.surface_transmittance(p, dir, tmax);
        if s <= 0.0 || self.rs.volumes.is_empty() {
            return s;
        }
        s * self.volume_transmittance(p, dir, tmax, &mut Rng::new(seed(p, dir)))
    }

    fn volume_transmittance(&self, p: Vec3, dir: Vec3, tmax: f32, rng: &mut Rng) -> f32 {
        self.rs.volumes.transmittance(p, dir, tmax, self.cfg.volumes.shadow_quality.estimates(), rng)
    }

    /// Light scattered toward `-d` at `p` inside `v`, per LightMix group
    /// (before the albedo): every light, shadowed, times the phase.
    pub(crate) fn volume_nee(&self, v: &VolumeObj, p: Vec3, d: Vec3, rng: &mut Rng, mut f: impl FnMut(u8, Vec3)) {
        let boost = self.cfg.volumes.boost.clamp(0.0, 1.0);
        for l in &self.rs.lights {
            let Some((dir, dist, k)) = incident(l, p, rng) else { continue };
            let cos = d.dot(dir);
            let len = (dist - self.eps).min(1e30);
            let (surf, tv) = if self.cfg.shadows && l.shadows {
                let s = self.surface_transmittance(p, dir, len);
                (s, if s > 0.0 { self.volume_transmittance(p, dir, len, rng) } else { 0.0 })
            } else {
                (1.0, 1.0)
            };
            if surf <= 0.0 {
                continue;
            }
            let mut pv = v.mat.phase(cos) * tv;
            if boost > 0.0 {
                pv += boost * OCTAVES.iter().map(|&(a, b, c)| a * phase_scaled(&v.mat, cos, b) * tv.powf(c)).sum::<f32>();
            }
            if pv > 0.0 {
                f(l.group, l.radiance * (k * surf * pv));
            }
        }
    }

    /// Raytracer: a short random walk through the volumes from the first
    /// collision before `tmax` (`None` when the ray passes every volume):
    /// light at each scatter, then sky / environment where the walk leaves.
    pub(crate) fn volume_single(&self, o: Vec3, d: Vec3, tmax: f32, rng: &mut Rng) -> Option<Vec3> {
        let c: Collision = self.rs.volumes.collide(o, d, tmax, rng)?;
        let max = self.cfg.volumes.max_bounces.clamp(1, 64);
        let (mut l, mut beta, mut c, mut p, mut d) = (Vec3::ZERO, Vec3::ONE, c, o + d * c.t, d);
        for bounce in 0..max {
            let v = &self.rs.volumes.list[c.vol];
            l += beta * v.mat.glow(c.density);
            beta *= v.mat.albedo();
            self.volume_nee(v, p, d, rng, |_, x| l += beta * x);
            if bounce > 2 {
                let survive = beta.max_element().clamp(0.0, 0.95);
                if rng.f() >= survive {
                    break;
                }
                beta /= survive;
            }
            d = sample_phase(&v.mat, d, rng);
            match self.rs.volumes.collide(p, d, self.rs.radius.max(1.0) * 1e3, rng) {
                Some(n) => (c, p) = (n, p + d * n.t),
                None => {
                    l += beta * (self.environment(d) + self.rs.ambient);
                    break;
                }
            }
        }
        Some(l)
    }
}
