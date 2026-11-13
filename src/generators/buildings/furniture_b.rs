//! Furniture, part two: tables. Dining table, coffee table, side table
//! and desk. Floor at z = 0, front facing -Y; the width runs along X.

use super::furniture::{cushion, legs};
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::ids;
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use std::f32::consts::TAU;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::DiningTable => dining(b, hx, hy),
        K::CoffeeTable => coffee(b, hx, hy),
        K::SideTable => side(b, hx, hy),
        K::Desk => desk(b, hx, hy),
        _ => {}
    }
}

/// A seeded table top: wood, white, dark or glass.
fn top_mat(b: &mut Ctx) -> u16 {
    [ids::TRIM, ids::TRIM, ids::PLASTER, ids::SLATE, ids::GLASS][b.r.gen_range(0..5)]
}

fn dining(b: &mut Ctx, hx: f32, hy: f32) {
    // A rectangular table (or round, by seed when the lot is square)
    // with an apron, legs, place mats, plates and a centre bowl.
    let top = top_mat(b);
    let round = (hx - hy).abs() < 0.15 && b.r.gen_bool(0.6);
    let k = &mut b.kit;
    let h = 0.75;
    if round {
        k.mat(ids::TRIM).frustum(Vec3::ZERO, 0.3, 0.26, 0.04, 16, true);
        k.frustum(Vec3::new(0.0, 0.0, 0.04), 0.06, 0.06, h - 0.08, 10, true);
        k.mat(top).frustum(Vec3::new(0.0, 0.0, h - 0.04), hx, hx, 0.04, 32, true);
    } else {
        k.mat(ids::TRIM);
        legs(k, hx, hy, 0.08, h - 0.04, 0.03);
        k.block(Vec3::new(-hx + 0.06, -hy + 0.06, h - 0.14), Vec3::new(hx - 0.06, hy - 0.06, h - 0.04));
        k.mat(top).block(Vec3::new(-hx, -hy, h - 0.04), Vec3::new(hx, hy, h));
    }
    // Place settings around the edge.
    let seats: Vec<Vec2> = if round {
        (0..4).map(|i| Vec2::from_angle(i as f32 * TAU / 4.0) * (hx - 0.25)).collect()
    } else {
        let n = ((hx * 2.0 / 0.65) as usize).max(1);
        (0..n).flat_map(|i| {
            let x = -hx + (i as f32 + 0.5) * hx * 2.0 / n as f32;
            [Vec2::new(x, -hy + 0.22), Vec2::new(x, hy - 0.22)]
        }).collect()
    };
    for p in seats {
        k.mat(ids::STUCCO).block((p - Vec2::new(0.18, 0.13)).extend(h), (p + Vec2::new(0.18, 0.13)).extend(h + 0.004));
        k.mat(ids::PLASTER).frustum(p.extend(h + 0.004), 0.1, 0.12, 0.02, 16, true);
    }
    k.mat(ids::SLATE).frustum(Vec3::new(0.0, 0.0, h), 0.08, 0.14, 0.08, 12, true);
    k.mat(ids::ROOF).cuboid(Vec3::new(0.03, 0.0, h + 0.09), Vec3::splat(0.035), bevy::math::Quat::IDENTITY);
    k.mat(ids::STUCCO).cuboid(Vec3::new(-0.04, 0.02, h + 0.09), Vec3::splat(0.035), bevy::math::Quat::IDENTITY);
}

fn coffee(b: &mut Ctx, hx: f32, hy: f32) {
    // A low table with a lower shelf, a stack of books and a mug.
    let top = top_mat(b);
    let k = &mut b.kit;
    let h = 0.42;
    k.mat(ids::TRIM);
    legs(k, hx, hy, 0.05, h - 0.03, 0.025);
    k.block(Vec3::new(-hx + 0.03, -hy + 0.03, 0.1), Vec3::new(hx - 0.03, hy - 0.03, 0.12));
    k.mat(top).block(Vec3::new(-hx, -hy, h - 0.03), Vec3::new(hx, hy, h));
    let books = [ids::ROOF, ids::SLATE, ids::STUCCO];
    for (i, &c) in books.iter().enumerate() {
        let z = h + i as f32 * 0.03;
        let o = Vec3::new(-hx * 0.4 + i as f32 * 0.01, 0.0, z);
        k.mat(c).block(o - Vec3::new(0.12, 0.09, 0.0), o + Vec3::new(0.12, 0.09, 0.028));
    }
    k.mat(ids::PLASTER).frustum(Vec3::new(hx * 0.4, -hy * 0.2, h), 0.04, 0.04, 0.09, 12, true);
    k.mat(ids::SLATE).block(Vec3::new(-0.2, -0.1, 0.12), Vec3::new(0.2, 0.1, 0.16));
}

