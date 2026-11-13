//! Parks and leisure prefabs, part two: plaza, monument, park pond,
//! bandstand and skate park. Ground at z = 0, front facing -Y.

use super::parks::bench;
use super::services::{flag, tree};
use super::{BuildingKind as K, Ctx, doodads};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Plaza => plaza(b, hx, hy),
        K::Monument => monument(b, hx, hy),
        K::ParkPond => pond(b, hx, hy),
        K::Bandstand => bandstand(b, hx, hy),
        K::SkatePark => skate(b, hx, hy),
        _ => {}
    }
}

/// A round planter with a tree at `p`.
fn tree_bed(k: &mut Kit, p: Vec3, h: f32) {
    k.mat(ids::CURB).frustum(p, 1.3, 1.3, 0.5, 12, true);
    k.mat(ids::DUST).frustum(p + Vec3::Z * 0.5, 1.15, 1.15, 0.02, 12, true);
    tree(k, p + Vec3::Z * 0.5, h);
}

fn plaza(b: &mut Ctx, hx: f32, hy: f32) {
    // A paved square with a patterned centre, a fountain, tree beds,
    // benches, lamps and flag poles.
    let k = &mut b.kit;
    k.mat(ids::COBBLE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.1));
    let r = hx.min(hy) * 0.45;
    k.mat(ids::SIDEWALK).frustum(Vec3::Z * 0.1, r, r, 0.01, 32, true);
    k.mat(ids::CURB).frustum(Vec3::Z * 0.11, r * 0.75, r * 0.75, 0.01, 32, true);
    k.mat(ids::SIDEWALK).frustum(Vec3::Z * 0.12, r * 0.6, r * 0.6, 0.01, 32, true);
    k.mat(ids::CURB).frustum(Vec3::Z * 0.1, 3.0, 3.0, 0.6, 20, true);
    k.mat(ids::WATER).frustum(Vec3::Z * 0.1, 2.7, 2.7, 0.55, 20, true);
    k.mat(ids::CURB).frustum(Vec3::Z * 0.6, 0.5, 0.35, 1.2, 10, true);
    k.frustum(Vec3::Z * 1.8, 1.2, 0.2, 0.3, 14, true);
    k.mat(ids::WATER).frustum(Vec3::Z * 2.1, 0.12, 0.02, 1.0, 8, true);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        tree_bed(k, Vec3::new(x * (hx - 3.0), y * (hy - 3.0), 0.1), 7.0);
    }
    for i in 0..6 {
        let a = i as f32 * TAU / 6.0 + 0.5;
        let p = Vec2::from_angle(a) * (r + 1.5);
        bench(k, p.extend(0.1), a + FRAC_PI_2);
    }
    let g = b.glow(false);
    for s in [-1.0f32, 1.0] {
        doodads::streetlamp(&mut b.kit, Vec3::new(s * (r + 3.0), -hy + 1.0, 0.1), Vec2::new(-s, 0.0), g);
    }
    let paints = [ids::ROOF, ids::SLATE, ids::STUCCO];
    for (i, x) in [-4.0, 0.0, 4.0].into_iter().enumerate() {
        flag(&mut b.kit, Vec3::new(x, hy - 1.0, 0.1), 9.0, paints[i]);
    }
}

