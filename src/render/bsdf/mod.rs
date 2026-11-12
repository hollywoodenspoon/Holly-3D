//! Principled path-tracer BSDF: Lambert diffuse (+ random-walk SSS entry and
//! cloth sheen) and anisotropic GGX specular, under an optional clearcoat
//! lacquer, plus a smooth / frosted dielectric with chromatic dispersion
//! for glass and crystal. Follows the SolidCraft path tracer's lobe mix
//! (`diffuse probability = 0.5 (1 - metallic)`) so densities match its MIS.

pub mod microfacet;
#[cfg(test)]
mod tests;

use crate::materials::principled::{CHANNEL_LAMBDA, Principled};
use crate::materials::shading::{Surface, fresnel_dielectric, fresnel_schlick, refract};
use crate::render::trace::Rng;
use bevy::math::Vec3;
use microfacet::{Frame, alphas, schlick_w};
pub use microfacet::{distribution, sample_vndf_world as sample_vndf, smith_g1};
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
    /// The path enters the surface for a subsurface random walk.
    pub sss: bool,
    /// Dispersion picked this color channel for the rest of the path.
    pub channel: Option<usize>,
}

/// One shading point's BSDF in world space; `n` faces the viewer.
pub struct Bsdf {
    pub n: Vec3,
    frame: Frame,
    pub base: Vec3,
    pub metallic: f32,
    /// Isotropic GGX alpha (roughness²) — frosted refraction blur.
    pub alpha: f32,
    ax: f32,
    ay: f32,
    pub transmission: f32,
    pub ior: f32,
    pub entering: bool,
    clearcoat: f32,
    cc_alpha: f32,
    sheen: Vec3,
    sss: f32,
    /// Per-channel IOR when dispersion is on.
    ior_rgb: Option<[f32; 3]>,
    /// Channel already chosen by an earlier dispersive refraction.
    pub channel: Option<usize>,
    /// Pure white Lambert (random-walk exit).
    diffuse_only: bool,
}

/// GGX density of an isotropic VNDF reflection (kept for the GPU parity tests).
pub fn ggx_pdf(nh: f32, nv: f32, a2: f32) -> f32 {
    distribution(nh, a2) / (2.0 * (nv + (a2 + (1.0 - a2) * nv * nv).sqrt())).max(1e-12)
}

impl Bsdf {
    pub fn new(s: &Surface, n: Vec3, entering: bool) -> Self {
        let r = s.roughness.clamp(0.0, 1.0);
        let a = (r * r).max(2e-3);
        Self {
            n,
            frame: Frame::new(n, 0.0),
            base: s.base,
            metallic: s.metallic.clamp(0.0, 1.0),
            alpha: a,
            ax: a,
            ay: a,
            transmission: s.transmission.clamp(0.0, 1.0),
            ior: s.ior.max(1.0),
            entering,
            clearcoat: 0.0,
            cc_alpha: 0.0,
            sheen: Vec3::ZERO,
            sss: 0.0,
            ior_rgb: None,
            channel: None,
            diffuse_only: false,
        }
    }

    /// Adds the material's Principled lobes.
    pub fn principled(mut self, p: &Principled, roughness: f32) -> Self {
        if p.anisotropy > 0.0 {
            self.frame = Frame::new(self.n, p.aniso_rotation);
            (self.ax, self.ay) = alphas(roughness.clamp(0.0, 1.0), p.anisotropy);
        }
        self.clearcoat = p.clearcoat.clamp(0.0, 1.0);
        let cr = p.clearcoat_roughness.clamp(0.0, 1.0);
        self.cc_alpha = (cr * cr).max(2e-3);
        let tint = Vec3::ONE.lerp(self.base / self.base.max_element().max(1e-4), p.sheen_tint.clamp(0.0, 1.0));
        self.sheen = tint * p.sheen.clamp(0.0, 1.0);
        self.sss = p.sss_weight.clamp(0.0, 1.0);
        if p.abbe > 0.0 {
            self.ior_rgb = Some(CHANNEL_LAMBDA.map(|l| p.ior_at(self.ior, l).max(1.0)));
        }
        self
    }

    fn f0(&self) -> Vec3 {
        Vec3::splat(0.04).lerp(self.base, self.metallic)
    }

    fn p_diffuse(&self) -> f32 {
        0.5 * (1.0 - self.metallic)
    }

    fn p_coat(&self) -> f32 {
        self.clearcoat * 0.25
    }

    /// `(f cos, pdf)` for a non-delta direction `l` (NEE / MIS), including
    /// the opaque share `1 - transmission`.
    pub fn eval(&self, v: Vec3, l: Vec3) -> (Vec3, f32) {
        let (vl, ll) = (self.frame.local(v), self.frame.local(l));
        let opaque = 1.0 - self.transmission;
        if vl.z <= 1e-5 || ll.z <= 0.0 || opaque <= 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        if self.diffuse_only {
            return (Vec3::splat(ll.z / PI), ll.z / PI);
        }
        let hl = (vl + ll).normalize_or_zero();
        let vh = vl.dot(hl).max(0.0);
        let kd = (1.0 - self.metallic) * ll.z;
        // Sheen sits on the diffuse layer, which gives up the sheen's
        // directional albedo (~0.3 (1 - nv)^5 at full weight).
        let under = 1.0 - 0.3 * self.sheen.max_element() * schlick_w(vl.z);
        let diffuse = self.base * (kd * (1.0 - self.sss) * under / PI) + self.sheen * (kd * schlick_w(ll.dot(hl)));
        let spec = fresnel_schlick(vh, self.f0()) * microfacet::spec(vl, ll, self.ax, self.ay);
        let (pd, pc) = (self.p_diffuse(), self.p_coat());
        let mut pdf = pd * (1.0 - self.sss) * ll.z / PI + (1.0 - pd) * microfacet::pdf(vl, hl, self.ax, self.ay);
        let mut f = diffuse + spec;
        if self.clearcoat > 0.0 {
            let fc = 0.04 + 0.96 * schlick_w(vh);
            let coat = self.clearcoat * fc * microfacet::spec(vl, ll, self.cc_alpha, self.cc_alpha);
            f = f * (1.0 - self.clearcoat * (0.04 + 0.96 * schlick_w(vl.z))) + Vec3::splat(coat);
            pdf = (1.0 - pc) * pdf + pc * microfacet::pdf(vl, hl, self.cc_alpha, self.cc_alpha);
        }
        (f * opaque, pdf * opaque)
    }

