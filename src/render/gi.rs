//! F10 GI & AO for the built-in raytracer: next-event estimation on emissive
//! polygons (area lights), ambient occlusion, 1-bounce cosine-weighted
//! indirect diffuse (color bleed) and the procedural studio softbox dome
//! seen by reflection rays when no HDRI is loaded.

use super::emitters::point_on;
use super::trace::{Rng, Shading, Tracer, cosine_dir};
use crate::materials::ShadingModel;
use crate::materials::shading::{LightIn, shade};
use bevy::math::Vec3;

/// A rectangular softbox: facing direction, its two in-plane axes, half
/// extents (tangent of the half angle) and brightness.
struct Panel {
    axis: Vec3,
    t: Vec3,
    b: Vec3,
    half: (f32, f32),
    power: f32,
}

fn panels() -> [Panel; 3] {
    let key = Vec3::new(0.25, -0.35, 1.0).normalize();
    let kt = Vec3::X;
    let kb = key.cross(kt).normalize();
    let strip = |side: f32| {
        let axis = Vec3::new(side, 0.3, 0.25).normalize();
        let t = Vec3::Z.cross(axis).normalize();
        Panel { axis, t, b: axis.cross(t).normalize(), half: (0.18, 0.75), power: 1.6 }
    };
    [Panel { axis: key, t: kt, b: kb, half: (0.55, 0.4), power: 3.0 }, strip(1.0), strip(-1.0)]
}

/// Studio softbox radiance in direction `d` (Z up).
pub fn softbox(d: Vec3) -> Vec3 {
    let mut c = 0.0;
    for p in panels() {
        let k = d.dot(p.axis);
        if k <= 0.05 {
            continue;
        }
        let (x, y) = ((d.dot(p.t) / k).abs() / p.half.0, (d.dot(p.b) / k).abs() / p.half.1);
        let edge = 1.0 - x.max(y);
        c += p.power * (edge / 0.12).clamp(0.0, 1.0);
    }
    Vec3::new(1.0, 0.98, 0.95) * c
}

/// Smallest squared distance to an emitter sample (stops the 1/d² blow-up
/// for points right next to an emissive panel).
pub const MIN_DIST_SQ: f32 = 0.05;
/// Firefly clamp: one NEE sample may carry at most this many times the
/// emitter's radiance (before the 1/n average).
pub const FIREFLY_CAP: f32 = 4.0;

/// Radiance carried by one emitter NEE sample: the area-light estimator
/// `L cos_l A / (d² p n)` with d² clamped to [`MIN_DIST_SQ`], the solid
/// angle bounded by a hemisphere (2π) and a luminance firefly clamp, so low
/// Emitter Samples give smooth falloff instead of sparkle near floors.
pub fn nee_radiance(radiance: Vec3, cos_l: f32, area: f32, d2: f32, prob: f32, n: u32) -> Vec3 {
    let solid_angle = (cos_l * area / d2.max(MIN_DIST_SQ)).min(std::f32::consts::TAU);
    let w = solid_angle / (prob.max(1e-6) * n.max(1) as f32);
    let cap = FIREFLY_CAP * std::f32::consts::TAU / n.max(1) as f32;
    radiance * w.min(cap)
}