fn monument(b: &mut Ctx, hx: f32, hy: f32) {
    // A landmark on a stepped plinth in a round garden: an obelisk, a
    // column with a figure on top, or a triumphal arch (by seed).
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    k.mat(ids::SIDEWALK).block(Vec3::new(-1.2, -hy, 0.0), Vec3::new(1.2, 0.0, 0.08));
    let r = hx.min(hy) * 0.6;
    k.mat(ids::SIDEWALK).frustum(Vec3::Z * 0.05, r, r, 0.04, 28, true);
    let base = r * 0.55;
    for i in 0..3 {
        let w = base - i as f32 * 0.6;
        k.mat(ids::CURB).block(Vec3::new(-w, -w, 0.09 + i as f32 * 0.35), Vec3::new(w, w, 0.44 + i as f32 * 0.35));
    }
    let z = 1.14;
    match b.r.gen_range(0..3) {
        0 => {
            k.mat(ids::CURB).block(Vec3::new(-1.4, -1.4, z), Vec3::new(1.4, 1.4, z + 2.0));
            k.at(Mat4::from_translation(Vec3::Z * (z + 2.0)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4), |k| {
                k.frustum(Vec3::ZERO, 1.3, 0.75, 18.0, 4, false);
                k.frustum(Vec3::Z * 18.0, 0.75, 0.0, 1.6, 4, false);
            });
        }
        1 => {
            k.mat(ids::CURB).block(Vec3::new(-1.6, -1.6, z), Vec3::new(1.6, 1.6, z + 2.5));
            k.mat(ids::PLASTER).frustum(Vec3::Z * (z + 2.5), 0.9, 0.75, 14.0, 16, false);
            k.mat(ids::CURB).block(Vec3::new(-1.1, -1.1, z + 16.5), Vec3::new(1.1, 1.1, z + 17.1));
            let f = Vec3::Z * (z + 17.1);
            k.mat(ids::RUST).frustum(f, 0.35, 0.3, 1.4, 8, true);
            k.frustum(f + Vec3::Z * 1.4, 0.18, 0.18, 0.4, 8, true);
            k.beam(f + Vec3::new(0.0, 0.0, 1.3), f + Vec3::new(0.7, -0.3, 2.2), 0.12, 0.12);
        }
        _ => {
            let (w, d, h) = (base * 0.9, 1.8, 10.0);
            k.mat(ids::CURB);
            for s in [-1.0f32, 1.0] {
                k.block(Vec3::new(s * w - 1.6, -d, z), Vec3::new(s * w + 1.6, d, z + h));
            }
            k.block(Vec3::new(-w - 1.6, -d, z + h), Vec3::new(w + 1.6, d, z + h + 3.0));
            k.at(Mat4::from_translation(Vec3::new(0.0, -d - 0.01, z)) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.arch_wall(2.0 * w - 3.2, (w - 1.6).min(4.0), h + 0.01, 2.0 * d + 0.02, false));
            k.mat(ids::TRIM).block(Vec3::new(-w - 1.8, -d - 0.2, z + h + 3.0), Vec3::new(w + 1.8, d + 0.2, z + h + 3.4));
        }
    }
    for i in 0..8 {
        let p = Vec2::from_angle(i as f32 * TAU / 8.0) * (r + 2.0);
        let on_path = p.x.abs() < 2.0 && p.y < 0.0;
        if p.x.abs() < hx - 1.0 && p.y.abs() < hy - 1.0 && !on_path {
            tree(k, p.extend(0.05), 5.0);
        }
    }
}

fn pond(b: &mut Ctx, hx: f32, hy: f32) {
    // An irregular pond on a lawn, a gravel path round it, a wooden
    // footbridge over the narrow end, reeds, ducks, trees and benches.
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    let n = 24;
    let shape: Vec<Vec2> = (0..n)
        .map(|i| {
            let a = i as f32 * TAU / n as f32;
            let wob = 1.0 + 0.12 * (a * 3.0).sin() + 0.06 * (a * 5.0 + 1.0).cos();
            Vec2::new(a.cos() * hx * 0.62, a.sin() * hy * 0.5) * wob
        })
        .collect();
    let path: Vec<Vec2> = shape.iter().map(|p| *p * 1.18).collect();
    k.mat(ids::DUST).prism(&path, 0.0, 0.07, true, false);
    k.mat(ids::FOLIAGE).prism(&shape.iter().map(|p| *p * 1.08).collect::<Vec<_>>(), 0.0, 0.08, true, false);
    k.mat(ids::WATER).prism(&shape, 0.0, 0.09, true, false);
    // Footbridge across the east end.
    let x = hx * 0.45;
    let span = hy * 0.5 * 1.3;
    k.mat(ids::TRIM).beam(Vec3::new(x, -span, 0.1), Vec3::new(x, span, 0.1), 1.6, 0.15);
    for s in [-0.8, 0.8] {
        k.beam(Vec3::new(x + s, -span, 1.0), Vec3::new(x + s, span, 1.0), 0.08, 0.08);
        for i in 0..=6 {
            let y = -span + 2.0 * span * i as f32 / 6.0;
            k.block(Vec3::new(x + s - 0.05, y - 0.05, 0.1), Vec3::new(x + s + 0.05, y + 0.05, 1.0));
        }
    }
    // Reeds and ducks.
    for p in shape.iter().step_by(3) {
        if b.r.gen_bool(0.6) {
            let q = (*p * 0.95).extend(0.09);
            b.kit.mat(ids::FOLIAGE).frustum(q, 0.4, 0.05, b.r.gen_range(0.6..1.2), 5, false);
        }
    }
    for i in 0..3 {
        let p = Vec3::new(-hx * 0.2 + i as f32 * 1.6, b.r.gen_range(-2.0..2.0), 0.09);
        let k = &mut b.kit;
        k.mat(ids::PLASTER).cuboid(p + Vec3::Z * 0.12, Vec3::new(0.22, 0.12, 0.1), bevy::math::Quat::IDENTITY);
        k.mat(ids::FOLIAGE).cuboid(p + Vec3::new(0.18, 0.0, 0.3), Vec3::splat(0.07), bevy::math::Quat::IDENTITY);
    }
    let k = &mut b.kit;
    for (x, y) in [(-hx + 2.5, hy - 2.5), (-hx + 3.0, -hy + 3.0), (hx - 2.5, hy - 2.0)] {
        tree(k, Vec3::new(x, y, 0.05), 8.0);
    }
    bench(k, Vec3::new(-hx * 0.3, -hy + 1.2, 0.05), 0.0);
    bench(k, Vec3::new(hx * 0.1, hy - 1.2, 0.05), std::f32::consts::PI);
}

