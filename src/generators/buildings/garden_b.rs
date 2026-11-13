//! Garden & lot details, part two: garden shed, doghouse, washing line,
//! wheelie bins, garden gnome, hose reel, backyard pool and porch steps.
//! Ground at z = 0, front facing -Y; the width runs along X.

use super::houses::door;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::ids;
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::GardenShed => shed(b, hx, hy),
        K::Doghouse => doghouse(b, hx, hy),
        K::WashingLine => washing(b, hx, hy),
        K::WheelieBins => bins(b, hx),
        K::GardenGnome => gnome(b),
        K::HoseReel => hose(b),
        K::BackyardPool => pool(b, hx, hy),
        K::PorchSteps => porch(b, hx, hy),
        _ => {}
    }
}

fn shed(b: &mut Ctx, hx: f32, hy: f32) {
    // A timber shed with a pent or gable roof, a door, a window and a
    // concrete base.
    let h = 2.0;
    let k = &mut b.kit;
    k.mat(ids::CURB).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.1));
    let (sx, sy) = (hx - 0.1, hy - 0.1);
    k.mat(ids::TRIM).block(Vec3::new(-sx, -sy, 0.1), Vec3::new(sx, sy, h));
    for i in 0..((h / 0.15) as usize) {
        let z = 0.1 + i as f32 * 0.15;
        k.mat(ids::DUST).block(Vec3::new(-sx - 0.01, -sy - 0.01, z), Vec3::new(sx + 0.01, -sy + 0.01, z + 0.02));
    }
    if b.r.gen_bool(0.5) {
        b.kit.at(Mat4::from_translation(Vec3::Z * h) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::SLATE).gable(sy, sx, 0.7, 0.15, ids::TRIM));
    } else {
        let k = &mut b.kit;
        k.mat(ids::TRIM).poly(&[Vec3::new(-sx, -sy, h), Vec3::new(-sx, sy, h), Vec3::new(-sx, sy, h + 0.4)]);
        k.poly(&[Vec3::new(sx, sy, h), Vec3::new(sx, -sy, h), Vec3::new(sx, sy, h + 0.4)]);
        k.block(Vec3::new(-sx, sy - 0.02, h), Vec3::new(sx, sy, h + 0.4));
        let (a, c) = (Vec3::new(0.0, -sy - 0.2, h - 0.05), Vec3::new(0.0, sy + 0.2, h + 0.45));
        k.mat(ids::SLATE).quad(a - Vec3::X * (sx + 0.15), a + Vec3::X * (sx + 0.15), c + Vec3::X * (sx + 0.15), c - Vec3::X * (sx + 0.15));
        k.quad(c - Vec3::X * (sx + 0.15), c + Vec3::X * (sx + 0.15), a + Vec3::X * (sx + 0.15), a - Vec3::X * (sx + 0.15));
    }
    let k = &mut b.kit;
    door(k, -sx * 0.35, -sy - 0.03, 0.1, 0.8, 1.8);
    window(k, sx * 0.45, -sy - 0.03, 1.0, 0.6, 0.5, ids::GLASS);
}

fn doghouse(b: &mut Ctx, hx: f32, hy: f32) {
    // A kennel with an arched opening, a gable roof, a bowl and a name
    // board.
    let (w, d, h) = (hx.min(0.7), hy.min(0.8), 0.7);
    let paint = [ids::ROOF, ids::STUCCO, ids::TRIM][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    k.mat(paint).block(Vec3::new(-w, -d, 0.0), Vec3::new(w, d, h));
    k.at(Mat4::from_translation(Vec3::Z * h) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::SLATE).gable(d, w, 0.45, 0.08, paint));
    k.mat(ids::ASPHALT).quad(Vec3::new(-0.22, -d - 0.01, 0.05), Vec3::new(0.22, -d - 0.01, 0.05), Vec3::new(0.22, -d - 0.01, 0.42), Vec3::new(-0.22, -d - 0.01, 0.42));
    k.at(Mat4::from_translation(Vec3::new(0.0, -d - 0.011, 0.42)) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.frustum(Vec3::ZERO, 0.22, 0.22, 0.0, 10, true));
    k.mat(ids::PLASTER).block(Vec3::new(-0.15, -d - 0.02, h + 0.05), Vec3::new(0.15, -d - 0.01, h + 0.15));
    k.mat(ids::IRON).frustum(Vec3::new(w * 0.6, -d - 0.35, 0.0), 0.12, 0.1, 0.06, 10, true);
}

