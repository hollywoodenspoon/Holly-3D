//! Doodads & street props: streetlamp, lantern post, car / truck, wooden
//! cart, market stall, barrel / crate cluster, dumpster, sandbag wall,
//! Czech-hedgehog tank trap, barbed wire coil, park bench and fountain.
//! Width x Length scale each prop's footprint.

use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    let g = b.glow(false);
    match b.p.kind {
        K::Streetlamp => streetlamp(&mut b.kit, Vec3::ZERO, Vec2::NEG_Y, g),
        K::LanternPost => lantern_post(&mut b.kit, g),
        K::Car => car(b, hx, hy),
        K::Cart => cart(&mut b.kit, hx, hy),
        K::MarketStall => stall(b, hx, hy),
        K::BarrelCrates => barrels(b, hx, hy),
        K::Dumpster => dumpster(&mut b.kit, hx, hy),
        K::SandbagWall => sandbags(b, hx, hy),
        K::TankTrap => tank_trap(&mut b.kit, hx),
        K::BarbedWire => barbed_wire(&mut b.kit, hx, hy),
        K::Bench => bench(&mut b.kit, hx, hy),
        K::Fountain => fountain(&mut b.kit, hx),
        _ => {}
    }
}

/// Modern streetlamp at `at`, arm reaching along `dir`, lamp Material `glow`.
pub fn streetlamp(kit: &mut Kit, at: Vec3, dir: Vec2, glow: u16) {
    let d = dir.normalize_or_zero().extend(0.0);
    kit.mat(ids::IRON).frustum(at, 0.12, 0.08, 6.5, 6, true);
    let top = at + Vec3::Z * 6.4;
    kit.beam(top, top + d * 1.6, 0.1, 0.1);
    kit.mat(glow).cuboid(top + d * 1.6 - Vec3::Z * 0.15, Vec3::new(0.3, 0.3, 0.1), Quat::IDENTITY);
}

fn lantern_post(kit: &mut Kit, glow: u16) {
    kit.mat(ids::IRON).frustum(Vec3::ZERO, 0.2, 0.2, 0.3, 8, true);
    kit.frustum(Vec3::Z * 0.3, 0.07, 0.05, 2.6, 6, true);
    kit.mat(glow).frustum(Vec3::Z * 2.9, 0.16, 0.2, 0.45, 6, true);
    kit.mat(ids::IRON).frustum(Vec3::Z * 3.35, 0.24, 0.0, 0.25, 6, false);
}

fn car(b: &mut Ctx, hx: f32, hy: f32) {
    // Body, cabin (truck: cab + cargo box), wheels, lights.
    let truck = hy > 3.0 && b.r.gen_bool(0.35);
    let paint = [ids::BRICK, ids::STUCCO, ids::IRON, ids::PLASTER][b.r.gen_range(0..4)];
    let wr = 0.34;
    b.kit.mat(paint).block(Vec3::new(-hx, -hy, wr), Vec3::new(hx, hy, wr + 0.65));
    if truck {
        b.kit.block(Vec3::new(-hx, -hy, wr + 0.65), Vec3::new(hx, -hy + 1.6, wr + 1.7));
        b.kit.mat(ids::RUST).block(Vec3::new(-hx, -hy + 1.75, wr + 0.65), Vec3::new(hx, hy, wr + 2.2));
        crate::generators::settlement::window(&mut b.kit, 0.0, -hy - 0.01, wr + 0.85, hx * 1.6, 0.7, ids::GLASS);
    } else {
        let c = [Vec3::new(-hx * 0.9, -hy * 0.45, 0.0), Vec3::new(hx * 0.9, -hy * 0.45, 0.0), Vec3::new(hx * 0.9, hy * 0.6, 0.0), Vec3::new(-hx * 0.9, hy * 0.6, 0.0)];
        let (z0, z1) = (wr + 0.65, wr + 1.15);
        let t = [Vec3::new(-hx * 0.8, -hy * 0.2, 0.0), Vec3::new(hx * 0.8, -hy * 0.2, 0.0), Vec3::new(hx * 0.8, hy * 0.45, 0.0), Vec3::new(-hx * 0.8, hy * 0.45, 0.0)];
        b.kit.mat(ids::GLASS);
        for i in 0..4 {
            let j = (i + 1) % 4;
            b.kit.quad(c[i] + Vec3::Z * z0, c[j] + Vec3::Z * z0, t[j] + Vec3::Z * z1, t[i] + Vec3::Z * z1);
        }
        b.kit.mat(paint).poly(&t.map(|p| p + Vec3::Z * z1));
    }
    b.kit.mat(ids::ASPHALT);
    for (sx, sy) in [(-1.0, -0.65), (1.0, -0.65), (1.0, 0.65), (-1.0, 0.65)] {
        b.kit.at(Mat4::from_translation(Vec3::new(sx * (hx - 0.05), sy * hy, wr)) * Mat4::from_rotation_y(FRAC_PI_2), |k| {
            k.frustum(Vec3::Z * -0.12, wr, wr, 0.24, 10, true)
        });
    }
    let g = b.glow(false);
    for s in [-1.0, 1.0] {
        b.kit.mat(g).block(Vec3::new(s * hx * 0.7 - 0.15, -hy - 0.02, wr + 0.4), Vec3::new(s * hx * 0.7 + 0.15, -hy, wr + 0.55));
    }
}