impl Tracer<'_> {
    /// Next-event estimation: one [`LightIn`] per sample on an emissive
    /// polygon, with its unbiased area-light weight.
    pub(super) fn emitter_lights(&self, sh: &Shading, rng: &mut Rng, out: &mut Vec<LightIn>) {
        let em = &self.rs.emitters;
        let n = self.cfg.gi.emitter_samples;
        if em.is_empty() || n == 0 {
            return;
        }
        let o = sh.p + sh.ng * self.eps;
        for _ in 0..n {
            let Some((e, prob)) = em.pick(rng) else { return };
            if e.tri == sh.tri {
                continue;
            }
            let t = &self.rs.bvh.tris[e.tri];
            let q = point_on(&t.p, rng);
            let v = q - o;
            let (dist, dir) = (v.length().max(1e-4), v.normalize_or_zero());
            let nl = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
            let cos_l = nl.dot(-dir).abs();
            if sh.n.dot(dir) <= 0.0 || cos_l <= 1e-4 {
                continue;
            }
            if self.cfg.shadows && self.rs.bvh.occluded(o, dir, dist - self.eps * 4.0) {
                continue;
            }
            out.push(LightIn { dir, radiance: nee_radiance(e.radiance, cos_l, e.area, dist * dist, prob, n) });
        }
    }

    /// Ambient occlusion factor (1 = open, `1 - strength` = fully occluded).
    pub(super) fn ao(&self, sh: &Shading, rng: &mut Rng) -> f32 {
        let gi = &self.cfg.gi;
        if !gi.ao {
            return 1.0;
        }
        let n = gi.ao_samples.clamp(1, 64);
        let o = sh.p + sh.ng * self.eps;
        // Cut-out holes and see-through cards let the ambient light in.
        let hit: f32 = (0..n).map(|_| 1.0 - self.transmittance(o, cosine_dir(sh.n, rng), gi.ao_radius.max(1e-4))).sum();
        1.0 - gi.ao_strength.clamp(0.0, 1.0) * hit / n as f32
    }

    /// 1-bounce indirect diffuse irradiance (times albedo by the caller):
    /// surfaces hit by cosine-weighted rays, lit by the scene lights and
    /// emitters only (their own glow is already counted by NEE).
    pub(super) fn indirect(&self, sh: &Shading, rng: &mut Rng) -> Vec3 {
        let gi = &self.cfg.gi;
        if !gi.gi {
            return Vec3::ZERO;
        }
        let n = gi.gi_samples.clamp(1, 256);
        let o = sh.p + sh.ng * self.eps;
        let mut sum = Vec3::ZERO;
        for _ in 0..n {
            let d = cosine_dir(sh.n, rng);
            let Some(h) = self.rs.bvh.closest(o, d, f32::INFINITY) else { continue };
            let mut s2 = self.interp(&h, o, d);
            let m = &self.rs.materials[self.rs.bvh.tris[h.tri].mat];
            s2.s.emissive = Vec3::ZERO;
            sum += if m.model == ShadingModel::Unlit {
                s2.s.base
            } else {
                let lights = self.direct(&s2, rng);
                shade(m, &s2.s, s2.n, -d, &lights, Vec3::ZERO)
            };
        }
        sum / n as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nee_clamps_near_emitters_and_fireflies() {
        let l = Vec3::ONE;
        // Far away: plain inverse-square estimator.
        let far = nee_radiance(l, 1.0, 1.0, 100.0, 1.0, 1);
        assert!((far.x - 0.01).abs() < 1e-6);
        // Touching the panel: bounded, not ~1e6.
        let near = nee_radiance(l, 1.0, 1.0, 1e-6, 1.0, 1);
        assert_eq!(near, nee_radiance(l, 1.0, 1.0, MIN_DIST_SQ, 1.0, 1).min(near));
        assert!(near.x <= std::f32::consts::TAU + 1e-4, "solid angle <= 2π: {near}");
        // A rarely picked tiny emitter can't spike beyond the firefly cap.
        let rare = nee_radiance(l, 1.0, 0.1, 0.06, 1e-4, 4);
        assert!(rare.x <= FIREFLY_CAP * std::f32::consts::TAU / 4.0 + 1e-4, "{rare}");
    }

    #[test]
    fn softbox_has_bright_panels_and_dark_gaps() {
        assert!(softbox(Vec3::new(0.25, -0.35, 1.0).normalize()).x > 2.5, "key panel overhead");
        assert_eq!(softbox(-Vec3::Z), Vec3::ZERO, "nothing below");
        assert_eq!(softbox(Vec3::new(0.0, -1.0, 0.05).normalize()), Vec3::ZERO, "gap between panels");
        assert!(softbox(Vec3::new(1.0, 0.3, 0.25).normalize()).x > 1.0, "side strip");
    }
}