fn washing(b: &mut Ctx, hx: f32, hy: f32) {
    // A line between two T-posts with clothes pegged out, or a rotary
    // dryer (by seed).
    let k = &mut b.kit;
    let colours = [ids::PLASTER, ids::ROOF, ids::SLATE, ids::STUCCO];
    if b.r.gen_bool(0.5) {
        let x = hx - 0.2;
        k.mat(ids::IRON);
        for s in [-x, x] {
            k.block(Vec3::new(s - 0.03, -0.03, 0.0), Vec3::new(s + 0.03, 0.03, 1.9));
            k.block(Vec3::new(s - 0.03, -0.4, 1.8), Vec3::new(s + 0.03, 0.4, 1.86));
        }
        for y in [-0.35, 0.35] {
            k.beam(Vec3::new(-x, y, 1.8), Vec3::new(x, y, 1.8), 0.01, 0.01);
        }
        let n = ((x * 2.0 / 0.7) as usize).max(1);
        for i in 0..n {
            let (cx, y) = (-x + 0.4 + i as f32 * 0.7, if i % 2 == 0 { -0.35 } else { 0.35 });
            let (w, h) = (b.r.gen_range(0.25..0.35), b.r.gen_range(0.4..0.7));
            b.kit.mat(colours[i % 4]).block(Vec3::new(cx - w, y - 0.01, 1.8 - h), Vec3::new(cx + w, y + 0.01, 1.79));
        }
    } else {
        let r = hx.min(hy).min(1.6);
        k.mat(ids::IRON).block(Vec3::new(-0.03, -0.03, 0.0), Vec3::new(0.03, 0.03, 1.8));
        for (i, &paint) in colours.iter().enumerate() {
            let a = i as f32 * TAU / 4.0;
            let d = Vec3::new(a.cos(), a.sin(), 0.0);
            k.mat(ids::IRON).beam(Vec3::Z * 1.4, d * r + Vec3::Z * 1.75, 0.025, 0.025);
            let side = Vec3::new(-d.y, d.x, 0.0);
            let p = d * r * 0.6 + Vec3::Z * 1.7;
            k.mat(paint).block(p - side * 0.3 - Vec3::Z * 0.55 - d * 0.01, p + side * 0.3 + d * 0.01);
        }
        for f in [0.45, 0.75, 1.0] {
            k.mat(ids::IRON).frustum(Vec3::Z * (1.4 + 0.35 * f), r * f, r * f, 0.01, 4, false);
        }
    }
}

fn bins(b: &mut Ctx, hx: f32) {
    // A row of wheeled bins with hinged lids in different colours.
    let n = ((hx * 2.0 / 0.7) as usize).clamp(1, 4);
    let colours = [ids::SLATE, ids::STUCCO, ids::ASPHALT, ids::RUST];
    for i in 0..n {
        let x = -hx + 0.35 + i as f32 * 0.7;
        let open = b.r.gen_bool(0.2);
        let k = &mut b.kit;
        let c = colours[(i + b.p.seed as usize) % 4];
        k.mat(c).cuboid(Vec3::new(x, 0.0, 0.53), Vec3::new(0.29, 0.3, 0.48), bevy::math::Quat::IDENTITY);
        let lid = if open { Mat4::from_translation(Vec3::new(x, 0.33, 1.02)) * Mat4::from_rotation_x(-1.9) } else { Mat4::from_translation(Vec3::new(x, 0.33, 1.02)) };
        k.at(lid, |k| k.block(Vec3::new(-0.31, -0.68, 0.0), Vec3::new(0.31, 0.0, 0.05)));
        k.mat(ids::ASPHALT);
        for s in [-0.22, 0.22] {
            k.at(Mat4::from_translation(Vec3::new(x + s, 0.3, 0.1)) * Mat4::from_rotation_z(FRAC_PI_2) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.frustum(Vec3::Z * -0.03, 0.1, 0.1, 0.06, 8, true));
        }
    }
}