fn bandstand(b: &mut Ctx, hx: f32, hy: f32) {
    // An octagonal raised stage with slim columns, a railing, steps and
    // a pointed roof with a finial, on a lawn with deck chairs.
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    let r = hx.min(hy) * 0.4;
    k.mat(ids::BRICK).frustum(Vec3::Z * 0.05, r, r, 1.0, 8, true);
    k.mat(ids::TRIM).frustum(Vec3::Z * 1.05, r + 0.1, r + 0.1, 0.1, 8, true);
    let paint = if b.r.gen_bool(0.5) { ids::PLASTER } else { ids::STUCCO };
    let corner = |i: usize, rr: f32| {
        let a = (i as f32 + 0.5) * TAU / 8.0 - FRAC_PI_2;
        Vec2::from_angle(a) * rr
    };
    for i in 0..8 {
        let p = corner(i, r * 0.92);
        k.mat(paint).frustum(p.extend(1.15), 0.09, 0.09, 3.0, 8, true);
        if i != 7 {
            let q = corner(i + 1, r * 0.92);
            k.beam(p.extend(1.9), q.extend(1.9), 0.06, 0.06);
            k.beam(p.extend(1.3), q.extend(1.3), 0.04, 0.04);
        }
    }
    k.at(Mat4::from_translation(Vec3::Z * 4.15) * Mat4::from_rotation_z(TAU / 16.0), |k| {
        k.mat(ids::SLATE).frustum(Vec3::ZERO, r * 1.15, 0.0, r * 0.8, 8, true);
    });
    k.mat(ids::RUST).frustum(Vec3::Z * (4.15 + r * 0.8), 0.06, 0.0, 1.0, 6, true);
    // Steps down from the gap in the railing (front, -Y).
    for i in 0..4 {
        let y = -r * 0.92 + 0.3 - i as f32 * 0.3;
        k.mat(ids::BRICK).block(Vec3::new(-0.9, y - 0.3, 0.05), Vec3::new(0.9, y, 1.05 - i as f32 * 0.25));
    }
    for i in 0..10 {
        let x = -hx * 0.6 + (i % 5) as f32 * hx * 0.3;
        let y = -hy * 0.75 + (i / 5) as f32 * 1.6;
        k.at(Mat4::from_translation(Vec3::new(x, y, 0.05)), |k| {
            k.mat(ids::TRIM).beam(Vec3::new(0.0, -0.4, 0.1), Vec3::new(0.0, 0.3, 0.9), 0.6, 0.05);
            k.mat(paint).quad(Vec3::new(-0.28, -0.3, 0.4), Vec3::new(0.28, -0.3, 0.4), Vec3::new(0.28, 0.25, 0.85), Vec3::new(-0.28, 0.25, 0.85));
        });
    }
}

