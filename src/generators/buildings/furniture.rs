//! Furniture, part one: seating. Dining chair, office chair, armchair,
//! bar stool, sofa and sectional sofa. Floor at z = 0, the seat faces -Y.

use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::DiningChair => dining_chair(b),
        K::OfficeChair => office_chair(b),
        K::Armchair => armchair(b, hx, hy),
        K::BarStool => bar_stool(b),
        K::Sofa => sofa(b, hx, hy),
        K::SectionalSofa => sectional(b, hx, hy),
        _ => {}
    }
}

/// A seeded upholstery colour.
pub(super) fn fabric(b: &mut Ctx) -> u16 {
    [ids::ROOF, ids::SLATE, ids::STUCCO, ids::PLASTER][b.r.gen_range(0..4)]
}

/// A soft box from `min` to `max`. At Mid detail its corners are rounded
/// in plan and the top edge is softened.
pub(super) fn cushion(k: &mut Kit, min: Vec3, max: Vec3) {
    if !k.mid {
        k.block(min, max);
        return;
    }
    let r = ((max - min).min_element() * 0.25).min(0.06);
    let (c, h) = ((min + max) * 0.5, (max - min) * 0.5);
    let rounded = |hx: f32, hy: f32, r: f32| -> Vec<Vec2> {
        let mut pts = Vec::new();
        for (q, s) in [(0, Vec2::new(1.0, 1.0)), (1, Vec2::new(-1.0, 1.0)), (2, Vec2::new(-1.0, -1.0)), (3, Vec2::new(1.0, -1.0))] {
            let o = Vec2::new(c.x, c.y) + Vec2::new(hx - r, hy - r) * s;
            for i in 0..=3 {
                let a = q as f32 * FRAC_PI_2 + i as f32 * FRAC_PI_2 / 3.0;
                pts.push(o + Vec2::from_angle(a) * r);
            }
        }
        pts
    };
    k.prism(&rounded(h.x, h.y, r), min.z, h.z * 2.0 - r, false, true);
    k.prism(&rounded(h.x - r * 0.4, h.y - r * 0.4, r * 0.6), max.z - r, r, true, false);
}

/// Four straight legs under a rectangle, inset by `inset`.
pub(super) fn legs(k: &mut Kit, hx: f32, hy: f32, inset: f32, h: f32, t: f32) {
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let p = Vec2::new(x * (hx - inset), y * (hy - inset));
        k.block((p - Vec2::splat(t)).extend(0.0), (p + Vec2::splat(t)).extend(h));
    }
}

fn dining_chair(b: &mut Ctx) {
    // Wooden chair: four legs, a seat pad, and a slatted or solid back.
    let pad = fabric(b);
    let slats = b.r.gen_bool(0.5);
    let k = &mut b.kit;
    k.mat(ids::TRIM);
    legs(k, 0.22, 0.22, 0.02, 0.45, 0.02);
    k.block(Vec3::new(-0.22, -0.22, 0.42), Vec3::new(0.22, 0.22, 0.46));
    for x in [-0.2, 0.2] {
        k.block(Vec3::new(x - 0.02, 0.18, 0.46), Vec3::new(x + 0.02, 0.22, 0.95));
    }
    k.block(Vec3::new(-0.22, 0.18, 0.85), Vec3::new(0.22, 0.22, 0.95));
    if slats {
        for x in [-0.1, 0.0, 0.1] {
            k.block(Vec3::new(x - 0.02, 0.19, 0.5), Vec3::new(x + 0.02, 0.21, 0.85));
        }
    } else {
        k.block(Vec3::new(-0.18, 0.19, 0.6), Vec3::new(0.18, 0.21, 0.82));
    }
    k.mat(pad);
    cushion(k, Vec3::new(-0.2, -0.2, 0.46), Vec3::new(0.2, 0.18, 0.5));
}

