//! Kitchen & bath fittings: base-unit run, wall cabinets, kitchen
//! island, toilet, vanity sink, bathtub and shower cubicle. Floor at
//! z = 0, front facing -Y, backs against a wall at +Y.

use super::furniture::cushion;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec3};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::KitchenUnits => units(b, hx, hy),
        K::WallCabinets => wall_cabinets(b, hx, hy),
        K::KitchenIsland => island(b, hx, hy),
        K::Toilet => toilet(b),
        K::VanitySink => vanity(b, hx, hy),
        K::Bathtub => bathtub(b, hx, hy),
        K::ShowerCubicle => shower(b, hx, hy),
        _ => {}
    }
}

/// A seeded cabinet-door finish.
fn door_mat(b: &mut Ctx) -> u16 {
    [ids::PLASTER, ids::SLATE, ids::TRIM, ids::STUCCO][b.r.gen_range(0..4)]
}

/// A row of cabinet doors (or drawers) along the -Y face at `y`.
fn doors(k: &mut Kit, x0: f32, x1: f32, y: f32, z0: f32, z1: f32, front: u16) {
    let n = (((x1 - x0) / 0.6).round() as usize).max(1);
    let w = (x1 - x0) / n as f32;
    for i in 0..n {
        let a = x0 + i as f32 * w;
        k.mat(front);
        cushion(k, Vec3::new(a + 0.005, y - 0.02, z0), Vec3::new(a + w - 0.005, y, z1));
        let hxp = if i % 2 == 0 { a + w - 0.06 } else { a + 0.06 };
        k.mat(ids::IRON).block(Vec3::new(hxp - 0.01, y - 0.04, z1 - 0.2), Vec3::new(hxp + 0.01, y - 0.02, z1 - 0.06));
    }
}

/// A stainless sink basin with a mixer tap, set into a top at `z`.
fn sink(k: &mut Kit, c: Vec3, hw: f32, hd: f32) {
    k.mat(ids::IRON).block(c - Vec3::new(hw, hd, 0.0), c + Vec3::new(hw, hd, 0.012));
    k.mat(ids::ASPHALT).block(c - Vec3::new(hw - 0.03, hd - 0.03, -0.012), c + Vec3::new(hw - 0.03, hd - 0.03, 0.014));
    let t = c + Vec3::new(0.0, hd + 0.04, 0.0);
    k.mat(ids::IRON).frustum(t, 0.02, 0.02, 0.25, 8, true);
    k.beam(t + Vec3::Z * 0.25, t + Vec3::new(0.0, -0.18, 0.25), 0.02, 0.02);
}

