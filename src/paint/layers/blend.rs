//! Folding the paint layers over the base material at one surface point.
//! Used per shading sample by the raytracer and per texel by the viewport
//! composite, so both show the same result.

use super::source::EMIT_GAIN;
use super::{BlendMode, PaintStack};
use crate::materials::shading::{Surface, linear_to_srgb, srgb_to_linear};
use crate::paint::texel::{EMISSION, HEIGHT, METAL, ROUGH};
use bevy::math::{Vec2, Vec3};

/// The height the paint layers add at a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Relief {
    /// 0..1; 0.5 is level with the base surface.
    pub height: f32,
    /// How much the layers cover the base relief (0 = base shows, 1 = flat).
    pub cover: f32,
}

impl Relief {
    pub const NONE: Relief = Relief { height: 0.5, cover: 0.0 };
}

fn to_srgb(c: Vec3) -> Vec3 {
    Vec3::new(linear_to_srgb(c.x), linear_to_srgb(c.y), linear_to_srgb(c.z))
}

fn to_linear(c: Vec3) -> Vec3 {
    Vec3::new(srgb_to_linear(c.x), srgb_to_linear(c.y), srgb_to_linear(c.z))
}

fn mix_color(mode: BlendMode, below: Vec3, layer: Vec3) -> Vec3 {
    match mode {
        BlendMode::Normal | BlendMode::HeightBlend => layer,
        BlendMode::Multiply => below * layer,
        BlendMode::Add => (below + layer).min(Vec3::ONE),
        BlendMode::Screen => Vec3::ONE - (Vec3::ONE - below) * (Vec3::ONE - layer),
    }
}

fn mix_scalar(mode: BlendMode, below: f32, layer: f32) -> f32 {
    match mode {
        BlendMode::Normal | BlendMode::HeightBlend => layer,
        BlendMode::Multiply => below * layer,
        BlendMode::Add => (below + layer).min(1.0),
        BlendMode::Screen => 1.0 - (1.0 - below) * (1.0 - layer),
    }
}

/// Height-Blend weight: a layer risen to `level` (its opacity) covers
/// everything lower than that, with a soft shoreline.
pub fn height_weight(level: f32, base_height: f32) -> f32 {
    if level >= 1.0 {
        return 1.0;
    }
    let (lo, hi) = (level - 0.08, level + 0.08);
    let t = ((base_height - lo) / (hi - lo)).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

impl PaintStack {
    /// Folds every visible layer over `s` at `uv`. `base_height` is the
    /// height of the base surface (asked for only by Height-Blend layers).
    pub fn apply(&self, uv: Vec2, p: Vec3, s: &mut Surface, base_height: impl Fn() -> f32) -> Relief {
        let mut relief = Relief::NONE;
        // Colours blend in sRGB, like paint; converted only once painted.
        let mut color: Option<Vec3> = None;
        let mut bh: Option<f32> = None;
        for l in self.layers.iter().filter(|l| l.visible && l.opacity > 0.0) {
            let Some(smp) = l.sample(uv, p) else { continue };
            let k = match l.blend {
                BlendMode::HeightBlend => height_weight(l.opacity, *bh.get_or_insert_with(&base_height)),
                _ => l.opacity.min(1.0),
            };
            if k <= 0.0 {
                continue;
            }
            let w = smp.a * k;
            if w > 0.0 {
                let below = color.unwrap_or_else(|| to_srgb(s.base));
                color = Some(below.lerp(mix_color(l.blend, below, smp.color), w));
            }
            let (wr, wm) = (smp.cov[ROUGH] * k, smp.cov[METAL] * k);
            s.roughness += (mix_scalar(l.blend, s.roughness, smp.pbr[ROUGH]) - s.roughness) * wr;
            s.metallic += (mix_scalar(l.blend, s.metallic, smp.pbr[METAL]) - s.metallic) * wm;
            let wh = smp.cov[HEIGHT] * k;
            if wh > 0.0 {
                match l.blend {
                    // Adds its bumps without flattening what is below.
                    BlendMode::Add | BlendMode::Screen | BlendMode::Multiply => relief.height += (smp.pbr[HEIGHT] - 0.5) * wh,
                    _ => {
                        relief.height += (smp.pbr[HEIGHT] - relief.height) * wh;
                        relief.cover += (1.0 - relief.cover) * wh;
                    }
                }
            }
            let we = smp.cov[EMISSION] * k;
            if we > 0.0 {
                let glow = to_linear(smp.color) * smp.pbr[EMISSION] * EMIT_GAIN;
                s.emissive = match l.blend {
                    BlendMode::Normal | BlendMode::HeightBlend => s.emissive.lerp(glow, we),
                    BlendMode::Multiply => s.emissive * (1.0 - we + we * smp.pbr[EMISSION]),
                    _ => s.emissive + glow * we,
                };
            }
        }
        if let Some(c) = color {
            s.base = to_linear(c);
        }
        s.roughness = s.roughness.clamp(0.02, 1.0);
        s.metallic = s.metallic.clamp(0.0, 1.0);
        relief.height = relief.height.clamp(0.0, 1.0);
        relief
    }

    /// Only the relief at `uv` (cheaper than [`Self::apply`]). Height-Blend
    /// layers count at their plain opacity here.
    pub fn relief(&self, uv: Vec2, p: Vec3) -> Relief {
        let mut r = Relief::NONE;
        for l in self.layers.iter().filter(|l| l.visible && l.opacity > 0.0) {
            let Some(smp) = l.sample(uv, p) else { continue };
            let wh = smp.cov[HEIGHT] * l.opacity.min(1.0);
            if wh <= 0.0 {
                continue;
            }
            match l.blend {
                BlendMode::Add | BlendMode::Screen | BlendMode::Multiply => r.height += (smp.pbr[HEIGHT] - 0.5) * wh,
                _ => {
                    r.height += (smp.pbr[HEIGHT] - r.height) * wh;
                    r.cover += (1.0 - r.cover) * wh;
                }
            }
        }
        r.height = r.height.clamp(0.0, 1.0);
        r
    }

    /// The tangent-space normal with the paint relief worked in: the base
    /// normal `base` fades where paint covers it, and the painted height
    /// tilts the result. `None` when there is nothing to tilt.
    pub fn tangent_normal(&self, uv: Vec2, p: Vec3, base: Option<Vec3>) -> Option<Vec3> {
        let here = self.relief(uv, p);
        let e = 1.0 / self.size as f32;
        let h = |du: f32, dv: f32| self.relief(uv + Vec2::new(du, dv), p).height;
        let (dx, dy) = ((h(e, 0.0) - h(-e, 0.0)) / (2.0 * e), (h(0.0, e) - h(0.0, -e)) / (2.0 * e));
        tilt(base, here.cover, dx, dy)
    }
}

/// Paint height slope (per UV unit) to normal tilt; matches a height map
/// on the Bump slot at Amount 100.
pub const PAINT_BUMP: f32 = 1.0 / 32.0;

/// Combines a base tangent normal with the paint cover and height slopes.
pub fn tilt(base: Option<Vec3>, cover: f32, dx: f32, dy: f32) -> Option<Vec3> {
    if cover <= 0.0 && dx == 0.0 && dy == 0.0 {
        return base;
    }
    let b = base.unwrap_or(Vec3::Z).lerp(Vec3::Z, cover.clamp(0.0, 1.0));
    let t = Vec3::new(-dx * PAINT_BUMP, -dy * PAINT_BUMP, 1.0).normalize();
    Some(Vec3::new(b.x + t.x, b.y + t.y, b.z).normalize_or(Vec3::Z))
}