fn office_chair(b: &mut Ctx) {
    // Five-star base on casters, gas lift, seat, mesh back and armrests.
    let pad = fabric(b);
    let k = &mut b.kit;
    k.mat(ids::IRON);
    for i in 0..5 {
        let d = Vec2::from_angle(i as f32 * TAU / 5.0 + FRAC_PI_2);
        k.beam(Vec3::new(0.0, 0.0, 0.1), (d * 0.3).extend(0.07), 0.04, 0.04);
        k.mat(ids::ASPHALT).frustum((d * 0.3).extend(0.0), 0.03, 0.03, 0.05, 6, true);
        k.mat(ids::IRON);
    }
    k.frustum(Vec3::new(0.0, 0.0, 0.08), 0.03, 0.025, 0.36, 8, true);
    k.mat(pad);
    cushion(k, Vec3::new(-0.25, -0.24, 0.44), Vec3::new(0.25, 0.22, 0.52));
    k.mat(ids::IRON).block(Vec3::new(-0.03, 0.2, 0.46), Vec3::new(0.03, 0.26, 0.62));
    k.mat(ids::ASPHALT);
    cushion(k, Vec3::new(-0.23, 0.24, 0.6), Vec3::new(0.23, 0.29, 1.1));
    k.mat(ids::IRON);
    for x in [-0.27, 0.27] {
        k.block(Vec3::new(x - 0.015, -0.02, 0.5), Vec3::new(x + 0.015, 0.02, 0.68));
        k.block(Vec3::new(x - 0.03, -0.14, 0.68), Vec3::new(x + 0.03, 0.08, 0.7));
    }
}

fn armchair(b: &mut Ctx, hx: f32, hy: f32) {
    // A boxy club armchair with thick arms, a seat cushion and a back
    // cushion on short legs.
    let pad = fabric(b);
    let (w, d) = (hx.min(0.5), hy.min(0.45));
    let k = &mut b.kit;
    k.mat(ids::TRIM);
    legs(k, w, d, 0.06, 0.1, 0.02);
    k.mat(pad);
    cushion(k, Vec3::new(-w, -d, 0.1), Vec3::new(w, d, 0.38));
    for s in [-1.0f32, 1.0] {
        let x = s * (w - 0.08);
        cushion(k, Vec3::new(x - 0.08, -d, 0.38), Vec3::new(x + 0.08, d, 0.65));
    }
    cushion(k, Vec3::new(-w + 0.16, d - 0.2, 0.38), Vec3::new(w - 0.16, d, 0.9));
    cushion(k, Vec3::new(-w + 0.17, -d + 0.02, 0.38), Vec3::new(w - 0.17, d - 0.2, 0.5));
}

fn bar_stool(b: &mut Ctx) {
    // A tall round stool: pedestal or four legs (by seed), a foot ring
    // and a padded top.
    let pad = fabric(b);
    let pedestal = b.r.gen_bool(0.5);
    let k = &mut b.kit;
    k.mat(ids::IRON);
    if pedestal {
        k.frustum(Vec3::ZERO, 0.22, 0.2, 0.03, 16, true);
        k.frustum(Vec3::new(0.0, 0.0, 0.03), 0.03, 0.03, 0.7, 8, true);
        for i in 0..12 {
            let (a, c) = (Vec2::from_angle(i as f32 * TAU / 12.0) * 0.17, Vec2::from_angle((i + 1) as f32 * TAU / 12.0) * 0.17);
            k.beam(a.extend(0.3), c.extend(0.3), 0.02, 0.02);
        }
        for i in 0..3 {
            let d = Vec2::from_angle(i as f32 * TAU / 3.0);
            k.beam((d * 0.03).extend(0.3), (d * 0.17).extend(0.3), 0.015, 0.015);
        }
    } else {
        k.mat(ids::TRIM);
        for i in 0..4 {
            let d = Vec2::from_angle(i as f32 * FRAC_PI_2 + FRAC_PI_2 * 0.5);
            k.beam((d * 0.2).extend(0.02), (d * 0.12).extend(0.74), 0.03, 0.03);
        }
        for i in 0..4 {
            let (a, c) = (Vec2::from_angle(i as f32 * FRAC_PI_2 + FRAC_PI_2 * 0.5), Vec2::from_angle((i + 1) as f32 * FRAC_PI_2 + FRAC_PI_2 * 0.5));
            k.beam((a * 0.17).extend(0.28), (c * 0.17).extend(0.28), 0.02, 0.02);
        }
    }
    k.mat(pad).frustum(Vec3::new(0.0, 0.0, 0.73), 0.19, 0.18, 0.06, 16, true);
}

