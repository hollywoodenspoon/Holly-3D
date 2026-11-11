//! Sample-slot preview spheres (analytic ray-sphere on a checker backdrop),
//! rendered on the CPU with the same shading code as the raytracer.

use super::shading::{LightIn, fresnel_schlick, lin3, refract, shade, surface, tonemap};
use super::{Material, ShadingModel};
use bevy::math::{Vec2, Vec3, Vec4};

const KEY: Vec3 = Vec3::new(-0.55, 0.62, 0.56);

fn backdrop(d: Vec3, origin: Vec3) -> Vec3 {
    // Checker backdrop plane at z = -2.5, sky gradient elsewhere.
    if d.z < -1e-3 {
        let t = (-2.5 - origin.z) / d.z;
        let q = origin + d * t;
        let c = ((q.x * 1.6).floor() + (q.y * 1.6).floor()).rem_euclid(2.0);
        return if c < 1.0 { Vec3::splat(0.05) } else { Vec3::splat(0.38) };
    }
    env(d)
}

/// Studio environment for reflections: dim floor, soft sky, one softbox.
fn env(d: Vec3) -> Vec3 {
    let sky = Vec3::new(0.18, 0.2, 0.24).lerp(Vec3::new(0.55, 0.58, 0.62), (d.y * 0.5 + 0.5).clamp(0.0, 1.0));
    let soft = (d.dot(KEY.normalize()) - 0.86).max(0.0) * 30.0;
    sky + Vec3::splat(soft.min(4.0))
}

/// Shades one camera ray; returns linear color + coverage.
fn trace(m: &Material, o: Vec3, d: Vec3) -> (Vec3, f32) {
    let b = o.dot(d);
    let disc = b * b - (o.length_squared() - 1.0);
    if disc < 0.0 {
        return (backdrop(d, o), 0.0);
    }
    let p = o + d * (-b - disc.sqrt());
    let n = p.normalize();
    let v = -d;
    let uv = Vec2::new(n.x.atan2(n.z) / std::f32::consts::TAU + 0.5, n.y.asin() / std::f32::consts::PI + 0.5);
    let q = Vec3::new(p.x, -p.z, p.y);
    // Relief on the sphere: U runs around it, V up it (3 tiles around, so bricks read).
    let east = Vec3::new(n.z, 0.0, -n.x).normalize_or(Vec3::X);
    let north = n.cross(east);
    let n = match super::relief::tangent_normal(m, uv, q, Vec3::ZERO, Vec3::ZERO) {
        Some(tn) => super::relief::perturb(n, east, north, tn),
        None => n,
    };
    let s = surface(m, uv, q, Vec4::ONE);
    let lights = [
        LightIn { dir: KEY.normalize(), radiance: Vec3::splat(3.2) },
        LightIn { dir: Vec3::new(0.7, -0.2, 0.68).normalize(), radiance: Vec3::new(0.7, 0.75, 0.9) },
    ];
    let mut c = shade(m, &s, n, v, &lights, Vec3::splat(0.35));
    if matches!(m.model, ShadingModel::Physical | ShadingModel::Crystal) {
        let r = d - n * 2.0 * d.dot(n);
        let f0 = Vec3::splat(0.04).lerp(s.base, s.metallic);
        let f = fresnel_schlick(n.dot(v), f0);
        let blur = s.roughness.powf(0.7);
        let refl = env(r).lerp(Vec3::splat(0.32), blur);
        c += f * refl * (1.0 - blur * 0.75);
        if s.transmission > 0.0 {
            c = c.lerp(transmit(m, p, d, n, s.base, s.ior) + f * refl, s.transmission);
        }
    }
    let alpha = s.alpha;
    (c.lerp(backdrop(d, o), 1.0 - alpha), 1.0)
}

/// Refraction through the unit sphere with Beer-Lambert absorption.
fn transmit(m: &Material, p: Vec3, d: Vec3, n: Vec3, base: Vec3, ior: f32) -> Vec3 {
    let Some(t) = refract(d, n, 1.0 / ior) else { return env(d) };
    let len = (-2.0 * p.dot(t)).max(0.0);
    let q = p + t * len;
    let out = refract(t, -q.normalize(), ior).unwrap_or(t - q * 2.0 * t.dot(q));
    // Light reaches the attenuation color after `2 x thickness` (same as the viewport).
    let absorb = lin3(m.attenuation_color).powf(len / (m.thickness.max(0.01) * 2.0));
    backdrop(out, q) * absorb * base
}

/// Renders a `size` x `size` RGBA preview of the material.
pub fn render_preview(m: &Material, size: u32) -> Vec<[u8; 4]> {
    let size = size.max(8);
    let eye = Vec3::new(0.0, 0.0, 3.4);
    let half = 0.38;
    let ink = lin3(m.outline.color);
    let mut out = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut acc = Vec3::ZERO;
            for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let px = ((x as f32 + sx) / size as f32 * 2.0 - 1.0) * half;
                let py = (1.0 - (y as f32 + sy) / size as f32 * 2.0) * half;
                let d = Vec3::new(px, py, -1.0).normalize();
                let (mut c, hit) = trace(m, eye, d);
                if m.outline.enabled && m.outline.width > 0.0 {
                    // Silhouette distance in pixels (sphere radius ~ size * 0.39 px).
                    let miss = (eye.cross(d).length() - 1.0) * size as f32 * 0.42;
                    let w = m.outline.width * 0.8 + 0.5;
                    if (hit == 0.0 && miss < w) || (hit > 0.0 && miss > -0.6) {
                        c = ink;
                    }
                }
                acc += c;
            }
            let c = tonemap(acc / 4.0);
            out.push([(c.x * 255.0) as u8, (c.y * 255.0) as u8, (c.z * 255.0) as u8, 255]);
        }
    }
    out
}
