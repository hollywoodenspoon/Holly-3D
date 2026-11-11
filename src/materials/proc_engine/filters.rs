//! Image filters over baked grids: Slope Blur, Normal From Height and
//! cavity Ambient Occlusion.
//!
//! Ported from MaterialCraft (https://github.com/storytold/materialcraft:
//! `slope_blur` and `ambient_occlusion` in `filters.rs`,
//! `normal_from_height_hq` in `filters_extra.rs`, before the old procedural
//! library was removed), copyright (c) 2026 ArtCraft Team and the
//! MaterialCraft contributors, MIT OR Apache-2.0.

use super::grid::{Grid, gaussian, par_map};
use bevy::math::{Vec2, Vec3, Vec4};

/// Slope (dx, dy) of a grid's luminance, lightly smoothed (Sobel), in value
/// units per pixel.
fn slope(g: &Grid, x: i64, y: i64) -> (f32, f32) {
    let h = |i: i64, j: i64| g.gray(x + i, y + j);
    let dx = (h(1, -1) + 2.0 * h(1, 0) + h(1, 1) - h(-1, -1) - 2.0 * h(-1, 0) - h(-1, 1)) / 8.0;
    let dy = (h(-1, 1) + 2.0 * h(0, 1) + h(1, 1) - h(-1, -1) - 2.0 * h(0, -1) - h(1, -1)) / 8.0;
    (dx, dy)
}

/// Slope blur: smears `img` downhill along the slopes of `slope_map`, which
/// drags colour into crevices (worn edges, drips, eroded stone).
/// `intensity` is the smear length in 1/256ths of the tile.
pub fn slope_blur(img: &Grid, slope_map: &Grid, intensity: f32, samples: u32) -> Grid {
    let n = samples.clamp(1, 64);
    let step = intensity / 256.0 / n as f32 * 4.0;
    let size = slope_map.size as f32;
    Grid::bake(img.size, |uv| {
        let mut p = uv;
        let mut acc = img.sample(uv);
        for _ in 0..n {
            let (dx, dy) = slope(slope_map, (p.x * size).floor() as i64, (p.y * size).floor() as i64);
            let l = (dx * dx + dy * dy).sqrt().max(1e-6);
            p -= Vec2::new(dx, dy) / l * step;
            acc += img.sample(p);
        }
        acc / (n + 1) as f32
    })
}

/// Packs a tangent-space normal into a normal-map colour (0..1).
pub fn pack_normal(n: Vec3) -> Vec4 {
    (n.normalize_or(Vec3::Z) * 0.5 + Vec3::splat(0.5)).extend(1.0)
}

/// A normal-map colour back to a tangent-space normal.
pub fn unpack_normal(c: Vec4) -> Vec3 {
    (c.truncate() * 2.0 - Vec3::ONE).normalize_or(Vec3::Z)
}

/// Tangent-space normal map from a height map with a 5 × 5
/// smoothed-derivative (Sobel-like) kernel, which stays crisp on edges and
/// quiet on noisy heights. `intensity` 1 suits a 0..1 height map.
pub fn normal_from_height(h: &Grid, intensity: f32) -> Grid {
    const SMOOTH: [f32; 5] = [1.0, 4.0, 6.0, 4.0, 1.0];
    const DERIV: [f32; 5] = [-1.0, -2.0, 0.0, 2.0, 1.0];
    // The kernels sum to 128 × the slope of a linear ramp.
    let k = intensity * h.size as f32 / 32.0 / 128.0;
    let n = h.size as i64;
    let data = par_map((n * n) as usize, |i| {
        let (x, y) = (i as i64 % n, i as i64 / n);
        let (mut gx, mut gy) = (0.0, 0.0);
        for j in 0..5 {
            for i in 0..5 {
                let v = h.gray(x + i as i64 - 2, y + j as i64 - 2);
                gx += SMOOTH[j] * DERIV[i] * v;
                gy += DERIV[j] * SMOOTH[i] * v;
            }
        }
        pack_normal(Vec3::new(-gx * k, -gy * k, 1.0)).to_array()
    });
    Grid { size: h.size, data }
}