fn cart(kit: &mut Kit, hx: f32, hy: f32) {
    let wr = 0.55;
    kit.mat(ids::TRIM).block(Vec3::new(-hx, -hy * 0.6, wr), Vec3::new(hx, hy * 0.7, wr + 0.12));
    for s in [-1.0, 1.0] {
        kit.block(Vec3::new(s * hx - 0.05, -hy * 0.6, wr), Vec3::new(s * hx + 0.05, hy * 0.7, wr + 0.6));
        kit.at(Mat4::from_translation(Vec3::new(s * (hx + 0.1), hy * 0.1, wr)) * Mat4::from_rotation_y(FRAC_PI_2), |k| {
            k.frustum(Vec3::Z * -0.05, wr, wr, 0.1, 12, true);
        });
    }
    // Shafts for the horse.
    for s in [-0.4, 0.4] {
        kit.beam(Vec3::new(s * hx, -hy * 0.6, wr + 0.05), Vec3::new(s * hx, -hy, wr - 0.1), 0.08, 0.08);
    }
    kit.mat(ids::FOLIAGE).block(Vec3::new(-hx * 0.8, -hy * 0.4, wr + 0.12), Vec3::new(hx * 0.8, hy * 0.5, wr + 0.5));
}

fn stall(b: &mut Ctx, hx: f32, hy: f32) {
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx, -hy, 0.8), Vec3::new(hx, hy * 0.2, 0.9));
    b.kit.block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, -hy + 0.08, 0.8));
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let h = if sy < 0.0 { 2.3 } else { 2.0 };
        b.kit.block(Vec3::new(sx * hx - 0.05, sy * hy - 0.05, 0.0), Vec3::new(sx * hx + 0.05, sy * hy + 0.05, h));
    }
    // Striped canopy (alternating cloth IDs) and goods on the counter.
    let n = ((2.0 * hx / 0.5) as usize).max(2);
    for i in 0..n {
        let (x0, x1) = (-hx - 0.2 + (2.0 * hx + 0.4) * i as f32 / n as f32, -hx - 0.2 + (2.0 * hx + 0.4) * (i + 1) as f32 / n as f32);
        b.kit.mat(if i % 2 == 0 { ids::PLASTER } else { ids::BRICK });
        b.kit.quad(Vec3::new(x0, -hy - 0.5, 2.1), Vec3::new(x1, -hy - 0.5, 2.1), Vec3::new(x1, hy + 0.2, 2.6), Vec3::new(x0, hy + 0.2, 2.6));
    }
    b.kit.mat(ids::FOLIAGE);
    for _ in 0..6 {
        let c = Vec3::new(b.r.gen_range(-hx * 0.8..hx * 0.8), b.r.gen_range(-hy * 0.8..0.0), 0.9);
        b.kit.frustum(c, 0.15, 0.12, 0.15, 6, true);
    }
}

fn barrels(b: &mut Ctx, hx: f32, hy: f32) {
    let n = ((hx * hy * 2.0) as usize).clamp(3, 14);
    for i in 0..n {
        let c = Vec3::new(b.r.gen_range(-hx..hx), b.r.gen_range(-hy..hy), 0.0);
        if i % 2 == 0 {
            b.kit.mat(ids::TRIM).frustum(c, 0.3, 0.33, 0.45, 10, false);
            b.kit.frustum(c + Vec3::Z * 0.45, 0.33, 0.3, 0.45, 10, true);
            b.kit.mat(ids::IRON).frustum(c + Vec3::Z * 0.15, 0.32, 0.32, 0.05, 10, false);
        } else {
            let s: f32 = b.r.gen_range(0.35..0.55);
            let z = if b.r.gen_bool(0.3) { s * 2.0 } else { 0.0 };
            b.kit.mat(ids::TRIM).cuboid(c + Vec3::Z * (z + s), Vec3::splat(s), Quat::from_rotation_z(b.r.gen_range(0.0..1.5)));
        }
    }
}