fn units(b: &mut Ctx, hx: f32, hy: f32) {
    // A run of base units with a plinth, worktop, a sink under a tap,
    // a four-ring hob with an oven below, and a splashback.
    let front = door_mat(b);
    let top = [ids::SLATE, ids::TRIM, ids::PLASTER][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    let (h, d) = (0.9, hy.min(0.32));
    let y0 = hy - d * 2.0;
    k.mat(ids::ASPHALT).block(Vec3::new(-hx, y0 + 0.06, 0.0), Vec3::new(hx, hy, 0.1));
    k.mat(front).block(Vec3::new(-hx, y0 + 0.02, 0.1), Vec3::new(hx, hy, h - 0.04));
    k.mat(top).block(Vec3::new(-hx, y0 - 0.02, h - 0.04), Vec3::new(hx, hy, h));
    k.mat(ids::STUCCO).block(Vec3::new(-hx, hy - 0.01, h), Vec3::new(hx, hy, h + 0.5));
    // Oven bay on the right third, sink on the left third.
    let ox = hx - 0.3 - (hx * 0.3).min(0.4);
    doors(k, -hx, ox - 0.3, y0 + 0.02, 0.12, h - 0.06, front);
    doors(k, ox + 0.3, hx, y0 + 0.02, 0.12, h - 0.06, front);
    k.mat(ids::IRON).block(Vec3::new(ox - 0.3, y0 + 0.0, 0.12), Vec3::new(ox + 0.3, y0 + 0.02, h - 0.06));
    k.mat(ids::ASPHALT).block(Vec3::new(ox - 0.25, y0 - 0.005, 0.2), Vec3::new(ox + 0.25, y0 + 0.0, 0.62));
    k.mat(ids::IRON).block(Vec3::new(ox - 0.22, y0 - 0.04, 0.68), Vec3::new(ox + 0.22, y0 - 0.02, 0.7));
    k.mat(ids::ASPHALT).block(Vec3::new(ox - 0.29, y0 + 0.05, h), Vec3::new(ox + 0.29, y0 + 0.55, h + 0.008));
    for (dx, dy) in [(-0.14, 0.15), (0.14, 0.15), (-0.14, 0.4), (0.14, 0.4)] {
        k.mat(ids::IRON).frustum(Vec3::new(ox + dx, y0 + dy, h + 0.008), 0.08, 0.08, 0.006, 14, true);
    }
    sink(k, Vec3::new(-hx + 0.6, y0 + d, h), 0.3, 0.2);
}

fn wall_cabinets(b: &mut Ctx, hx: f32, hy: f32) {
    // Wall-hung cabinets with an under-cabinet light strip, mounted
    // 1.45 m up, with a cooker hood in the middle when wide enough.
    let front = door_mat(b);
    let glow = b.glow(false);
    let k = &mut b.kit;
    let (z0, z1, d) = (1.45, 2.15, hy.min(0.18));
    let y0 = hy - d * 2.0;
    k.mat(front).block(Vec3::new(-hx, y0 + 0.02, z0), Vec3::new(hx, hy, z1));
    if hx > 0.9 {
        doors(k, -hx, -0.3, y0 + 0.02, z0 + 0.02, z1 - 0.02, front);
        doors(k, 0.3, hx, y0 + 0.02, z0 + 0.02, z1 - 0.02, front);
        k.mat(ids::IRON).block(Vec3::new(-0.3, y0 - 0.1, z0 - 0.15), Vec3::new(0.3, hy, z0 + 0.05));
        k.block(Vec3::new(-0.12, hy - 0.2, z0 + 0.05), Vec3::new(0.12, hy, z1));
    } else {
        doors(k, -hx, hx, y0 + 0.02, z0 + 0.02, z1 - 0.02, front);
    }
    k.mat(glow).block(Vec3::new(-hx + 0.05, y0 + 0.05, z0 - 0.01), Vec3::new(hx - 0.05, y0 + 0.08, z0));
}

fn island(b: &mut Ctx, hx: f32, hy: f32) {
    // A freestanding island: cabinets on one side, an overhanging top
    // and bar stools along the other side, with a prep sink.
    let front = door_mat(b);
    let top = [ids::SLATE, ids::PLASTER, ids::TRIM][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    // Built facing +Y, then turned so the stools sit on the front (-Y).
    let turn = k.xf;
    k.xf = turn * Mat4::from_rotation_z(std::f32::consts::PI);
    let h = 0.92;
    let (cx, cy) = (hx - 0.05, hy - 0.3);
    k.mat(ids::ASPHALT).block(Vec3::new(-cx + 0.05, -cy + 0.05, 0.0), Vec3::new(cx - 0.05, cy * 0.4, 0.1));
    k.mat(front).block(Vec3::new(-cx, -cy, 0.1), Vec3::new(cx, cy * 0.4, h - 0.04));
    doors(k, -cx, cx, -cy, 0.12, h - 0.06, front);
    k.mat(top).block(Vec3::new(-hx, -cy - 0.02, h - 0.04), Vec3::new(hx, hy, h));
    sink(k, Vec3::new(-hx * 0.4, -cy + 0.3, h), 0.2, 0.15);
    let n = ((hx * 2.0 / 0.6) as usize).clamp(1, 5);
    for i in 0..n {
        let x = -hx + (i as f32 + 0.5) * hx * 2.0 / n as f32;
        let s = Vec3::new(x, hy - 0.05, 0.0);
        k.mat(ids::IRON).frustum(s, 0.16, 0.14, 0.02, 12, true);
        k.frustum(s + Vec3::Z * 0.02, 0.025, 0.025, 0.6, 8, true);
        k.mat(ids::SLATE).frustum(s + Vec3::Z * 0.62, 0.17, 0.16, 0.05, 12, true);
    }
    k.xf = turn;
}

fn toilet(b: &mut Ctx) {
    // A close-coupled toilet: pedestal, bowl, seat and lid up, cistern
    // with a flush button, against the wall at +Y.
    let k = &mut b.kit;
    k.mat(ids::PLASTER);
    k.frustum(Vec3::new(0.0, -0.02, 0.0), 0.12, 0.16, 0.25, 14, false);
    k.at(Mat4::from_translation(Vec3::new(0.0, -0.06, 0.25)) * Mat4::from_scale(Vec3::new(1.0, 1.3, 1.0)), |k| k.frustum(Vec3::ZERO, 0.16, 0.19, 0.14, 18, false));
    k.at(Mat4::from_translation(Vec3::new(0.0, -0.06, 0.39)) * Mat4::from_scale(Vec3::new(1.0, 1.3, 1.0)), |k| k.frustum(Vec3::ZERO, 0.19, 0.19, 0.02, 18, true));
    k.block(Vec3::new(-0.19, 0.13, 0.36), Vec3::new(0.19, 0.3, 0.78));
    k.block(Vec3::new(-0.2, 0.12, 0.78), Vec3::new(0.2, 0.3, 0.8));
    k.at(Mat4::from_translation(Vec3::new(0.0, 0.12, 0.41)) * Mat4::from_rotation_x(-0.2) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.block(Vec3::new(-0.18, 0.0, -0.02), Vec3::new(0.18, 0.4, 0.0)));
    k.mat(ids::IRON).frustum(Vec3::new(0.0, 0.21, 0.8), 0.03, 0.03, 0.01, 10, true);
}

fn vanity(b: &mut Ctx, hx: f32, hy: f32) {
    // A vanity unit with doors, a basin and tap, a mirror cabinet above
    // and a light bar.
    let front = door_mat(b);
    let glow = b.glow(false);
    let k = &mut b.kit;
    let h = 0.85;
    k.mat(front).block(Vec3::new(-hx, -hy + 0.02, 0.15), Vec3::new(hx, hy, h - 0.03));
    doors(k, -hx, hx, -hy + 0.02, 0.17, h - 0.05, front);
    k.mat(ids::PLASTER).block(Vec3::new(-hx - 0.01, -hy, h - 0.03), Vec3::new(hx + 0.01, hy, h));
    k.mat(ids::ASPHALT).block(Vec3::new(-hx * 0.6, -hy + 0.08, h), Vec3::new(hx * 0.6, hy - 0.12, h + 0.003));
    let t = Vec3::new(0.0, hy - 0.06, h);
    k.mat(ids::IRON).frustum(t, 0.02, 0.02, 0.18, 8, true);
    k.beam(t + Vec3::Z * 0.18, t + Vec3::new(0.0, -0.12, 0.17), 0.018, 0.018);
    k.mat(front).block(Vec3::new(-hx, hy - 0.14, 1.15), Vec3::new(hx, hy, 1.85));
    k.mat(ids::GLASS).block(Vec3::new(-hx + 0.03, hy - 0.145, 1.18), Vec3::new(hx - 0.03, hy - 0.14, 1.82));
    k.mat(glow).block(Vec3::new(-hx + 0.05, hy - 0.12, 1.88), Vec3::new(hx - 0.05, hy - 0.04, 1.92));
}

fn bathtub(b: &mut Ctx, hx: f32, hy: f32) {
    // A built-in bath with a tiled panel, a rolled rim, water inside,
    // a mixer and shower head on the end wall at +X.
    let k = &mut b.kit;
    let h = 0.55;
    k.mat(ids::PLASTER).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
    k.mat(ids::STUCCO).block(Vec3::new(-hx, -hy - 0.005, 0.0), Vec3::new(hx, -hy, h - 0.05));
    k.mat(ids::WATER).block(Vec3::new(-hx + 0.08, -hy + 0.08, h), Vec3::new(hx - 0.08, hy - 0.08, h + 0.004));
    k.mat(ids::PLASTER);
    for (min, max) in [
        (Vec3::new(-hx, -hy, h), Vec3::new(hx, -hy + 0.08, h + 0.03)),
        (Vec3::new(-hx, hy - 0.08, h), Vec3::new(hx, hy, h + 0.03)),
        (Vec3::new(-hx, -hy, h), Vec3::new(-hx + 0.08, hy, h + 0.03)),
        (Vec3::new(hx - 0.08, -hy, h), Vec3::new(hx, hy, h + 0.03)),
    ] {
        k.block(min, max);
    }
    k.mat(ids::IRON).block(Vec3::new(hx - 0.04, -0.06, h + 0.05), Vec3::new(hx, 0.06, h + 0.15));
    k.beam(Vec3::new(hx - 0.02, 0.0, h + 0.12), Vec3::new(hx - 0.18, 0.0, h + 0.12), 0.025, 0.025);
    k.block(Vec3::new(hx - 0.03, -0.015, h + 0.15), Vec3::new(hx, 0.015, 1.9));
    k.frustum(Vec3::new(hx - 0.15, 0.0, 1.84), 0.07, 0.04, 0.05, 12, true);
    k.beam(Vec3::new(hx - 0.02, 0.0, 1.89), Vec3::new(hx - 0.15, 0.0, 1.89), 0.02, 0.02);
}

fn shower(b: &mut Ctx, hx: f32, hy: f32) {
    // A square shower tray with tiled back walls, a glass enclosure with
    // a door, a riser rail with a head and a mixer.
    let k = &mut b.kit;
    let h = 2.0;
    k.mat(ids::PLASTER).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.06));
    k.mat(ids::IRON).frustum(Vec3::new(0.0, 0.0, 0.06), 0.05, 0.05, 0.003, 12, true);
    k.mat(ids::STUCCO).block(Vec3::new(-hx, hy - 0.05, 0.0), Vec3::new(hx, hy, h + 0.1));
    k.block(Vec3::new(hx - 0.05, -hy, 0.0), Vec3::new(hx, hy - 0.05, h + 0.1));
    k.mat(ids::GLASS).block(Vec3::new(-hx, -hy, 0.06), Vec3::new(-hx + 0.01, hy - 0.05, h));
    k.block(Vec3::new(-hx, -hy, 0.06), Vec3::new(hx - 0.05, -hy + 0.01, h));
    k.mat(ids::IRON);
    for (a, c) in [(Vec3::new(-hx, -hy, h), Vec3::new(hx - 0.05, -hy, h)), (Vec3::new(-hx, -hy, h), Vec3::new(-hx, hy - 0.05, h)), (Vec3::new(-hx, -hy, 0.06), Vec3::new(-hx, -hy, h))] {
        k.beam(a, c, 0.025, 0.025);
    }
    k.block(Vec3::new(0.0, -hy - 0.03, 0.9), Vec3::new(0.02, -hy - 0.01, 1.3));
    k.block(Vec3::new(-0.015, hy - 0.08, 1.0), Vec3::new(0.015, hy - 0.05, 1.95));
    k.frustum(Vec3::new(0.0, hy - 0.2, 1.9), 0.1, 0.1, 0.02, 14, true);
    k.beam(Vec3::new(0.0, hy - 0.06, 1.93), Vec3::new(0.0, hy - 0.2, 1.93), 0.02, 0.02);
    k.block(Vec3::new(-0.08, hy - 0.08, 1.0), Vec3::new(0.08, hy - 0.05, 1.1));
}
