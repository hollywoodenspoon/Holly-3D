//! The weathering layers of a procedural surface: edge wear, cavity dirt,
//! moss, scorch / rust and wetness. Each builds a mask from the height map
//! (height blend, slope blur, cavity) broken up by tileable noise, then
//! changes colour, roughness, metalness and height where the mask is set.
//!
//! The masks follow MaterialCraft's weathering nodes
//! (https://github.com/storytold/materialcraft: `weather`, `edge_wear`
//! and `dirt_mask` in `material_more.rs`, `height_blend` and `slope_blur`
//! in `filters.rs`, before the old procedural library was removed),
//! copyright (c) 2026 ArtCraft Team and the MaterialCraft contributors,
//! MIT OR Apache-2.0.

use super::adjust::height_blend;
use super::filters::slope_blur;
use super::grid::{Grid, gaussian, par_map};
use super::lattice::{fbm, ridged, smoothstep};
use super::surface::{Layers, ProcSurface};
use bevy::math::Vec3;

/// A weathering mask: `base` (0..1, where the effect belongs) broken up by
/// fractal noise and thresholded by `amount`.
fn mask(base: f32, noise: f32, amount: f32, contrast: f32) -> f32 {
    let soft = (1.0 - contrast) * 0.5 + 0.01;
    smoothstep(1.0 - amount - soft, 1.0 - amount + soft, base - (1.0 - noise))
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Applies the recipe's weathering sliders to the layers, in the order
/// things happen to a surface: it wears, collects dirt, grows moss, burns
/// or rusts, and finally gets wet.
pub fn apply(p: &ProcSurface, base_metal: f32, l: &mut Layers) {
    let n = l.size as usize;
    let uv = |i: usize| (((i % n) as f32 + 0.5) / n as f32, ((i / n) as f32 + 0.5) / n as f32);
    let s = p.seed ^ 0x3ea7;
    let any = [p.edge_wear, p.cavity_dirt, p.moss, p.scorch, p.wetness].iter().any(|a| *a > 0.0);
    if !any {
        return;
    }
    let wide = gaussian(l.size, &l.h, l.size as f32 / 48.0);

    if p.edge_wear > 0.0 {
        // Raised edges stand above their blurred surroundings.
        let raw: Vec<f32> = par_map(n * n, |i| {
            let (u, v) = uv(i);
            let edge = ((l.h[i] - wide[i]) * 5.0).clamp(0.0, 1.0);
            mask(edge * 1.6, fbm(u, v, 8.0, 5, 0.6, s ^ 1), p.edge_wear * 0.9, 0.6)
        });
        // Slope blur drags the chips downhill, so wear follows the bevels.
        let worn = slope_blur(&Grid::gray_from(l.size, &raw), &Grid::gray_from(l.size, &l.h), 3.0, 6);
        for i in 0..n * n {
            let k = worn.data[i][0].clamp(0.0, 1.0);
            let bare = if base_metal > 0.5 { Vec3::new(0.82, 0.82, 0.84) } else { (l.color[i] * 1.35 + Vec3::splat(0.08)).min(Vec3::ONE) };
            l.color[i] = l.color[i].lerp(bare, k * 0.85);
            l.rough[i] = mix(l.rough[i], if base_metal > 0.5 { 0.28 } else { (l.rough[i] + 0.1).min(1.0) }, k);
            l.metal[i] = mix(l.metal[i], base_metal.max(l.metal[i]), k);
            l.h[i] = (l.h[i] - 0.04 * k).max(0.0);
        }
    }
    if p.cavity_dirt > 0.0 {
        let dirt = Vec3::new(0.19, 0.14, 0.1);
        let k: Vec<f32> = par_map(n * n, |i| {
            let (u, v) = uv(i);
            let cavity = ((wide[i] - l.h[i]) * 5.0).clamp(0.0, 1.0);
            let noise = fbm(u, v, 6.0, 5, 0.6, s ^ 2);
            // Grime settles in the low places first, then creeps up the faces.
            let level = p.cavity_dirt * 0.85 + (noise - 0.5) * 0.4;
            height_blend(level, l.h[i], 0.7).max(mask(cavity * 1.5, noise, p.cavity_dirt, 0.5)) * (p.cavity_dirt * 3.0).min(1.0)
        });
        for (i, k) in k.into_iter().enumerate() {
            l.color[i] = l.color[i].lerp(dirt, k * 0.85);
            l.rough[i] = mix(l.rough[i], 0.95, k);
            l.metal[i] *= 1.0 - k;
        }
    }
    if p.moss > 0.0 {
        let k: Vec<(f32, f32)> = par_map(n * n, |i| {
            let (u, v) = uv(i);
            let patch = fbm(u, v, 3.0, 5, 0.55, s ^ 3);
            let level = p.moss * 0.9 + (patch - 0.5) * 0.8;
            (height_blend(level, l.h[i], 0.8) * smoothstep(0.3, 0.55, patch + p.moss * 0.5), fbm(u, v, 24.0, 3, 0.6, s ^ 4))
        });
        for (i, (m, fine)) in k.into_iter().enumerate() {
            l.color[i] = l.color[i].lerp(Vec3::new(0.15, 0.25, 0.07).lerp(Vec3::new(0.32, 0.42, 0.13), fine), m * 0.95);
            l.rough[i] = mix(l.rough[i], 0.96, m);
            l.metal[i] *= 1.0 - m;
            // Moss is a soft cushion: it fills the gap it grows in.
            l.h[i] = mix(l.h[i], (l.h[i] + 0.12).min(1.0) * (0.85 + 0.15 * fine), m);
        }
    }
    if p.scorch > 0.0 {
        let rust = base_metal > 0.5;
        let k: Vec<(f32, f32)> = par_map(n * n, |i| {
            let (u, v) = uv(i);
            let spread = ridged(u, v, 3.0, 4, 0.55, s ^ 5) * 0.6 + fbm(u, v, 5.0, 5, 0.6, s ^ 6) * 0.6;
            (mask(spread, fbm(u, v, 16.0, 4, 0.6, s ^ 7), p.scorch, 0.5), fbm(u, v, 32.0, 3, 0.6, s ^ 8))
        });
        for (i, (m, fine)) in k.into_iter().enumerate() {
            let stain = if rust { Vec3::new(0.24, 0.1, 0.05).lerp(Vec3::new(0.5, 0.24, 0.09), fine) } else { Vec3::splat(0.02 + 0.04 * fine) };
            l.color[i] = l.color[i].lerp(stain, m * 0.92);
            l.rough[i] = mix(l.rough[i], if rust { 0.9 } else { 0.97 }, m);
            l.metal[i] *= 1.0 - m;
            if rust {
                // Rust scale stands a little proud of the metal.
                l.h[i] = (l.h[i] + 0.05 * m * fine).min(1.0);
            }
        }
    }
    if p.wetness > 0.0 {
        let k: Vec<(f32, f32)> = par_map(n * n, |i| {
            let (u, v) = uv(i);
            let level = p.wetness * 0.75 + (fbm(u, v, 2.0, 4, 0.5, s ^ 9) - 0.5) * 0.35;
            (height_blend(level, l.h[i], 0.92), level)
        });
        for (i, (m, level)) in k.into_iter().enumerate() {
            // Standing water is level: it flattens whatever it covers.
            l.h[i] = mix(l.h[i], level.clamp(0.0, 1.0), m);
            l.color[i] *= 1.0 - 0.15 * p.wetness - 0.4 * m;
            l.rough[i] = mix(l.rough[i] * (1.0 - 0.3 * p.wetness), 0.03, m);
        }
    }
}
