//! Settlement props: deciduous and conifer trees, streetlamps, post-and-rail
//! fences, the village well and crop rows.

use crate::generators::kit::{Kit, ids};
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// A low-poly tree `h` metres tall standing at `at`.
pub fn tree(kit: &mut Kit, at: Vec3, h: f32, conifer: bool) {
    kit.mat(ids::TRIM).frustum(at, h * 0.045, h * 0.03, h * 0.4, 5, false);
    kit.mat(ids::FOLIAGE);
    if conifer {
        for (i, z) in [0.25, 0.45, 0.65].into_iter().enumerate() {
            let r = h * (0.26 - 0.06 * i as f32);
            kit.frustum(at + Vec3::Z * h * z, r, 0.0, h * 0.4, 7, true);
        }
    } else {
        let r = h * 0.3;
        kit.frustum(at + Vec3::Z * h * 0.3, r * 0.45, r, h * 0.3, 7, false);
        kit.frustum(at + Vec3::Z * h * 0.6, r, 0.0, h * 0.4, 7, false);
    }
}

/// A random tree (conifer or deciduous) of a random height.
pub fn random_tree(kit: &mut Kit, at: Vec3, r: &mut ChaCha8Rng, scale: f32) {
    let h = r.gen_range(5.0..10.0) * scale;
    tree(kit, at, h, r.gen_bool(0.35));
}

/// Modern streetlamp at `at`, arm reaching along `dir`.
pub fn streetlamp(kit: &mut Kit, at: Vec3, dir: Vec2) {
    let d = dir.normalize_or_zero().extend(0.0);
    kit.mat(ids::IRON).frustum(at, 0.12, 0.08, 6.5, 6, true);
    let top = at + Vec3::Z * 6.4;
    kit.beam(top, top + d * 1.6, 0.1, 0.1);
    kit.mat(ids::LIT).cuboid(top + d * 1.6 - Vec3::Z * 0.15, Vec3::new(0.3, 0.3, 0.1), bevy::math::Quat::IDENTITY);
}

/// Post-and-rail fence from `a` to `b` (heights follow `h`).
pub fn fence(kit: &mut Kit, a: Vec2, b: Vec2, h: impl Fn(Vec2) -> f32) {
    let len = a.distance(b);
    let n = (len / 2.5).ceil().max(1.0) as usize;
    kit.mat(ids::TRIM);
    for i in 0..=n {
        let p = a.lerp(b, i as f32 / n as f32);
        let z = h(p);
        kit.block(p.extend(z) - Vec3::new(0.06, 0.06, 0.0), p.extend(z + 1.1) + Vec3::new(0.06, 0.06, 0.0));
    }
    for rail in [0.45, 0.9] {
        kit.beam(a.extend(h(a) + rail), b.extend(h(b) + rail), 0.05, 0.1);
    }
}

/// Fence around a closed outline.
pub fn fence_loop(kit: &mut Kit, pts: &[Vec2], h: impl Fn(Vec2) -> f32 + Copy) {
    for i in 0..pts.len() {
        fence(kit, pts[i], pts[(i + 1) % pts.len()], h);
    }
}

/// Village well: stone ring, two posts and a small gabled roof.
pub fn well(kit: &mut Kit, at: Vec3) {
    kit.mat(ids::FACADE).frustum(at, 1.0, 1.0, 0.9, 10, true);
    kit.mat(ids::ROAD).frustum(at + Vec3::Z * 0.9, 0.75, 0.75, 0.01, 10, false);
    kit.mat(ids::TRIM);
    for s in [-1.0, 1.0] {
        kit.block(at + Vec3::new(s * 0.85 - 0.08, -0.08, 0.9), at + Vec3::new(s * 0.85 + 0.08, 0.08, 2.6));
    }
    kit.beam(at + Vec3::new(-0.9, 0.0, 2.2), at + Vec3::new(0.9, 0.0, 2.2), 0.12, 0.12);
    kit.at(bevy::math::Mat4::from_translation(at + Vec3::Z * 2.6), |k| k.mat(ids::ROOF).gable(1.1, 0.7, 0.7, 0.15, ids::TRIM));
}

/// Crop rows inside a lot (local frame): long low ridges along local X.
pub fn crop_rows(kit: &mut Kit, hx: f32, hy: f32, spacing: f32) {
    kit.mat(ids::FOLIAGE);
    let n = ((2.0 * hy - spacing) / spacing).floor().max(1.0) as usize;
    for i in 0..n {
        let y = -hy + spacing * (i as f32 + 1.0);
        kit.block(Vec3::new(-hx + 0.6, y - 0.18, 0.0), Vec3::new(hx - 0.6, y + 0.18, 0.45));
    }
}
