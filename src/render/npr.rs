//! Anime / manga post passes for Toon materials: the shadow band of a
//! `Manga Screentone` material is redrawn as a 45° halftone dot grid whose
//! dots grow with darkness, over the material's lit color.

use super::sample::Pixel;
use crate::materials::{Material, ShadingModel};
use bevy::math::{Vec2, Vec3};
use std::collections::HashMap;

fn lum(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// Halftone dot coverage test: is pixel `p` inside the dot of its cell
/// when the dot covers `fill` (0..1) of the cell area?
pub fn in_dot(p: Vec2, cell: f32, fill: f32) -> bool {
    let r = Vec2::new(p.x + p.y, p.y - p.x) * std::f32::consts::FRAC_1_SQRT_2 / cell.max(1.0);
    let f = r - r.round();
    f.length() < (fill.clamp(0.0, 1.0) / std::f32::consts::PI).sqrt()
}

/// Applies screentone to Toon pixels in place.
pub fn screentone(px: &mut [Pixel], w: usize, h: usize, mats: &[Material]) {
    let tone = |mat: u32| mats.get(mat as usize).filter(|m| m.model == ShadingModel::Toon && m.toon.screentone);
    if w == 0 || h == 0 || !mats.iter().any(|m| m.model == ShadingModel::Toon && m.toon.screentone) {
        return;
    }
    // Lit band = the brightest value of each (object, material) surface.
    let mut lit: HashMap<(u32, u32), Vec3> = HashMap::new();
    for p in px.iter().filter(|p| tone(p.g.mat).is_some()) {
        let c = p.layers.geom.truncate();
        let e = lit.entry((p.g.obj, p.g.mat)).or_insert(c);
        if lum(c) > lum(*e) {
            *e = c;
        }
    }
    for (i, p) in px.iter_mut().enumerate() {
        let Some(m) = tone(p.g.mat) else { continue };
        let top = lit[&(p.g.obj, p.g.mat)];
        let c = p.layers.geom.truncate();
        let (lc, lt) = (lum(c), lum(top));
        if lt <= 1e-6 || lc > lt * 0.85 {
            continue;
        }
        let dark = 1.0 - lc / lt;
        let at = Vec2::new((i % w) as f32, (i / w) as f32);
        let shadow = top * Vec3::from_array(m.toon.shadow_tint.map(crate::materials::shading::srgb_to_linear)) * 0.45;
        let rgb = if in_dot(at, m.toon.screentone_scale, dark * 1.4) { shadow } else { top };
        p.layers.geom = (rgb * p.layers.geom.w.min(1.0)).extend(p.layers.geom.w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::trace::GSample;

    #[test]
    fn dots_grow_with_darkness() {
        let cover = |fill| (0..64 * 64).filter(|i| in_dot(Vec2::new((i % 64) as f32, (i / 64) as f32), 8.0, fill)).count() as f32 / 4096.0;
        let (a, b) = (cover(0.2), cover(0.6));
        assert!((a - 0.2).abs() < 0.08 && (b - 0.6).abs() < 0.1 && a < b, "coverage tracks fill: {a} {b}");
        assert_eq!(cover(0.0), 0.0);
    }

    #[test]
    fn shadow_band_becomes_two_tone_dots() {
        let mut m = Material { model: ShadingModel::Toon, ..Default::default() };
        m.toon.screentone = true;
        let (w, h) = (32, 16);
        let mut px: Vec<Pixel> = (0..w * h)
            .map(|i| {
                let mut p = Pixel { g: GSample { obj: 0, mat: 0, depth: 3.0, normal: Vec3::Z }, ..Pixel::EMPTY };
                let v = if i % w < 16 { 0.8 } else { 0.3 };
                p.layers.geom = Vec3::splat(v).extend(1.0);
                p
            })
            .collect();
        screentone(&mut px, w, h, std::slice::from_ref(&m));
        assert!(px.iter().filter(|p| p.layers.geom.x == 0.8).count() > w * h / 2, "lit half untouched, light gaps in the shadow");
        let shadow: Vec<f32> = px.iter().enumerate().filter(|(i, _)| i % w >= 16).map(|(_, p)| p.layers.geom.x).collect();
        assert!(shadow.iter().any(|v| *v < 0.3) && shadow.contains(&0.8), "dots + paper in the shadow band");
        m.toon.screentone = false;
        let before: Vec<_> = px.iter().map(|p| p.layers.geom).collect();
        screentone(&mut px, w, h, &[m]);
        assert!(px.iter().zip(before).all(|(p, b)| p.layers.geom == b), "off = untouched");
    }
}