fn gnome(b: &mut Ctx) {
    // A garden gnome: boots, a round body, a beard, a face and a tall
    // pointed hat, maybe holding a fishing rod.
    let hat = [ids::ROOF, ids::SLATE, ids::STUCCO][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    k.mat(ids::CURB).frustum(Vec3::ZERO, 0.15, 0.15, 0.03, 10, true);
    k.mat(ids::TRIM).block(Vec3::new(-0.08, -0.08, 0.03), Vec3::new(0.08, 0.04, 0.08));
    k.mat(ids::SLATE).frustum(Vec3::Z * 0.08, 0.1, 0.08, 0.12, 10, true);
    k.mat(hat).frustum(Vec3::Z * 0.2, 0.1, 0.08, 0.1, 10, true);
    k.mat(ids::PLASTER).frustum(Vec3::new(0.0, -0.04, 0.22), 0.06, 0.02, 0.1, 8, true);
    k.mat(ids::STUCCO).frustum(Vec3::Z * 0.3, 0.06, 0.055, 0.07, 10, true);
    k.mat(hat).frustum(Vec3::Z * 0.37, 0.075, 0.0, 0.16, 10, true);
    if b.r.gen_bool(0.5) {
        b.kit.mat(ids::TRIM).beam(Vec3::new(0.08, -0.05, 0.2), Vec3::new(0.25, -0.25, 0.55), 0.01, 0.01);
    }
}

fn hose(b: &mut Ctx) {
    // A wall-mounted or cart hose reel: a drum wound with hose, a crank,
    // and a nozzle on the ground.
    let k = &mut b.kit;
    let c = Vec3::Z * 0.4;
    k.mat(ids::SLATE);
    for x in [-0.22, 0.22] {
        k.beam(Vec3::new(x, -0.25, 0.03), c + Vec3::X * x, 0.03, 0.03);
        k.beam(Vec3::new(x, 0.25, 0.03), c + Vec3::X * x, 0.03, 0.03);
        k.at(Mat4::from_translation(c + Vec3::X * x) * Mat4::from_rotation_y(FRAC_PI_2), |k| k.frustum(Vec3::Z * -0.01, 0.25, 0.25, 0.02, 14, true));
    }
    k.mat(ids::RUST).at(Mat4::from_translation(c) * Mat4::from_rotation_y(FRAC_PI_2), |k| k.frustum(Vec3::Z * -0.2, 0.17, 0.17, 0.4, 14, true));
    k.mat(ids::IRON).beam(c + Vec3::X * 0.25, c + Vec3::new(0.32, 0.0, 0.0), 0.02, 0.02);
    k.beam(c + Vec3::X * 0.32, c + Vec3::new(0.32, 0.0, -0.12), 0.02, 0.02);
    k.mat(ids::RUST).tube(Vec3::new(0.0, -0.15, 0.25), Vec3::new(0.1, -0.5, 0.04), 0.012, 6);
    k.mat(ids::ROOF).block(Vec3::new(0.08, -0.62, 0.0), Vec3::new(0.13, -0.5, 0.05));
}

fn pool(b: &mut Ctx, hx: f32, hy: f32) {
    // An above-ground garden pool with a ladder, or a paved in-ground
    // pool with a coping and steps (by seed).
    let k = &mut b.kit;
    if b.r.gen_bool(0.5) {
        let r = hx.min(hy) - 0.2;
        k.mat(ids::SLATE).frustum(Vec3::ZERO, r, r, 1.2, 24, false);
        k.mat(ids::PLASTER).frustum(Vec3::Z * 1.2, r + 0.08, r + 0.08, 0.06, 24, true);
        k.mat(ids::WATER).frustum(Vec3::Z * 1.05, r - 0.05, r - 0.05, 0.16, 24, true);
        let p = Vec3::new(0.0, -r - 0.35, 0.0);
        k.mat(ids::IRON);
        for x in [-0.25, 0.25] {
            k.beam(p + Vec3::new(x, -0.3, 0.03), p + Vec3::new(x, 0.25, 1.5), 0.03, 0.03);
        }
        for i in 1..4 {
            let t = i as f32 / 4.0;
            k.block(p + Vec3::new(-0.25, -0.3 + 0.55 * t - 0.03, 1.5 * t - 0.02), p + Vec3::new(0.25, -0.3 + 0.55 * t + 0.03, 1.5 * t + 0.02));
        }
    } else {
        k.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.1));
        let (px, py) = (hx - 1.0, hy - 1.0);
        k.mat(ids::CURB).block(Vec3::new(-px - 0.3, -py - 0.3, 0.1), Vec3::new(px + 0.3, py + 0.3, 0.16));
        k.mat(ids::WATER).block(Vec3::new(-px, -py, 0.1), Vec3::new(px, py, 0.17));
        for i in 0..3 {
            k.mat(ids::PLASTER).block(Vec3::new(px - 1.2, -py + i as f32 * 0.35, 0.1), Vec3::new(px, -py + 0.35 + i as f32 * 0.35, 0.165));
        }
        k.mat(ids::IRON);
        for x in [-px + 0.3, -px + 0.8] {
            k.tube(Vec3::new(x, py + 0.3, 0.1), Vec3::new(x, py + 0.3, 0.9), 0.025, 6);
            k.tube(Vec3::new(x, py + 0.3, 0.9), Vec3::new(x, py - 0.1, 0.9), 0.025, 6);
        }
    }
}