fn dumpster(kit: &mut Kit, hx: f32, hy: f32) {
    kit.mat(ids::RUST).block(Vec3::new(-hx, -hy, 0.15), Vec3::new(hx, hy, 1.3));
    kit.mat(ids::IRON).quad(Vec3::new(-hx, hy, 1.3), Vec3::new(-hx, -hy - 0.1, 1.45), Vec3::new(hx, -hy - 0.1, 1.45), Vec3::new(hx, hy, 1.3));
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        kit.frustum(Vec3::new(sx * (hx - 0.15), sy * (hy - 0.15), 0.0), 0.08, 0.08, 0.15, 6, true);
    }
}

fn sandbags(b: &mut Ctx, hx: f32, hy: f32) {
    // Staggered courses of pillow-shaped bags along Y.
    let rows = b.p.floors.clamp(1, 8);
    let (bl, bh) = (0.6, 0.18);
    let n = ((2.0 * hy / bl) as usize).max(1);
    for r in 0..rows {
        let off = if r % 2 == 0 { 0.0 } else { bl * 0.5 };
        for i in 0..n {
            let y = -hy + off + bl * (i as f32 + 0.5);
            if y > hy {
                continue;
            }
            let tilt = b.r.gen_range(-0.08..0.08);
            b.kit.mat(ids::LOT).cuboid(Vec3::new(0.0, y, bh * (r as f32 + 0.5)), Vec3::new(hx.min(0.45) * 0.9, bl * 0.47, bh * 0.5), Quat::from_rotation_z(tilt));
        }
    }
}

fn tank_trap(kit: &mut Kit, hx: f32) {
    // Three crossed steel angle beams.
    let l = hx * 1.1;
    kit.mat(ids::RUST);
    let c = Vec3::Z * l * 0.42;
    for d in [Vec3::new(1.0, 1.0, 1.0), Vec3::new(-1.0, 1.0, -1.0), Vec3::new(1.0, -1.0, -1.0)] {
        let d = d.normalize() * l * 0.5;
        kit.beam(c - d, c + d, 0.12, 0.12);
    }
}

fn barbed_wire(kit: &mut Kit, hx: f32, hy: f32) {
    // A concertina coil along Y on a few pickets.
    let r = hx.min(0.6);
    let loops = ((2.0 * hy / 0.25) as usize).max(4);
    kit.mat(ids::IRON);
    let mut prev = Vec3::new(r, -hy, r);
    for i in 1..=loops * 8 {
        let t = i as f32 / (loops * 8) as f32;
        let a = t * loops as f32 * TAU;
        let p = Vec3::new(a.cos() * r, -hy + 2.0 * hy * t, r + a.sin() * r);
        kit.beam(prev, p, 0.02, 0.02);
        prev = p;
    }
    for y in [-hy, 0.0, hy] {
        kit.mat(ids::TRIM).block(Vec3::new(-0.04, y - 0.04, 0.0), Vec3::new(0.04, y + 0.04, 2.0 * r + 0.2));
    }
}

fn bench(kit: &mut Kit, hx: f32, hy: f32) {
    kit.mat(ids::TRIM);
    for i in 0..3 {
        let y = -hy * 0.6 + i as f32 * hy * 0.35;
        kit.block(Vec3::new(-hx, y, 0.45), Vec3::new(hx, y + hy * 0.28, 0.5));
    }
    for i in 0..2 {
        let z = 0.65 + i as f32 * 0.2;
        kit.block(Vec3::new(-hx, hy * 0.45, z), Vec3::new(hx, hy * 0.55, z + 0.12));
    }
    kit.mat(ids::IRON);
    for s in [-1.0, 1.0] {
        kit.block(Vec3::new(s * (hx - 0.15) - 0.04, -hy * 0.6, 0.0), Vec3::new(s * (hx - 0.15) + 0.04, hy * 0.55, 0.45));
        kit.block(Vec3::new(s * (hx - 0.15) - 0.04, hy * 0.45, 0.45), Vec3::new(s * (hx - 0.15) + 0.04, hy * 0.55, 0.95));
    }
}

fn fountain(kit: &mut Kit, hx: f32) {
    let r = hx;
    kit.mat(ids::FACADE).frustum(Vec3::ZERO, r, r, 0.6, 16, true);
    kit.mat(ids::WATER).frustum(Vec3::Z * 0.5, r * 0.88, r * 0.88, 0.02, 16, false);
    kit.mat(ids::FACADE).frustum(Vec3::ZERO, r * 0.15, r * 0.12, 1.6, 8, true);
    kit.frustum(Vec3::Z * 1.6, r * 0.45, r * 0.4, 0.25, 12, true);
    kit.mat(ids::WATER).frustum(Vec3::Z * 1.83, r * 0.36, r * 0.36, 0.01, 12, false);
    kit.mat(ids::FACADE).frustum(Vec3::Z * 1.85, r * 0.06, r * 0.04, 0.8, 6, true);
}
