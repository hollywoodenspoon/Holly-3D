//! Path-tracer BSDF: Lambert diffuse + GGX microfacet specular (visible-
//! normal sampling, separable Smith G) + a smooth dielectric for glass /
//! crystal. Follows the SolidCraft path tracer's lobe mix
//! (`diffuse probability = 0.5 (1 - metallic)`) so densities match its MIS.

use crate::materials::shading::{Surface, fresnel_dielectric, fresnel_schlick, refract};
use crate::render::trace::Rng;
use bevy::math::Vec3;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lobe {
    Diffuse,
    Glossy,
    Transmission,
}

#[derive(Clone, Copy, Debug)]
pub struct BsdfSample {
    pub dir: Vec3,
    /// `f cos / pdf` (the path throughput multiplier).
    pub weight: Vec3,
    /// Solid-angle density (0 for delta lobes).
    pub pdf: f32,
    pub lobe: Lobe,
    pub delta: bool,
}

/// One shading point's BSDF in world space; `n` faces the viewer.
pub struct Bsdf {
    pub n: Vec3,
    pub base: Vec3,
    pub metallic: f32,
    /// GGX alpha (roughness²), clamped away from 0.
    pub alpha: f32,
    pub transmission: f32,
    pub ior: f32,
    pub entering: bool,
}

/// GGX normal distribution `D(h)`.
pub fn distribution(nh: f32, a2: f32) -> f32 {
    let d = nh * nh * (a2 - 1.0) + 1.0;
    a2 / (PI * d * d).max(1e-12)
}

/// Smith masking for one direction.
pub fn smith_g1(nx: f32, a2: f32) -> f32 {
    2.0 * nx / (nx + (a2 + (1.0 - a2) * nx * nx).sqrt()).max(1e-12)
}

/// Density of a VNDF-sampled GGX reflection `D G1(v) / (4 nv)`.
pub fn ggx_pdf(nh: f32, nv: f32, a2: f32) -> f32 {
    distribution(nh, a2) / (2.0 * (nv + (a2 + (1.0 - a2) * nv * nv).sqrt())).max(1e-12)
}

impl Bsdf {
    pub fn new(s: &Surface, n: Vec3, entering: bool) -> Self {
        let r = s.roughness.clamp(0.0, 1.0);
        Self { n, base: s.base, metallic: s.metallic.clamp(0.0, 1.0), alpha: (r * r).max(2e-3), transmission: s.transmission.clamp(0.0, 1.0), ior: s.ior.max(1.0), entering }
    }

    fn f0(&self) -> Vec3 {
        Vec3::splat(0.04).lerp(self.base, self.metallic)
    }

    fn p_diffuse(&self) -> f32 {
        0.5 * (1.0 - self.metallic)
    }

    /// `(f cos, pdf)` for a non-delta direction `l` (NEE / MIS), including
    /// the opaque share `1 - transmission`.
    pub fn eval(&self, v: Vec3, l: Vec3) -> (Vec3, f32) {
        let (nv, nl) = (self.n.dot(v), self.n.dot(l));
        let opaque = 1.0 - self.transmission;
        if nv <= 1e-5 || nl <= 0.0 || opaque <= 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        let h = (v + l).normalize_or_zero();
        let (nh, vh) = (self.n.dot(h).max(0.0), v.dot(h).max(0.0));
        let a2 = self.alpha * self.alpha;
        let diffuse = self.base * (1.0 - self.metallic) / PI * nl;
        let f = fresnel_schlick(vh, self.f0());
        let spec = f * (distribution(nh, a2) * smith_g1(nv, a2) * smith_g1(nl, a2) / (4.0 * nv));
        let pd = self.p_diffuse();
        let pdf = pd * nl / PI + (1.0 - pd) * ggx_pdf(nh, nv, a2);
        ((diffuse + spec) * opaque, pdf * opaque)
    }

    /// Samples an outgoing direction for view direction `v`.
    pub fn sample(&self, v: Vec3, rng: &mut Rng) -> Option<BsdfSample> {
        if self.transmission > 0.0 && rng.f() < self.transmission {
            return self.sample_glass(v, rng);
        }
        let nv = self.n.dot(v);
        if nv <= 1e-5 {
            return None;
        }
        let pd = self.p_diffuse();
        let (dir, lobe) = if rng.f() < pd {
            (super::super::trace::cosine_dir(self.n, rng), Lobe::Diffuse)
        } else {
            let h = sample_vndf(self.n, v, self.alpha, rng.f(), rng.f());
            (v.reflect_about(h), Lobe::Glossy)
        };
        let (f_cos, pdf) = self.eval(v, dir);
        if pdf <= 1e-8 || f_cos.max_element() <= 0.0 {
            return None;
        }
        Some(BsdfSample { dir, weight: f_cos / pdf, pdf, lobe, delta: false })
    }

    /// Smooth dielectric: Fresnel-chosen mirror reflection or refraction;
    /// the color tints light entering the volume.
    fn sample_glass(&self, v: Vec3, rng: &mut Rng) -> Option<BsdfSample> {
        let d = -v;
        let eta = if self.entering { 1.0 / self.ior } else { self.ior };
        let cos = self.n.dot(v).clamp(0.0, 1.0);
        let refr = refract(d, self.n, eta);
        let fr = if refr.is_some() { fresnel_dielectric(cos, eta) } else { 1.0 };
        let blur = |dir: Vec3, rng: &mut Rng| (dir + rng.ball() * self.alpha * 0.5).normalize_or(dir);
        if rng.f() < fr {
            let dir = blur(d - self.n * 2.0 * d.dot(self.n), rng);
            return Some(BsdfSample { dir, weight: Vec3::ONE, pdf: 0.0, lobe: Lobe::Glossy, delta: true });
        }
        let tint = if self.entering { self.base } else { Vec3::ONE };
        Some(BsdfSample { dir: blur(refr?, rng), weight: tint, pdf: 0.0, lobe: Lobe::Transmission, delta: true })
    }
}

trait Reflect {
    fn reflect_about(self, h: Vec3) -> Vec3;
}

impl Reflect for Vec3 {
    /// Mirrors `self` (pointing away from the surface) about `h`.
    fn reflect_about(self, h: Vec3) -> Vec3 {
        h * 2.0 * self.dot(h) - self
    }
}

/// Heitz 2018 visible-normal sampling of an isotropic GGX lobe around `n`.
pub fn sample_vndf(n: Vec3, v: Vec3, alpha: f32, u1: f32, u2: f32) -> Vec3 {
    let (t, b) = n.any_orthonormal_pair();
    let vl = Vec3::new(v.dot(t), v.dot(b), v.dot(n));
    let vh = Vec3::new(alpha * vl.x, alpha * vl.y, vl.z).normalize();
    let len2 = vh.x * vh.x + vh.y * vh.y;
    let t1 = if len2 > 0.0 { Vec3::new(-vh.y, vh.x, 0.0) / len2.sqrt() } else { Vec3::X };
    let t2 = vh.cross(t1);
    let r = u1.sqrt();
    let phi = 2.0 * PI * u2;
    let p1 = r * phi.cos();
    let s = 0.5 * (1.0 + vh.z);
    let p2 = (1.0 - s) * (1.0 - p1 * p1).max(0.0).sqrt() + s * r * phi.sin();
    let nh = t1 * p1 + t2 * p2 + vh * (1.0 - p1 * p1 - p2 * p2).max(0.0).sqrt();
    let hl = Vec3::new(alpha * nh.x, alpha * nh.y, nh.z.max(0.0)).normalize();
    (t * hl.x + b * hl.y + n * hl.z).normalize()
}
