//! Random-walk subsurface scattering (as in Cycles / SolidCraft): light
//! that enters an SSS surface takes exponential free-flight steps with
//! isotropic scattering, losing energy by the van de Hulst albedo of the
//! SSS Scatter Color, until it leaves through any surface — so thin ears,
//! noses, leaves and wax glow from behind. Also Beer-Lambert absorption.

use super::integrator::PathTracer;
use crate::materials::Material;
use crate::materials::principled::Principled;
use crate::materials::shading::lin3;
use crate::render::trace::{Rng, Shading};
use bevy::math::Vec3;

/// Most scattering events before a walk is given up (energy lost).
const MAX_STEPS: u32 = 256;

/// Uniform direction on the unit sphere.
fn sphere_dir(rng: &mut Rng) -> Vec3 {
    let z = 1.0 - 2.0 * rng.f();
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = std::f32::consts::TAU * rng.f();
    Vec3::new(r * phi.cos(), r * phi.sin(), z)
}

/// Beer-Lambert transmittance over `dist` inside a transmissive material.
pub fn absorption(p: &Principled, dist: f32) -> Vec3 {
    if p.absorption_density <= 0.0 {
        return Vec3::ONE;
    }
    let c = lin3(p.absorption_color).max(Vec3::splat(1e-3));
    Vec3::new(c.x.ln(), c.y.ln(), c.z.ln()).mul_add(Vec3::splat(p.absorption_density * dist), Vec3::ZERO).exp()
}

impl PathTracer<'_> {
    /// Walks from entry point `sh` into the material; returns the exit
    /// point (normals facing out) and the walk's throughput.
    pub(super) fn random_walk(&self, m: &Material, sh: &Shading, rng: &mut Rng) -> Option<(Shading, Vec3)> {
        let p = &m.principled;
        let sigma = 1.0 / p.mean_free_path();
        let col = lin3(p.sss_color);
        let albedo = Vec3::new(Principled::walk_albedo(col.x), Principled::walk_albedo(col.y), Principled::walk_albedo(col.z));
        let mut o = sh.p - sh.ng * self.t.eps;
        // Enter with a cosine lobe around the inward normal.
        let mut d = crate::render::trace::cosine_dir(-sh.n, rng);
        let mut w = Vec3::ONE;
        for _ in 0..MAX_STEPS {
            let t = -(1.0 - rng.f()).max(1e-12).ln() / sigma;
            if let Some(h) = self.t.rs.bvh.closest(o, d, t) {
                let mut ex = self.t.interp(&h, o, d);
                (ex.n, ex.ng, ex.entering) = (-ex.n, -ex.ng, false);
                return Some((ex, w));
            }
            o += d * t;
            w *= albedo;
            // Russian roulette keeps long walks cheap and unbiased.
            let survive = w.max_element().clamp(0.0, 0.98);
            if rng.f() >= survive {
                return None;
            }
            w /= survive;
            d = sphere_dir(rng);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};
    use crate::render::RenderSettings;
    use crate::render::scene_build::RenderScene;
    use crate::render::trace::Tracer;
    use crate::scene::{BaseObject, Scene};
    use crate::viewport::camera::{ViewCamera, ViewKind};

    /// Share of walks entering a slab from above that leave through its
    /// underside (light shining through), for slab height scale `sz`.
    fn through(sz: f32) -> f32 {
        let mut s = Scene::default();
        let id = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::ZERO);
        let mut m = Material::named("Wax");
        (m.principled.sss_weight, m.principled.sss_radius, m.principled.sss_scale, m.principled.sss_color) = (1.0, [1.0; 3], 0.1, [1.0; 3]);
        let mid = s.materials.mats.insert(m);
        s.objects[id].material = Some(mid);
        s.objects[id].transform.scale = Vec3::new(20.0, 20.0, sz);
        s.objects[id].evaluate();
        let rs = RenderScene::build(&s, ViewCamera::new(ViewKind::Perspective), false);
        let (lo, hi) = rs.bvh.tris.iter().flat_map(|t| t.p).fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.z), b.max(p.z)));
        let cfg = RenderSettings::default();
        let ps = crate::render::PathSettings::default();
        let pt = PathTracer::new(Tracer::new(&rs, &cfg), &ps);
        let (o, d) = (Vec3::new(0.0, 0.0, hi + 1.0), Vec3::NEG_Z);
        let h = rs.bvh.closest(o, d, f32::INFINITY).unwrap();
        let sh = pt.t.interp(&h, o, d);
        let mut rng = Rng::new(9);
        let n = 4000;
        let below = (0..n).filter_map(|_| pt.random_walk(&rs.materials[0], &sh, &mut rng)).filter(|(ex, _)| ex.p.z < (lo + hi) * 0.5).count();
        below as f32 / n as f32
    }

    #[test]
    fn random_walk_shines_through_thin_slabs_only() {
        let (thin, thick) = (through(0.005), through(1.0));
        assert!(thin > 0.3, "a thin slab passes light through: {thin}");
        assert!(thick < thin * 0.25, "a thick slab mostly returns it: {thick} vs {thin}");
    }

    #[test]
    fn walk_albedo_inverts_the_scatter_color() {
        assert!(Principled::walk_albedo(0.0) < 0.05 && Principled::walk_albedo(0.999) > 0.99);
        assert!(Principled::walk_albedo(0.5) > 0.8, "multi-scatter needs a high single-scatter albedo");
    }
}
