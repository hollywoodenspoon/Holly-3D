//! CPU shading shared by slot previews and the built-in raytracer:
//! resolves a material + maps at a surface point and evaluates the
//! Physical (GGX), Toon ramp, Crystal and Unlit models.

use super::maps::MapChannel;
use super::{Material, ShadingModel};
use bevy::math::{Vec2, Vec3, Vec4};
use std::f32::consts::PI;

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

pub fn linear_to_srgb(c: f32) -> f32 {
    let c = c.max(0.0);
    if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

pub fn lin3(c: [f32; 3]) -> Vec3 {
    Vec3::from_array(c.map(srgb_to_linear))
}

pub fn lin(c: Vec3) -> Vec3 {
    Vec3::new(srgb_to_linear(c.x), srgb_to_linear(c.y), srgb_to_linear(c.z))
}

/// Material resolved at one surface point (linear color).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Surface {
    pub base: Vec3,
    pub alpha: f32,
    pub metallic: f32,
    pub roughness: f32,
    pub transmission: f32,
    pub ior: f32,
    pub emissive: Vec3,
}

/// Evaluates material parameters and maps; `vcol` is the vertex paint color.
pub fn surface(m: &Material, uv: Vec2, p: Vec3, vcol: Vec4) -> Surface {
    let mut base = Vec3::from_array(m.base_color);
    let mut alpha = m.opacity;
    let (mut metallic, mut roughness) = (m.metallic, m.roughness);
    let mut emissive = Vec3::from_array(m.emissive) * m.emissive_strength;
    if let Some((n, a)) = m.active_map(MapChannel::BaseColor) {
        let s = n.sample(uv, p);
        base = base.lerp(s.truncate(), a);
        alpha *= 1.0 - a + a * s.w;
    }
    if let Some((n, a)) = m.active_map(MapChannel::Metallic) {
        metallic = metallic * (1.0 - a) + n.sample_scalar(uv, p) * a;
    }
    if let Some((n, a)) = m.active_map(MapChannel::Roughness) {
        roughness = roughness * (1.0 - a) + n.sample_scalar(uv, p) * a;
    }
    if let Some((n, a)) = m.active_map(MapChannel::Opacity) {
        alpha *= 1.0 - a + a * n.sample_scalar(uv, p);
    }
    if let Some((n, a)) = m.active_map(MapChannel::Emission) {
        emissive += lin(n.sample(uv, p).truncate()) * a * m.emissive_strength.max(1.0);
    }
    Surface {
        base: lin(base) * lin(vcol.truncate()),
        alpha: (alpha * vcol.w).clamp(0.0, 1.0),
        metallic: metallic.clamp(0.0, 1.0),
        roughness: roughness.clamp(0.02, 1.0),
        transmission: if m.model == ShadingModel::Crystal { m.transmission.clamp(0.0, 1.0) } else { 0.0 },
        ior: m.ior.max(1.0),
        emissive: lin(emissive.min(Vec3::splat(1.0))) * emissive.max_element().max(1.0),
    }
}

/// Bump-map height at a point (0 when no bump map).
pub fn bump_height(m: &Material, uv: Vec2, p: Vec3) -> f32 {
    m.active_map(MapChannel::Bump).map_or(0.0, |(n, a)| n.sample_scalar(uv, p) * a)
}

/// Incoming light at a point: direction towards the light + radiance (shadowed).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct LightIn {
    pub dir: Vec3,
    pub radiance: Vec3,
}

pub fn fresnel_schlick(cos: f32, f0: Vec3) -> Vec3 {
    f0 + (Vec3::ONE - f0) * (1.0 - cos.clamp(0.0, 1.0)).powi(5)
}

/// Dielectric Fresnel reflectance for an interface with relative IOR `eta`.
pub fn fresnel_dielectric(cos_i: f32, eta: f32) -> f32 {
    let f0 = ((1.0 - eta) / (1.0 + eta)).powi(2);
    f0 + (1.0 - f0) * (1.0 - cos_i.abs().clamp(0.0, 1.0)).powi(5)
}

/// Refracts `d` through normal `n` (facing against `d`) with ratio `eta` = n1/n2.
pub fn refract(d: Vec3, n: Vec3, eta: f32) -> Option<Vec3> {
    let cos_i = -n.dot(d);
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    (k >= 0.0).then(|| (d * eta + n * (eta * cos_i - k.sqrt())).normalize())
}

fn ggx(n: Vec3, v: Vec3, l: Vec3, rough: f32, f0: Vec3) -> Vec3 {
    let h = (v + l).normalize_or_zero();
    let (nl, nv, nh, vh) = (n.dot(l).max(0.0), n.dot(v).max(1e-4), n.dot(h).max(0.0), v.dot(h).max(0.0));
    let a2 = (rough * rough).powi(2).max(1e-6);
    let d = a2 / (PI * (nh * nh * (a2 - 1.0) + 1.0).powi(2));
    let k = (rough + 1.0).powi(2) / 8.0;
    let g = nv / (nv * (1.0 - k) + k) * nl / (nl * (1.0 - k) + k);
    fresnel_schlick(vh, f0) * (d * g / (4.0 * nv * nl).max(1e-4))
}

/// Direct lighting (+ ambient) for one shading point. `n` faces the viewer,
/// `v` points towards the viewer. Reflections/refraction are added by the caller.
pub fn shade(m: &Material, s: &Surface, n: Vec3, v: Vec3, lights: &[LightIn], ambient: Vec3) -> Vec3 {
    match m.model {
        ShadingModel::Unlit => s.base + s.emissive,
        ShadingModel::Toon => shade_toon(m, s, n, v, lights, ambient),
        _ => {
            let f0 = Vec3::splat(0.04).lerp(s.base, s.metallic);
            let diffuse = s.base * (1.0 - s.metallic) * (1.0 - s.transmission);
            let mut c = diffuse * ambient + s.emissive;
            for l in lights {
                let nl = n.dot(l.dir).max(0.0);
                if nl > 0.0 {
                    c += (diffuse / PI + ggx(n, v, l.dir, s.roughness, f0)) * l.radiance * nl;
                }
            }
            c
        }
    }
}

fn shade_toon(m: &Material, s: &Surface, n: Vec3, v: Vec3, lights: &[LightIn], ambient: Vec3) -> Vec3 {
    let t = &m.toon;
    let steps = t.steps.clamp(2, 5) as f32;
    let shadow = s.base * lin3(t.shadow_tint);
    let mut c = shadow * ambient.max_element().min(1.0) * 0.6 + s.emissive;
    for l in lights {
        let nl = n.dot(l.dir).max(0.0);
        let band = ((nl * steps).floor() / (steps - 1.0)).min(1.0);
        let energy = l.radiance / PI;
        c += shadow.lerp(s.base, band) * energy;
        let h = (l.dir + v).normalize_or_zero();
        let shin = 2.0 / (s.roughness * s.roughness).max(1e-3);
        if t.specular > 0.0 && nl > 0.0 && n.dot(h).max(0.0).powf(shin) > 0.5 {
            c += Vec3::splat(t.specular) * energy;
        }
    }
    if t.rim > 0.0 && 1.0 - n.dot(v).max(0.0) > 0.62 {
        c += lin3(t.rim_color) * t.rim * 0.6;
    }
    c
}

/// ACES-ish filmic tonemap -> display sRGB (0..1).
pub fn tonemap(c: Vec3) -> Vec3 {
    let f = |x: f32| {
        let x = x.max(0.0) * 0.8;
        linear_to_srgb(((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0))
    };
    Vec3::new(f(c.x), f(c.y), f(c.z))
}
