//! Shadow & Reflection Catcher: a surface that renders transparent in the
//! alpha channel except where other objects darken it (light shadows and
//! contact AO) or reflect in it, so a render drops onto any photo backplate.

use super::integrator::PathTracer;
use crate::render::lightmix::Groups;
use crate::render::trace::{Rng, Shading};
use bevy::math::Vec3;

fn lum(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// What the catcher adds at a camera hit.
pub struct Caught {
    /// Black shadow + AO opacity combined with the reflection's coverage.
    pub alpha: f32,
    /// Reflected light (premultiplied), per LightMix group.
    pub groups: Groups,
}

/// Fraction of the unshadowed light that shadows remove (0..1).
pub fn shadow_fraction(open: f32, lit: f32) -> f32 {
    if open <= 1e-8 { 0.0 } else { (1.0 - lit / open).clamp(0.0, 1.0) }
}

/// Combined catcher alpha: shadow and AO darken; the reflection's own
/// coverage `refl` covers the rest.
pub fn catcher_alpha(shadow: f32, ao_occlusion: f32, refl: f32) -> f32 {
    let dark = 1.0 - (1.0 - shadow) * (1.0 - ao_occlusion);
    (dark + refl * (1.0 - dark)).clamp(0.0, 1.0)
}

impl PathTracer<'_> {
    /// Shades a catcher hit seen from direction `d`; `ao` is the camera
    /// AO ray result (1 = open).
    pub(super) fn catch(&self, sh: &Shading, d: Vec3, ao: f32, nested: bool, rng: &mut Rng) -> Caught {
        let t = &self.t;
        let o = sh.p + sh.ng * t.eps;
        let (mut open, mut lit) = (0.0, 0.0);
        for l in &t.rs.lights {
            let Some((dir, dist, k)) = crate::render::volumetrics::incident(l, sh.p, rng) else { continue };
            let e = lum(l.radiance) * k * sh.n.dot(dir).max(0.0);
            open += e;
            lit += if t.cfg.shadows && l.shadows { e * t.visibility(o, dir, dist, l.softness, rng) } else { e };
        }
        let shadow = shadow_fraction(open, lit);
        let occl = (1.0 - ao) * t.cfg.gi.ao_strength.clamp(0.0, 1.0) * f32::from(t.cfg.gi.ao);
        let mut groups = Groups::default();
        let mut refl = 0.0;
        let r = d - 2.0 * d.dot(sh.n) * sh.n;
        if !nested && let Some(h) = t.rs.bvh.closest(o, r, f32::INFINITY) {
            let m = &t.rs.materials[t.rs.bvh.tris[h.tri].mat];
            if !m.shadow_catcher {
                let cos = sh.n.dot(-d).clamp(0.0, 1.0);
                let f = (0.04 + 0.96 * (1.0 - cos).powi(5)) * (1.0 - sh.s.roughness).powi(2);
                let s = self.radiance_from(o, r, true, rng);
                for (g, c) in groups.iter_mut().zip(s.groups) {
                    *g = c * f;
                }
                refl = f * s.alpha;
            }
        }
        Caught { alpha: catcher_alpha(shadow, occl, refl), groups }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catcher_alpha_is_clear_until_shadowed() {
        assert_eq!(catcher_alpha(0.0, 0.0, 0.0), 0.0, "open ground is fully transparent");
        assert_eq!(catcher_alpha(1.0, 0.0, 0.0), 1.0, "full shadow is opaque black");
        assert!((catcher_alpha(0.5, 0.0, 0.0) - 0.5).abs() < 1e-6);
        assert!((catcher_alpha(0.0, 0.5, 0.0) - 0.5).abs() < 1e-6, "contact AO");
        assert!((catcher_alpha(0.5, 0.0, 0.5) - 0.75).abs() < 1e-6, "reflection covers the rest");
        assert_eq!(shadow_fraction(0.0, 0.0), 0.0, "no lights, no shadow");
        assert!((shadow_fraction(2.0, 0.5) - 0.75).abs() < 1e-6);
    }
}