fn side(b: &mut Ctx, hx: f32, hy: f32) {
    // A small table with a drawer and a potted plant or a lamp on top.
    let top = top_mat(b);
    let lamp = b.r.gen_bool(0.5);
    let shade = super::furniture::fabric(b);
    let k = &mut b.kit;
    let h = 0.58;
    k.mat(ids::TRIM);
    legs(k, hx, hy, 0.03, h - 0.03, 0.02);
    k.block(Vec3::new(-hx + 0.02, -hy + 0.02, h - 0.16), Vec3::new(hx - 0.02, hy - 0.02, h - 0.03));
    k.mat(ids::STUCCO).block(Vec3::new(-hx + 0.06, -hy + 0.015, h - 0.14), Vec3::new(hx - 0.06, -hy + 0.02, h - 0.05));
    k.mat(ids::IRON).block(Vec3::new(-0.03, -hy, h - 0.1), Vec3::new(0.03, -hy + 0.015, h - 0.09));
    k.mat(top).block(Vec3::new(-hx, -hy, h - 0.03), Vec3::new(hx, hy, h));
    if lamp {
        k.mat(ids::IRON).frustum(Vec3::new(0.0, 0.0, h), 0.08, 0.07, 0.02, 12, true);
        k.frustum(Vec3::new(0.0, 0.0, h + 0.02), 0.012, 0.012, 0.3, 6, true);
        let glow = b.glow(false);
        let k = &mut b.kit;
        k.mat(glow).frustum(Vec3::new(0.0, 0.0, h + 0.26), 0.03, 0.03, 0.06, 8, true);
        k.mat(shade).frustum(Vec3::new(0.0, 0.0, h + 0.2), 0.16, 0.1, 0.2, 16, false);
    } else {
        k.mat(ids::RUST).frustum(Vec3::new(0.0, 0.0, h), 0.06, 0.08, 0.12, 10, true);
        super::garden::shrub(k, Vec3::new(0.0, 0.0, h + 0.12), 0.12);
    }
}

fn desk(b: &mut Ctx, hx: f32, hy: f32) {
    // A work desk with a drawer pedestal, modesty panel, a monitor,
    // keyboard, mouse and a desk lamp.
    let top = top_mat(b);
    let k = &mut b.kit;
    let h = 0.74;
    let px = hx - 0.42;
    k.mat(ids::TRIM);
    for x in [-hx + 0.02, -hx + 0.05] {
        k.block(Vec3::new(x - 0.015, -hy + 0.03, 0.0), Vec3::new(x + 0.015, hy - 0.03, h - 0.03));
    }
    k.block(Vec3::new(px, -hy + 0.03, 0.0), Vec3::new(hx - 0.02, hy - 0.03, h - 0.03));
    k.block(Vec3::new(-hx + 0.05, hy - 0.05, 0.3), Vec3::new(px, hy - 0.03, h - 0.03));
    k.mat(ids::STUCCO);
    for i in 0..3 {
        let z = 0.04 + i as f32 * 0.22;
        cushion(k, Vec3::new(px + 0.02, -hy + 0.02, z), Vec3::new(hx - 0.04, -hy + 0.03, z + 0.2));
        k.mat(ids::IRON).block(Vec3::new((px + hx) * 0.5 - 0.05, -hy + 0.005, z + 0.15), Vec3::new((px + hx) * 0.5 + 0.05, -hy + 0.02, z + 0.165));
        k.mat(ids::STUCCO);
    }
    k.mat(top).block(Vec3::new(-hx, -hy, h - 0.03), Vec3::new(hx, hy, h));
    // Monitor on a stand, keyboard and mouse.
    let m = Vec3::new(-0.1, hy - 0.2, h);
    k.mat(ids::IRON).block(m + Vec3::new(-0.12, -0.08, 0.0), m + Vec3::new(0.12, 0.08, 0.01));
    k.block(m + Vec3::new(-0.02, 0.0, 0.01), m + Vec3::new(0.02, 0.03, 0.2));
    k.mat(ids::ASPHALT).block(m + Vec3::new(-0.3, -0.02, 0.12), m + Vec3::new(0.3, 0.01, 0.48));
    let glow = b.glow(false);
    let k = &mut b.kit;
    k.mat(glow).block(m + Vec3::new(-0.28, -0.025, 0.14), m + Vec3::new(0.28, -0.02, 0.46));
    k.mat(ids::ASPHALT).block(Vec3::new(-0.32, -hy + 0.12, h), Vec3::new(0.12, -hy + 0.27, h + 0.02));
    k.block(Vec3::new(0.2, -hy + 0.15, h), Vec3::new(0.26, -hy + 0.25, h + 0.025));
    // A desk lamp on the left.
    let l = Vec3::new(-hx + 0.2, hy - 0.2, h);
    k.mat(ids::IRON).frustum(l, 0.07, 0.07, 0.02, 12, true);
    k.beam(l + Vec3::Z * 0.02, l + Vec3::new(0.05, -0.05, 0.32), 0.015, 0.015);
    k.beam(l + Vec3::new(0.05, -0.05, 0.32), l + Vec3::new(0.15, -0.15, 0.28), 0.015, 0.015);
    k.frustum(l + Vec3::new(0.15, -0.15, 0.2), 0.07, 0.03, 0.08, 10, false);
}