fn porch(b: &mut Ctx, hx: f32, hy: f32) {
    // A raised timber porch deck with steps down the front, posts, a
    // railing and a roof, ready to push against a house wall at +Y.
    let (h, d) = (0.72, hy * 2.0 - 1.2);
    let k = &mut b.kit;
    let y0 = -hy + 1.2;
    k.mat(ids::TRIM).block(Vec3::new(-hx, y0, 0.0), Vec3::new(hx, hy, h));
    let steps = 4;
    for i in 0..steps {
        let z = h * (i + 1) as f32 / (steps + 1) as f32;
        let y = y0 - 1.2 + i as f32 * 0.3;
        k.mat(if i % 2 == 0 { ids::TRIM } else { ids::DUST }).block(Vec3::new(-0.8, y, 0.0), Vec3::new(0.8, y0, z));
    }
    let paint = if b.r.gen_bool(0.6) { ids::PLASTER } else { ids::TRIM };
    k.mat(paint);
    for x in [-hx + 0.1, -0.9, 0.9, hx - 0.1] {
        k.block(Vec3::new(x - 0.07, y0 + 0.05, h), Vec3::new(x + 0.07, y0 + 0.19, h + 2.4));
    }
    for (a, c) in [(-hx + 0.1, -0.9), (0.9, hx - 0.1)] {
        k.block(Vec3::new(a, y0 + 0.08, h + 0.85), Vec3::new(c, y0 + 0.16, h + 0.92));
        let n = (((c - a) / 0.15) as usize).max(1);
        for i in 1..n {
            let x = a + (c - a) * i as f32 / n as f32;
            k.block(Vec3::new(x - 0.02, y0 + 0.1, h), Vec3::new(x + 0.02, y0 + 0.14, h + 0.85));
        }
    }
    let rz = h + 2.4;
    k.mat(ids::SLATE).quad(Vec3::new(-hx - 0.2, y0 - 0.3, rz), Vec3::new(hx + 0.2, y0 - 0.3, rz), Vec3::new(hx + 0.2, hy, rz + d * 0.25), Vec3::new(-hx - 0.2, hy, rz + d * 0.25));
    k.quad(Vec3::new(-hx - 0.2, hy, rz + d * 0.25), Vec3::new(hx + 0.2, hy, rz + d * 0.25), Vec3::new(hx + 0.2, y0 - 0.3, rz), Vec3::new(-hx - 0.2, y0 - 0.3, rz));
    k.mat(paint).block(Vec3::new(-hx, y0, rz - 0.2), Vec3::new(hx, y0 + 0.2, rz));
}
