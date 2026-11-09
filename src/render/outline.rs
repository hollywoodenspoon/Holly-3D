//! Universal ink outline post pass: silhouettes (object / depth breaks) and
//! optional crease lines (normal breaks), drawn for every material that has
//! `Render Ink Outline` enabled — PBR, toon and crystal alike.

use super::trace::GSample;
use crate::materials::Material;
use crate::materials::shading::srgb_to_linear;
use bevy::math::{Vec2, Vec3, Vec4};

/// True when `a` and `b` lie on different surfaces (silhouette edge).
fn silhouette(a: &GSample, b: &GSample) -> bool {
    if a.obj != b.obj {
        return true;
    }
    let near = a.depth.min(b.depth);
    near.is_finite() && (a.depth - b.depth).abs() > near * 0.03
}

/// Ink half-width (pixels) + color for the edge between two neighbours.
fn edge(a: &GSample, b: &GSample, mats: &[Material]) -> Option<(f32, [f32; 3])> {
    let sil = silhouette(a, b);
    // The line belongs to the nearer surface of the pair.
    let owner = if a.depth <= b.depth { a } else { b };
    let o = &mats.get(owner.mat as usize).filter(|m| m.outline.enabled)?.outline;
    if sil {
        return Some((o.width * 0.5, o.color));
    }
    let crease = o.crease_lines && a.obj != u32::MAX && a.normal.dot(b.normal) < o.crease_angle.to_radians().cos();
    crease.then_some((o.width * 0.25, o.color))
}

/// Ink coverage per pixel (0..1) + its sRGB color.
pub fn ink(g: &[GSample], w: usize, h: usize, mats: &[Material]) -> Vec<(f32, [f32; 3])> {
    let mut cov = vec![(0.0f32, [0.0; 3]); w * h];
    let mut splat = |mid: Vec2, half: f32, col: [f32; 3]| {
        let r = half + 0.5;
        let (x0, x1) = ((mid.x - r).floor().max(0.0) as usize, ((mid.x + r).ceil() as usize).min(w - 1));
        let (y0, y1) = ((mid.y - r).floor().max(0.0) as usize, ((mid.y + r).ceil() as usize).min(h - 1));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = (Vec2::new(x as f32, y as f32) - mid).length();
                let c = (r - d).clamp(0.0, 1.0);
                let slot = &mut cov[y * w + x];
                if c > slot.0 {
                    *slot = (c, col);
                }
            }
        }
    };
    for y in 0..h {
        for x in 0..w {
            let a = &g[y * w + x];
            let p = Vec2::new(x as f32, y as f32);
            if x + 1 < w
                && let Some((half, col)) = edge(a, &g[y * w + x + 1], mats)
            {
                splat(p + Vec2::new(0.5, 0.0), half, col);
            }
            if y + 1 < h
                && let Some((half, col)) = edge(a, &g[(y + 1) * w + x], mats)
            {
                splat(p + Vec2::new(0.0, 0.5), half, col);
            }
        }
    }
    cov
}

/// Inks `display` (straight sRGB + alpha) and `hdr` (linear) in place.
pub fn apply(g: &[GSample], w: usize, h: usize, mats: &[Material], display: &mut [Vec4], hdr: &mut [Vec4]) {
    if w == 0 || h == 0 || !mats.iter().any(|m| m.outline.enabled) {
        return;
    }
    for (i, (c, col)) in ink(g, w, h, mats).into_iter().enumerate() {
        if c <= 0.0 {
            continue;
        }
        let (s, l) = (Vec3::from_array(col), Vec3::from_array(col.map(srgb_to_linear)));
        display[i] = display[i].truncate().lerp(s, c).extend(display[i].w.max(c));
        hdr[i] = hdr[i].truncate().lerp(l, c).extend(hdr[i].w.max(c));
    }
}