fn skate(b: &mut Ctx, hx: f32, hy: f32) {
    // A concrete pad with a half-pipe, quarter pipes, a fun box with a
    // rail, a bowl and a ledge.
    let k = &mut b.kit;
    k.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.1));
    // Half-pipe along the back: curved walls either side of a flat.
    let (hw, y) = (hx * 0.7, hy * 0.55);
    let n = 8;
    for s in [-1.0f32, 1.0] {
        let x0 = s * hw;
        for i in 0..n {
            let (a0, a1) = (i as f32 / n as f32 * FRAC_PI_2, (i + 1) as f32 / n as f32 * FRAC_PI_2);
            let p = |a: f32| Vec2::new(x0 - s * 2.5 * (1.0 - a.sin()), 2.5 * (1.0 - a.cos()) + 0.1);
            let (p0, p1) = (p(a0), p(a1));
            let (lo, hi) = if s < 0.0 { (p1, p0) } else { (p0, p1) };
            k.mat(ids::CURB).quad(Vec3::new(lo.x, y - 3.0, lo.y), Vec3::new(lo.x, y + 3.0, lo.y), Vec3::new(hi.x, y + 3.0, hi.y), Vec3::new(hi.x, y - 3.0, hi.y));
        }
        let (a, c) = if s < 0.0 { (x0 - 0.3, x0) } else { (x0, x0 + 0.3) };
        k.block(Vec3::new(a, y - 3.0, 0.1), Vec3::new(c, y + 3.0, 2.6));
        k.mat(ids::IRON).tube(Vec3::new(x0, y - 3.0, 2.6), Vec3::new(x0, y + 3.0, 2.6), 0.05, 6);
        k.mat(ids::IRON).block(Vec3::new(x0 + s * 0.3 - 0.03, y - 3.0, 2.6), Vec3::new(x0 + s * 0.3 + 0.03, y + 3.0, 3.6));
    }
    k.mat(ids::CURB);
    for sy in [-1.0f32, 1.0] {
        let yy = y + sy * 3.0;
        k.block(Vec3::new(-hw - 0.3, yy - 0.15, 0.1), Vec3::new(hw + 0.3, yy + 0.15, 0.2));
    }
    // Fun box with a rail and ramps.
    let c = Vec3::new(-hx * 0.3, -hy * 0.35, 0.1);
    k.block(c - Vec3::new(1.5, 1.0, 0.0), c + Vec3::new(1.5, 1.0, 0.6));
    for s in [-1.0f32, 1.0] {
        let x = c.x + s * 1.5;
        k.quad(Vec3::new(x, c.y - 1.0, 0.7), Vec3::new(x, c.y + 1.0, 0.7), Vec3::new(x + s * 2.0, c.y + 1.0, 0.1), Vec3::new(x + s * 2.0, c.y - 1.0, 0.1));
        k.quad(Vec3::new(x + s * 2.0, c.y - 1.0, 0.1), Vec3::new(x + s * 2.0, c.y + 1.0, 0.1), Vec3::new(x, c.y + 1.0, 0.7), Vec3::new(x, c.y - 1.0, 0.7));
    }
    k.mat(ids::IRON).tube(c + Vec3::new(-1.4, 0.0, 0.95), c + Vec3::new(1.4, 0.0, 0.95), 0.04, 6);
    for x in [-1.2, 1.2] {
        k.block(c + Vec3::new(x - 0.03, -0.03, 0.6), c + Vec3::new(x + 0.03, 0.03, 0.95));
    }
    // A bowl: stepped rings dished into a raised deck.
    let bc = Vec3::new(hx * 0.4, -hy * 0.35, 0.1);
    k.mat(ids::CURB).frustum(bc, 3.6, 3.6, 1.2, 20, true);
    for i in 0..4 {
        let rr = 3.2 - i as f32 * 0.7;
        k.mat(if i % 2 == 0 { ids::SIDEWALK } else { ids::CURB }).frustum(bc + Vec3::Z * (1.2 - i as f32 * 0.25), rr, rr, 0.02, 20, true);
    }
    k.mat(ids::IRON).frustum(bc + Vec3::Z * 1.2, 3.65, 3.65, 0.06, 20, false);
    k.mat(ids::CURB).block(Vec3::new(-hx + 1.0, -hy + 1.0, 0.1), Vec3::new(hx * 0.1, -hy + 1.6, 0.6));
}
