//! Temple and cemetery prefabs. Ground at z = 0, entrance facing -Y.

use super::houses::door;
use super::services::tree;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::ids;
use bevy::math::{Mat4, Vec3};
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Temple => temple(b, hx, hy),
        K::Cemetery => cemetery(b, hx, hy),
        _ => {}
    }
}

fn temple(b: &mut Ctx, hx: f32, hy: f32) {
    // A stone platform with steps, a columned hall under two tiers of
    // wide hipped roofs, a gate and lanterns along the path.
    let (base, y0) = (1.2, -hy + 7.0);
    b.kit.mat(ids::COBBLE).block(Vec3::new(-1.6, -hy, 0.0), Vec3::new(1.6, y0, 0.08));
    let (px, py) = (hx * 0.85, (hy - y0) * 0.5);
    let c = Vec3::new(0.0, y0 + py, 0.0);
    b.kit.mat(ids::CURB).block(c - Vec3::new(px, py, 0.0), c + Vec3::new(px, py, base));
    let k = &mut b.kit;
    k.at(Mat4::from_translation(Vec3::new(0.0, y0 - 1.96, 0.0)), |k| {
        super::arch::stairs(k, super::StairKind::Straight, 4.0, base);
    });
    let (hx2, hy2, h) = (px - 2.0, py - 2.0, 5.0);
    k.mat(ids::PLASTER).block(c + Vec3::new(-hx2 + 1.0, -hy2 + 1.0, base), c + Vec3::new(hx2 - 1.0, hy2 - 1.0, base + h));
    k.mat(ids::BRICK);
    let n = ((hx2 * 2.0 / 2.6) as usize).max(3);
    for i in 0..=n {
        let x = -hx2 + 2.0 * hx2 * i as f32 / n as f32;
        for y in [-hy2, hy2] {
            k.frustum(c + Vec3::new(x, y, base), 0.25, 0.25, h, 8, true);
        }
    }
    door(k, 0.0, c.y - hy2 + 0.98, base, 2.4, 3.2);
    k.at(Mat4::from_translation(c + Vec3::Z * (base + h)), |k| k.mat(ids::SLATE).hipped(hx2, hy2, 2.4, 1.8));
    k.mat(ids::PLASTER).block(c + Vec3::new(-hx2 * 0.55, -hy2 * 0.55, base + h + 1.2), c + Vec3::new(hx2 * 0.55, hy2 * 0.55, base + h + 3.4));
    k.at(Mat4::from_translation(c + Vec3::Z * (base + h + 3.4)), |k| k.mat(ids::SLATE).hipped(hx2 * 0.55, hy2 * 0.55, 2.4, 1.4));
    // Gate at the start of the path.
    let gy = -hy + 1.5;
    k.mat(ids::BRICK);
    for s in [-1.0, 1.0] {
        k.frustum(Vec3::new(s * 2.4, gy, 0.0), 0.25, 0.22, 5.0, 10, true);
    }
    k.block(Vec3::new(-3.4, gy - 0.3, 5.0), Vec3::new(3.4, gy + 0.3, 5.5));
    k.block(Vec3::new(-2.8, gy - 0.2, 4.0), Vec3::new(2.8, gy + 0.2, 4.3));
    let glow = b.glow(true);
    for i in 0..3 {
        let y = gy + 2.0 + i as f32 * (y0 - gy - 3.0) / 2.0;
        for s in [-1.0, 1.0] {
            let p = Vec3::new(s * 2.4, y, 0.0);
            let k = &mut b.kit;
            k.mat(ids::CURB).block(p - Vec3::new(0.2, 0.2, 0.0), p + Vec3::new(0.2, 0.2, 1.0));
            k.mat(glow).block(p + Vec3::new(-0.25, -0.25, 1.0), p + Vec3::new(0.25, 0.25, 1.5));
            k.mat(ids::CURB).frustum(p + Vec3::Z * 1.5, 0.45, 0.0, 0.4, 4, true);
        }
    }
}

fn cemetery(b: &mut Ctx, hx: f32, hy: f32) {
    // A walled lawn with rows of headstones either side of a path, trees
    // and a small chapel at the back.
    b.kit.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.06));
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-1.2, -hy, 0.0), Vec3::new(1.2, hy - 8.0, 0.1));
    let k = &mut b.kit;
    k.mat(ids::CURB);
    k.block(Vec3::new(-hx, hy - 0.4, 0.0), Vec3::new(hx, hy, 1.2));
    k.block(Vec3::new(-hx, -hy, 0.0), Vec3::new(-hx + 0.4, hy, 1.2));
    k.block(Vec3::new(hx - 0.4, -hy, 0.0), Vec3::new(hx, hy, 1.2));
    k.block(Vec3::new(-hx, -hy, 0.0), Vec3::new(-2.0, -hy + 0.4, 1.2));
    k.block(Vec3::new(2.0, -hy, 0.0), Vec3::new(hx, -hy + 0.4, 1.2));
    for s in [-1.0, 1.0] {
        k.block(Vec3::new(s * 2.0 - 0.3, -hy, 0.0), Vec3::new(s * 2.0 + 0.3, -hy + 0.6, 2.2));
    }
    let rows = (((2.0 * hy - 12.0) / 2.4) as usize).max(1);
    let cols = (((hx - 3.0) / 1.6) as usize).max(1);
    for j in 0..rows {
        let y = -hy + 2.5 + j as f32 * 2.4;
        for s in [-1.0f32, 1.0] {
            for i in 0..cols {
                if b.r.gen_bool(0.12) {
                    continue;
                }
                let x = s * (2.5 + i as f32 * 1.6);
                let (w, h) = (b.r.gen_range(0.5..0.8), b.r.gen_range(0.6..1.2));
                let tilt = b.r.gen_range(-0.08..0.08);
                let cross = b.r.gen_bool(0.15);
                b.kit.at(Mat4::from_translation(Vec3::new(x, y, 0.06)) * Mat4::from_rotation_x(tilt), |k| {
                    k.mat(ids::CURB);
                    if cross {
                        k.block(Vec3::new(-0.08, -0.08, 0.0), Vec3::new(0.08, 0.08, h + 0.4));
                        k.block(Vec3::new(-0.35, -0.08, h - 0.1), Vec3::new(0.35, 0.08, h + 0.06));
                    } else {
                        k.block(Vec3::new(-w * 0.5, -0.1, 0.0), Vec3::new(w * 0.5, 0.1, h));
                    }
                });
            }
        }
    }
    for i in 0..4 {
        let p = Vec3::new(if i % 2 == 0 { -hx + 2.0 } else { hx - 2.0 }, -hy + 4.0 + (i / 2) as f32 * hy, 0.06);
        tree(&mut b.kit, p, 8.0);
    }
    let c = Vec3::new(0.0, hy - 4.5, 0.0);
    let k = &mut b.kit;
    k.mat(ids::CURB).block(c - Vec3::new(2.5, 3.5, 0.0), c + Vec3::new(2.5, 3.5, 4.0));
    k.at(Mat4::from_translation(c + Vec3::Z * 4.0) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2), |k| k.mat(ids::SLATE).gable(3.5, 2.5, 2.5, 0.3, ids::CURB));
    door(k, 0.0, c.y - 3.52, 0.06, 1.4, 2.6);
}
