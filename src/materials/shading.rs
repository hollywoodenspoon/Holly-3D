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
    /// Cavity ambient occlusion (1 = open); darkens ambient light only.
    pub ao: f32,
}

/// Evaluates material parameters, maps and paint layers; `vcol` is the
/// vertex paint color.
pub fn surface(m: &Material, uv: Vec2, p: Vec3, vcol: Vec4) -> Surface {
    let mut s = base_surface(m, uv, p, vcol);
    if let Some(paint) = &m.paint {
        paint.apply(uv, p, &mut s, || material_height(m, uv, p).unwrap_or(0.5));
    }
    s
}

/// [`surface`] for a point whose place and normal in the object's
/// projection space (`tp`, `tn`) are known: World / Triplanar materials
/// are projected from three sides; paint layers always use the UVs.
pub fn surface_at(m: &Material, uv: Vec2, p: Vec3, tp: Vec3, tn: Vec3, vcol: Vec4) -> Surface {
    if !m.projection.on() {
        return surface(m, uv, p, vcol);
    }
    let mut s = base_surface_at(m, uv, tp, tn, vcol);
    if let Some(paint) = &m.paint {
        paint.apply(uv, p, &mut s, || m.projection.blend(tp, tn, |puv, _| material_height(m, puv, tp).unwrap_or(0.5)));
    }
    s
}

/// [`base_surface`], projected from three sides when the material uses
/// World / Triplanar mapping (`tp` / `tn`: point and normal in projection space).
pub fn base_surface_at(m: &Material, uv: Vec2, tp: Vec3, tn: Vec3, vcol: Vec4) -> Surface {
    if m.projection.on() { m.projection.blend(tp, tn, |puv, _| Blend(base_surface(m, puv, tp, vcol))).0 } else { base_surface(m, uv, tp, vcol) }
}

/// A surface that can be weighted and summed (triplanar blending).
struct Blend(Surface);

impl std::ops::Mul<f32> for Blend {
    type Output = Blend;
    fn mul(self, k: f32) -> Blend {
        let s = self.0;
        Blend(Surface {
            base: s.base * k,
            alpha: s.alpha * k,
            metallic: s.metallic * k,
            roughness: s.roughness * k,
            transmission: s.transmission * k,
            ior: s.ior * k,
            emissive: s.emissive * k,
            ao: s.ao * k,
        })
    }
}

impl std::ops::Add for Blend {
    type Output = Blend;
    fn add(self, o: Blend) -> Blend {
        let (a, b) = (self.0, o.0);
        Blend(Surface {
            base: a.base + b.base,
            alpha: a.alpha + b.alpha,
            metallic: a.metallic + b.metallic,
            roughness: a.roughness + b.roughness,
            transmission: a.transmission + b.transmission,
            ior: a.ior + b.ior,
            emissive: a.emissive + b.emissive,
            ao: a.ao + b.ao,
        })
    }
}

/// The surface as a camera ray sees it. With "Preserve Texture
/// Appearance" on, the glow is the surface's own colour at Direct Camera
/// Brightness, however high the Emission Multiplier is; light sent into
/// the scene (GI, emitter sampling, reflections) keeps the full emission.
pub fn camera_view(m: &Material, s: &Surface) -> Surface {
    let mut out = *s;
    if m.decoupled() {
        out.emissive = s.base * m.direct_camera_brightness.clamp(0.0, 2.0);
    }
    out
}

/// Height of the material's own relief at a point (0..1), when it has
/// one: its procedural surface, else a height map on the Bump slot.
pub fn material_height(m: &Material, uv: Vec2, p: Vec3) -> Option<f32> {
    if let Some(pm) = super::proc_engine::surface::maps(&m.proc, m.roughness, m.metallic) {
        return Some(pm.at(uv).height);
    }
    match m.active_map(MapChannel::Bump) {
        Some((super::maps::MapNode::NormalFromHeight { .. }, _)) => None,
        Some((super::maps::MapNode::Bitmap { path, .. }, _)) if super::relief::is_normal_map(path) => None,
        Some((n, _)) => Some(n.sample_scalar(uv, p)),
        _ => None,
    }
}

/// The material without its paint layers ("Layer 0: Base Material").
pub fn base_surface(m: &Material, uv: Vec2, p: Vec3, vcol: Vec4) -> Surface {
    let mut base = Vec3::from_array(m.base_color);
    let mut alpha = m.opacity;
    let (mut metallic, mut roughness, mut ao) = (m.metallic, m.roughness, 1.0);
    // The procedural surface stands in for the plain colour, roughness and metalness.
    if let Some(pm) = super::proc_engine::surface::maps(&m.proc, m.roughness, m.metallic) {
        let s = pm.at(uv);
        (base, metallic, roughness, ao) = (s.base, s.metal, s.rough, s.ao);
    }
    let mut emissive = lin3(m.emissive) * m.emissive_strength;
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
    if m.alpha_cutout {
        alpha = if alpha >= 0.5 { 1.0 } else { 0.0 };
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
        emissive,
        ao,
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
            let mut c = diffuse * ambient * s.ao + s.emissive;
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