/// Cavity ambient occlusion from a `size` × `size` height field: 1 in the
/// open, darker where the surroundings stand higher (joints, cracks, pits).
/// `spread` is the search radius as a fraction of the tile.
pub fn cavity_ao(size: u32, h: &[f32], spread: f32, intensity: f32) -> Vec<f32> {
    let s = spread.max(0.001) * size as f32 * 0.25;
    let (wide, mid) = (gaussian(size, h, s), gaussian(size, h, s * 0.35));
    (0..h.len()).map(|i| (1.0 - ((wide[i] - h[i]).max(0.0) * 0.6 + (mid[i] - h[i]).max(0.0) * 0.4) * intensity * 6.0).clamp(0.0, 1.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp() -> Grid {
        // Height rises with U (a sine, so the grid still tiles).
        Grid::bake(64, |uv| Vec4::splat((uv.x * std::f32::consts::TAU).sin() * 0.5 + 0.5))
    }

    #[test]
    fn normals_lean_away_from_rising_height_and_flat_is_straight_up() {
        let n = normal_from_height(&ramp(), 4.0);
        // Rising towards +U at u = 0: the normal tips towards -U.
        let rising = n.texel(0, 10);
        assert!(rising.x < 0.45 && (rising.y - 0.5).abs() < 0.01 && rising.z > 0.5, "{rising}");
        assert!(n.texel(32, 10).x > 0.55, "falling height tips the other way");
        let flat = normal_from_height(&Grid::bake(16, |_| Vec4::splat(0.3)), 4.0);
        assert!(flat.data.iter().all(|p| (p[0] - 0.5).abs() < 1e-4 && (p[2] - 1.0).abs() < 1e-4));
        // More intensity tilts more, and every normal is unit length.
        assert!(normal_from_height(&ramp(), 12.0).texel(0, 10).x < rising.x);
        assert!(n.data.iter().all(|p| (unpack_normal(Vec4::from_array(*p)).length() - 1.0).abs() < 1e-3));
        assert!((unpack_normal(pack_normal(Vec3::new(0.3, -0.2, 0.9))) - Vec3::new(0.3, -0.2, 0.9).normalize()).length() < 1e-4);
    }

    #[test]
    fn cavities_are_occluded_and_open_ground_is_not() {
        // A flat field with one narrow groove along x = 32.
        let h: Vec<f32> = (0..64usize * 64).map(|i| if (i % 64).abs_diff(32) < 2 { 0.0 } else { 1.0 }).collect();
        let ao = cavity_ao(64, &h, 0.2, 1.0);
        assert!(ao[10 * 64 + 32] < 0.6, "groove: {}", ao[10 * 64 + 32]);
        assert!(ao[10 * 64 + 8] > 0.98, "open ground: {}", ao[10 * 64 + 8]);
        assert!(cavity_ao(64, &h, 0.2, 0.2)[10 * 64 + 32] > ao[10 * 64 + 32], "less intensity is lighter");
        assert!(cavity_ao(16, &[0.5; 256], 0.2, 1.0).iter().all(|a| *a == 1.0));
    }

    #[test]
    fn slope_blur_smears_along_the_slope_and_zero_intensity_changes_nothing() {
        // A bright stripe at u = 0.5, on a slope that rises with U.
        let img = Grid::bake(64, |uv| Vec4::splat(if (uv.x - 0.5).abs() < 0.05 { 1.0 } else { 0.0 }));
        let out = slope_blur(&img, &ramp(), 24.0, 16);
        assert!(out.texel(32, 8).x < 1.0);
        let outside = (0..64).filter(|x| (*x as f32 / 64.0 - 0.5).abs() > 0.08).map(|x| out.texel(x, 8).x).sum::<f32>();
        assert!(outside > 0.5, "{outside}");
        let same = slope_blur(&img, &ramp(), 0.0, 8);
        assert!(same.data.iter().zip(&img.data).all(|(a, b)| (a[0] - b[0]).abs() < 1e-4));
        let flat = slope_blur(&img, &Grid::bake(16, |_| Vec4::splat(0.5)), 24.0, 8);
        assert!(flat.data.iter().all(|p| (0.0..=1.0001).contains(&p[0])));
    }
}