fn sofa(b: &mut Ctx, hx: f32, hy: f32) {
    // A three-seat sofa: base, arms, back, a seat cushion per seat and a
    // couple of scatter cushions.
    let pad = fabric(b);
    let accent = fabric(b);
    let d = hy.min(0.48);
    let k = &mut b.kit;
    k.mat(ids::TRIM);
    legs(k, hx, d, 0.06, 0.1, 0.025);
    k.mat(pad);
    cushion(k, Vec3::new(-hx, -d, 0.1), Vec3::new(hx, d, 0.32));
    for s in [-1.0f32, 1.0] {
        let x = s * (hx - 0.1);
        cushion(k, Vec3::new(x - 0.1, -d, 0.32), Vec3::new(x + 0.1, d, 0.62));
    }
    cushion(k, Vec3::new(-hx + 0.2, d - 0.22, 0.32), Vec3::new(hx - 0.2, d, 0.85));
    let n = (((hx - 0.2) * 2.0 / 0.6) as usize).clamp(1, 4);
    let sw = (hx - 0.2) * 2.0 / n as f32;
    for i in 0..n {
        let x0 = -hx + 0.2 + i as f32 * sw;
        cushion(k, Vec3::new(x0 + 0.01, -d + 0.02, 0.32), Vec3::new(x0 + sw - 0.01, d - 0.22, 0.45));
        cushion(k, Vec3::new(x0 + 0.02, d - 0.34, 0.45), Vec3::new(x0 + sw - 0.02, d - 0.22, 0.8));
    }
    k.mat(accent);
    for s in [-1.0f32, 1.0] {
        let p = Vec3::new(s * (hx - 0.4), d - 0.42, 0.62);
        k.at(Mat4::from_translation(p) * Mat4::from_rotation_z(s * 0.3) * Mat4::from_rotation_x(-0.3), |k| cushion(k, Vec3::new(-0.2, -0.06, -0.17), Vec3::new(0.2, 0.06, 0.17)));
    }
}

fn sectional(b: &mut Ctx, hx: f32, hy: f32) {
    // An L-shaped sofa: a long run along the back (+Y) and a chaise
    // returning towards the front on the left, plus an end arm.
    let pad = fabric(b);
    let d = 0.48;
    let k = &mut b.kit;
    let (bx, by) = (-hx + d, hy - d);
    k.mat(ids::TRIM);
    for (x, y) in [(-hx, -hy), (bx, -hy), (-hx, hy), (hx, hy), (hx, by - d), (bx, by - d)] {
        let p = Vec2::new(x.clamp(-hx + 0.08, hx - 0.08), y.clamp(-hy + 0.08, hy - 0.08));
        k.block((p - Vec2::splat(0.025)).extend(0.0), (p + Vec2::splat(0.025)).extend(0.1));
    }
    k.mat(pad);
    // Back run along +Y and the chaise along -X.
    cushion(k, Vec3::new(-hx, by - d, 0.1), Vec3::new(hx, hy, 0.32));
    cushion(k, Vec3::new(-hx, -hy, 0.1), Vec3::new(bx, by - d, 0.32));
    cushion(k, Vec3::new(-hx + 0.2, hy - 0.22, 0.32), Vec3::new(hx - 0.2, hy, 0.85));
    cushion(k, Vec3::new(-hx, -hy + 0.2, 0.32), Vec3::new(-hx + 0.22, hy - 0.22, 0.85));
    cushion(k, Vec3::new(hx - 0.2, by - d, 0.32), Vec3::new(hx, hy, 0.62));
    let n = (((hx * 2.0 - 0.6) / 0.65) as usize).clamp(1, 6);
    let sw = (hx * 2.0 - 0.42) / n as f32;
    for i in 0..n {
        let x0 = -hx + 0.22 + i as f32 * sw;
        cushion(k, Vec3::new(x0 + 0.01, by - d + 0.02, 0.32), Vec3::new(x0 + sw - 0.01, hy - 0.22, 0.45));
    }
    cushion(k, Vec3::new(-hx + 0.22, -hy + 0.02, 0.32), Vec3::new(bx - 0.02, by - d - 0.01, 0.45));
    k.block(Vec3::new(-hx, -hy, 0.32), Vec3::new(bx, -hy + 0.18, 0.5));
}