    /// Samples an outgoing direction for view direction `v`.
    pub fn sample(&self, v: Vec3, rng: &mut Rng) -> Option<BsdfSample> {
        if self.transmission > 0.0 && rng.f() < self.transmission {
            return self.sample_glass(v, rng);
        }
        let vl = self.frame.local(v);
        if vl.z <= 1e-5 {
            return None;
        }
        let (pd, pc) = (self.p_diffuse(), self.p_coat());
        if self.diffuse_only {
            let dir = super::trace::cosine_dir(self.n, rng);
            let pdf = self.n.dot(dir).max(1e-6) / PI;
            return Some(BsdfSample { dir, weight: Vec3::ONE, pdf, lobe: Lobe::Diffuse, delta: false, sss: false, channel: None });
        }
        let (dir, lobe) = if pc > 0.0 && rng.f() < pc {
            let h = microfacet::sample_vndf(vl, self.cc_alpha, self.cc_alpha, rng.f(), rng.f());
            (self.frame.world(h * 2.0 * vl.dot(h) - vl), Lobe::Glossy)
        } else if rng.f() < pd {
            if self.sss > 0.0 && rng.f() < self.sss {
                return Some(self.sss_entry(vl.z, pd, pc));
            }
            (super::trace::cosine_dir(self.n, rng), Lobe::Diffuse)
        } else {
            let h = microfacet::sample_vndf(vl, self.ax, self.ay, rng.f(), rng.f());
            (self.frame.world(h * 2.0 * vl.dot(h) - vl), Lobe::Glossy)
        };
        let (f_cos, pdf) = self.eval(v, dir);
        if pdf <= 1e-8 || f_cos.max_element() <= 0.0 {
            return None;
        }
        Some(BsdfSample { dir, weight: f_cos / pdf, pdf, lobe, delta: false, sss: false, channel: None })
    }

    /// Diffuse light that enters for the random walk: the energy (and base
    /// color) the `(1 - sss)` diffuse reflection gave up, over its pick
    /// probability. The walk then tints it with the SSS Scatter Color.
    fn sss_entry(&self, nv: f32, pd: f32, pc: f32) -> BsdfSample {
        let coat = 1.0 - self.clearcoat * (0.04 + 0.96 * schlick_w(nv));
        let w = (1.0 - self.metallic) * coat / (pd * (1.0 - pc)).max(1e-6);
        BsdfSample { dir: -self.n, weight: self.base * w, pdf: 0.0, lobe: Lobe::Diffuse, delta: true, sss: true, channel: None }
    }

    /// Smooth / frosted dielectric: Fresnel-chosen reflection or refraction;
    /// the color tints light entering the volume. Dispersion follows one
    /// color channel per path, with its own Cauchy IOR.
    fn sample_glass(&self, v: Vec3, rng: &mut Rng) -> Option<BsdfSample> {
        let d = -v;
        let (mut ior, mut channel, mut spectral) = (self.ior, None, Vec3::ONE);
        if let Some(rgb) = self.ior_rgb {
            let k = self.channel.unwrap_or_else(|| ((rng.f() * 3.0) as usize).min(2));
            ior = rgb[k];
            if self.channel.is_none() {
                channel = Some(k);
                spectral = Vec3::ZERO;
                spectral[k] = 3.0;
            }
        }
        let eta = if self.entering { 1.0 / ior } else { ior };
        let cos = self.n.dot(v).clamp(0.0, 1.0);
        let refr = refract(d, self.n, eta);
        let fr = if refr.is_some() { fresnel_dielectric(cos, eta) } else { 1.0 };
        let blur = |dir: Vec3, rng: &mut Rng| (dir + rng.ball() * self.alpha * 0.5).normalize_or(dir);
        let s = |dir, weight, lobe, channel| BsdfSample { dir, weight, pdf: 0.0, lobe, delta: true, sss: false, channel };
        if rng.f() < fr {
            return Some(s(blur(d - self.n * 2.0 * d.dot(self.n), rng), Vec3::ONE, Lobe::Glossy, None));
        }
        let tint = if self.entering { self.base } else { Vec3::ONE };
        Some(s(blur(refr?, rng), tint * spectral, Lobe::Transmission, channel))
    }

    /// Lambertian BSDF (white) at the exit point of a random walk.
    pub fn lambert(n: Vec3) -> Self {
        let s = Surface { base: Vec3::ONE, alpha: 1.0, metallic: 0.0, roughness: 1.0, transmission: 0.0, ior: 1.5, emissive: Vec3::ZERO, ao: 1.0 };
        Self { diffuse_only: true, ..Self::new(&s, n, true) }
    }
}
