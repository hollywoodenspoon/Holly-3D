//! Appliances, part one: the big ones. Fridge, cooker, dishwasher,
//! washing machine, air conditioner, radiator, fan, vending machine and
//! arcade cabinet. Floor at z = 0, front facing -Y.

use super::furniture::cushion;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Fridge => fridge(b, hx, hy),
        K::Cooker => cooker(b, hx, hy),
        K::Dishwasher => dishwasher(b, hx, hy),
        K::WashingMachine => washer(b, hx, hy),
        K::AirConditioner => air_con(b, hx, hy),
        K::Radiator => radiator(b, hx, hy),
        K::Fan => fan(b),
        K::VendingMachine => vending(b, hx, hy),
        K::ArcadeCabinet => arcade(b, hx, hy),
        _ => {}
    }
}

/// White, steel or black appliance shell (by seed).
pub(super) fn shell(b: &mut Ctx) -> u16 {
    [ids::PLASTER, ids::PLASTER, ids::IRON, ids::ASPHALT][b.r.gen_range(0..4)]
}

/// A round dial on the -Y face at `p`.
pub(super) fn dial(k: &mut Kit, p: Vec3, r: f32) {
    k.at(Mat4::from_translation(p) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.mat(ids::ASPHALT).frustum(Vec3::ZERO, r, r * 0.9, 0.02, 12, true));
}

fn fridge(b: &mut Ctx, hx: f32, hy: f32) {
    // A tall fridge-freezer: freezer below, fridge above, or side by
    // side when wide; long bar handles and a water dispenser.
    let body = shell(b);
    let k = &mut b.kit;
    let h = 1.85;
    k.mat(body);
    cushion(k, Vec3::new(-hx, -hy + 0.04, 0.02), Vec3::new(hx, hy, h));
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.03, -hy + 0.05, 0.0), Vec3::new(hx - 0.03, -hy + 0.1, 0.05));
    let fy = -hy + 0.04;
    k.mat(body);
    if hx > 0.42 {
        for (x0, x1) in [(-hx, -0.005), (0.005, hx)] {
            cushion(k, Vec3::new(x0, fy - 0.04, 0.05), Vec3::new(x1, fy, h - 0.01));
        }
        for x in [-0.05, 0.05] {
            k.mat(ids::IRON).block(Vec3::new(x - 0.012, fy - 0.09, 0.7), Vec3::new(x + 0.012, fy - 0.04, 1.5));
        }
        k.mat(ids::ASPHALT).block(Vec3::new(-hx * 0.7, fy - 0.045, 1.1), Vec3::new(-hx * 0.3, fy - 0.04, 1.4));
    } else {
        cushion(k, Vec3::new(-hx, fy - 0.04, 0.05), Vec3::new(hx, fy, 0.7));
        cushion(k, Vec3::new(-hx, fy - 0.04, 0.72), Vec3::new(hx, fy, h - 0.01));
        k.mat(ids::IRON).block(Vec3::new(hx - 0.07, fy - 0.09, 0.45), Vec3::new(hx - 0.045, fy - 0.04, 0.68));
        k.block(Vec3::new(hx - 0.07, fy - 0.09, 0.8), Vec3::new(hx - 0.045, fy - 0.04, 1.4));
    }
    // A couple of magnets and a note.
    k.mat(ids::ROOF).block(Vec3::new(-hx * 0.5, fy - 0.05, 1.5), Vec3::new(-hx * 0.4, fy - 0.04, 1.55));
    k.mat(ids::STUCCO).block(Vec3::new(-hx * 0.6, fy - 0.045, 1.3), Vec3::new(-hx * 0.3, fy - 0.04, 1.5));
}

fn cooker(b: &mut Ctx, hx: f32, hy: f32) {
    // A freestanding cooker: oven door with a window and handle, a
    // control panel with dials, a hob with four rings and pans.
    let body = shell(b);
    let k = &mut b.kit;
    let h = 0.9;
    k.mat(body).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.02, -hy + 0.02, h), Vec3::new(hx - 0.02, hy - 0.02, h + 0.01));
    k.mat(body).block(Vec3::new(-hx, hy - 0.06, h), Vec3::new(hx, hy, h + 0.12));
    for i in 0..4 {
        let x = -hx + (i as f32 + 0.5) * hx * 0.5;
        dial(k, Vec3::new(x, -hy - 0.01, h - 0.06), 0.025);
    }
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.06, -hy - 0.005, 0.15), Vec3::new(hx - 0.06, -hy, h - 0.14));
    k.mat(ids::GLASS).block(Vec3::new(-hx + 0.12, -hy - 0.008, 0.25), Vec3::new(hx - 0.12, -hy - 0.005, h - 0.25));
    k.mat(ids::IRON).beam(Vec3::new(-hx + 0.1, -hy - 0.05, h - 0.17), Vec3::new(hx - 0.1, -hy - 0.05, h - 0.17), 0.02, 0.02);
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.06, -hy - 0.005, 0.03), Vec3::new(hx - 0.06, -hy, 0.12));
    for (i, (dx, dy)) in [(-0.14, -0.12), (0.14, -0.12), (-0.14, 0.12), (0.14, 0.12)].into_iter().enumerate() {
        let p = Vec3::new(dx * hx / 0.3, dy * hy / 0.3, h + 0.01);
        k.mat(ids::IRON).frustum(p, 0.08, 0.08, 0.008, 14, true);
        if i == 1 {
            k.frustum(p + Vec3::Z * 0.008, 0.1, 0.1, 0.1, 16, false);
            k.beam(p + Vec3::new(0.1, 0.0, 0.08), p + Vec3::new(0.3, 0.0, 0.09), 0.025, 0.015);
        } else if i == 2 {
            k.mat(ids::ROOF).frustum(p + Vec3::Z * 0.008, 0.09, 0.08, 0.14, 16, true);
        }
    }
}

fn dishwasher(b: &mut Ctx, hx: f32, hy: f32) {
    // A freestanding dishwasher: a smooth door, top control strip with
    // buttons and a light, a recessed handle and a kick plate.
    let body = shell(b);
    let glow = b.glow(true);
    let k = &mut b.kit;
    let h = 0.85;
    k.mat(body).block(Vec3::new(-hx, -hy + 0.02, 0.0), Vec3::new(hx, hy, h));
    cushion(k, Vec3::new(-hx, -hy, 0.1), Vec3::new(hx, -hy + 0.02, h - 0.1));
    k.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, h - 0.1), Vec3::new(hx, -hy + 0.02, h));
    for i in 0..5 {
        let x = -hx * 0.3 + i as f32 * hx * 0.15;
        k.mat(ids::IRON).block(Vec3::new(x - 0.012, -hy - 0.005, h - 0.06), Vec3::new(x + 0.012, -hy, h - 0.04));
    }
    k.mat(glow).block(Vec3::new(hx * 0.6, -hy - 0.005, h - 0.06), Vec3::new(hx * 0.7, -hy, h - 0.04));
    k.mat(ids::IRON).block(Vec3::new(-hx * 0.6, -hy - 0.04, h - 0.17), Vec3::new(hx * 0.6, -hy, h - 0.14));
    k.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy + 0.04, 0.0), Vec3::new(hx, -hy + 0.06, 0.1));
}

fn washer(b: &mut Ctx, hx: f32, hy: f32) {
    // A front-loading washing machine (or dryer, by seed): round door
    // with a glass porthole, a detergent drawer, programme dial and a
    // display.
    let body = shell(b);
    let dryer = b.r.gen_bool(0.3);
    let glow = b.glow(true);
    let k = &mut b.kit;
    let h = 0.85;
    k.mat(body);
    cushion(k, Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
    k.mat(ids::STUCCO).block(Vec3::new(-hx + 0.03, -hy - 0.005, h - 0.14), Vec3::new(hx - 0.03, -hy, h - 0.02));
    k.mat(body).block(Vec3::new(-hx + 0.04, -hy - 0.02, h - 0.13), Vec3::new(-hx + 0.22, -hy, h - 0.04));
    dial(k, Vec3::new(hx * 0.5, -hy - 0.005, h - 0.08), 0.035);
    k.mat(glow).block(Vec3::new(-0.04, -hy - 0.008, h - 0.11), Vec3::new(0.08, -hy - 0.005, h - 0.06));
    let c = Vec3::new(0.0, -hy, 0.42);
    k.at(Mat4::from_translation(c) * Mat4::from_rotation_x(FRAC_PI_2), |k| {
        k.mat(ids::IRON).frustum(Vec3::ZERO, 0.22, 0.2, 0.05, 24, true);
        k.mat(if dryer { ids::ASPHALT } else { ids::GLASS }).frustum(Vec3::new(0.0, 0.0, 0.05), 0.16, 0.14, 0.01, 24, true);
    });
    k.mat(ids::IRON).block(c + Vec3::new(0.18, -0.07, -0.06), c + Vec3::new(0.22, -0.05, 0.06));
}

fn air_con(b: &mut Ctx, hx: f32, hy: f32) {
    // A split air conditioner: a wall-mounted indoor unit at 2.1 m with
    // louvres, and the outdoor condenser on feet with a fan grille.
    let glow = b.glow(true);
    let k = &mut b.kit;
    let (w, y) = (hx.min(0.45), hy - 0.12);
    k.mat(ids::PLASTER);
    cushion(k, Vec3::new(-w, y - 0.12, 2.0), Vec3::new(w, hy, 2.3));
    k.mat(ids::STUCCO);
    for i in 0..4 {
        let z = 2.03 + i as f32 * 0.025;
        k.block(Vec3::new(-w + 0.03, y - 0.13, z), Vec3::new(w - 0.03, y - 0.1, z + 0.012));
    }
    k.mat(glow).block(Vec3::new(w - 0.12, y - 0.125, 2.22), Vec3::new(w - 0.08, y - 0.12, 2.24));
    // Outdoor unit on the floor beside it.
    let o = Vec3::new(-hx + 0.45, -hy + 0.2, 0.0);
    k.mat(ids::IRON);
    for x in [-0.35, 0.35] {
        k.block(o + Vec3::new(x - 0.03, -0.15, 0.0), o + Vec3::new(x + 0.03, 0.15, 0.06));
    }
    k.mat(ids::PLASTER).block(o + Vec3::new(-0.42, -0.15, 0.06), o + Vec3::new(0.42, 0.15, 0.66));
    k.at(Mat4::from_translation(o + Vec3::new(-0.08, -0.15, 0.36)) * Mat4::from_rotation_x(FRAC_PI_2), |k| {
        k.mat(ids::ASPHALT).frustum(Vec3::ZERO, 0.25, 0.25, 0.01, 20, true);
        k.mat(ids::IRON);
        for i in 0..4 {
            let d = Vec2::from_angle(i as f32 * TAU / 4.0);
            k.beam(Vec3::new(0.0, 0.0, 0.012), (d * 0.24).extend(0.012), 0.015, 0.005);
        }
    });
    k.mat(ids::IRON).tube(o + Vec3::new(0.3, 0.15, 0.4), Vec3::new(o.x + 0.3, y, 2.1), 0.02, 6);
}

fn radiator(b: &mut Ctx, hx: f32, hy: f32) {
    // A panel radiator on wall brackets with vertical fins and a valve.
    let k = &mut b.kit;
    let y = hy - 0.06;
    let (z0, z1) = (0.15, 0.75);
    k.mat(ids::PLASTER).block(Vec3::new(-hx, y - 0.04, z0), Vec3::new(hx, y, z1));
    let n = ((hx * 2.0 / 0.05) as usize).min(60);
    for i in 0..n {
        let x = -hx + 0.025 + i as f32 * (hx * 2.0 - 0.05) / (n - 1).max(1) as f32;
        k.block(Vec3::new(x - 0.008, y - 0.055, z0 + 0.02), Vec3::new(x + 0.008, y - 0.04, z1 - 0.02));
    }
    k.mat(ids::IRON);
    for x in [-hx + 0.15, hx - 0.15] {
        k.block(Vec3::new(x - 0.02, y, z1 - 0.12), Vec3::new(x + 0.02, hy, z1 - 0.05));
    }
    k.tube(Vec3::new(hx + 0.02, y - 0.02, z0 + 0.05), Vec3::new(hx + 0.02, y - 0.02, 0.0), 0.012, 6);
    k.frustum(Vec3::new(hx + 0.02, y - 0.02, z0 + 0.03), 0.025, 0.025, 0.06, 8, true);
}

fn fan(b: &mut Ctx) {
    // A pedestal fan: round base, telescopic pole, motor housing, three
    // blades in a cage.
    let body = [ids::PLASTER, ids::ASPHALT, ids::SLATE][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    k.mat(body).frustum(Vec3::ZERO, 0.2, 0.17, 0.04, 16, true);
    k.frustum(Vec3::new(0.0, 0.0, 0.04), 0.02, 0.015, 1.0, 8, true);
    let c = Vec3::new(0.0, -0.05, 1.12);
    k.at(Mat4::from_translation(c) * Mat4::from_rotation_x(FRAC_PI_2), |k| {
        k.mat(body).frustum(Vec3::new(0.0, 0.0, -0.18), 0.06, 0.07, 0.16, 12, true);
        for i in 0..3 {
            let a = i as f32 * TAU / 3.0;
            let (d, s) = (Vec2::from_angle(a), Vec2::from_angle(a + 0.6));
            k.mat(ids::GLASS).poly(&[Vec3::new(0.0, 0.0, 0.02), (d * 0.2).extend(0.0), (s * 0.18).extend(0.0)]);
            k.poly(&[Vec3::new(0.0, 0.0, 0.02), (s * 0.18).extend(0.0), (d * 0.2).extend(0.0)]);
        }
        k.mat(ids::IRON);
        for i in 0..16 {
            let (a, c2) = (Vec2::from_angle(i as f32 * TAU / 16.0) * 0.23, Vec2::from_angle((i + 1) as f32 * TAU / 16.0) * 0.23);
            k.beam(a.extend(0.0), c2.extend(0.0), 0.008, 0.03);
        }
        for i in 0..8 {
            k.beam(Vec3::new(0.0, 0.0, 0.05), (Vec2::from_angle(i as f32 * TAU / 8.0) * 0.23).extend(0.0), 0.005, 0.005);
        }
    });
}

fn vending(b: &mut Ctx, hx: f32, hy: f32) {
    // A drinks/snack vending machine: lit glass front with rows of
    // products, a keypad, a coin slot and a pickup flap.
    let paint = [ids::ROOF, ids::SLATE, ids::ASPHALT][b.r.gen_range(0..3)];
    let glow = b.glow(false);
    let k = &mut b.kit;
    let h = 1.85;
    let gx = hx * 0.25;
    // A cabinet with an open display bay on the left of the front.
    let (z0, z1, back) = (0.46, h - 0.08, -hy + 0.52);
    k.mat(paint).block(Vec3::new(-hx, back, 0.0), Vec3::new(hx, hy, h));
    k.block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, back, z0));
    k.block(Vec3::new(-hx, -hy, z1), Vec3::new(hx, back, h));
    k.block(Vec3::new(-hx, -hy, z0), Vec3::new(-hx + 0.04, back, z1));
    k.block(Vec3::new(gx + 0.02, -hy, z0), Vec3::new(hx, back, z1));
    k.mat(glow).block(Vec3::new(-hx + 0.04, back - 0.01, z0), Vec3::new(gx + 0.02, back, z1));
    let colours = [ids::ROOF, ids::STUCCO, ids::PLASTER, ids::SLATE, ids::FOLIAGE];
    for row in 0..5 {
        let z = 0.55 + row as f32 * 0.24;
        k.mat(ids::IRON).block(Vec3::new(-hx + 0.06, -hy + 0.06, z - 0.01), Vec3::new(gx, -hy + 0.5, z));
        let n = (((gx + hx - 0.06) / 0.1) as usize).max(1);
        for i in 0..n {
            let x = -hx + 0.11 + i as f32 * 0.1;
            k.mat(colours[(row + i) % colours.len()]).frustum(Vec3::new(x, -hy + 0.12, z), 0.03, 0.03, 0.16, 8, true);
        }
    }
    // Glass only on the frame edges, so the products show in renders.
    k.mat(ids::GLASS);
    for (x0, x1) in [(-hx + 0.04, -hx + 0.06), (gx, gx + 0.02)] {
        k.block(Vec3::new(x0, -hy - 0.005, 0.48), Vec3::new(x1, -hy + 0.04, h - 0.08));
    }
    k.mat(ids::ASPHALT).block(Vec3::new(gx + 0.08, -hy - 0.01, 0.9), Vec3::new(hx - 0.06, -hy, 1.4));
    for r in 0..4 {
        for c in 0..3 {
            let p = Vec3::new(gx + 0.13 + c as f32 * 0.05, -hy - 0.015, 1.0 + r as f32 * 0.05);
            k.mat(ids::IRON).block(p - Vec3::new(0.015, 0.0, 0.015), p + Vec3::new(0.015, 0.005, 0.015));
        }
    }
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.15, -hy - 0.01, 0.15), Vec3::new(gx - 0.1, -hy, 0.38));
}

fn arcade(b: &mut Ctx, hx: f32, hy: f32) {
    // An upright arcade cabinet: side panels with a profile, a lit
    // marquee, a tilted screen, a control deck with a stick and buttons
    // and a coin door.
    let paint = [ids::ROOF, ids::SLATE, ids::ASPHALT, ids::FACADE][b.r.gen_range(0..4)];
    let (glow, neon) = (b.glow(false), b.glow(true));
    let k = &mut b.kit;
    let side = [Vec2::new(-hy, 0.0), Vec2::new(hy, 0.0), Vec2::new(hy, 1.8), Vec2::new(-hy + 0.15, 1.8), Vec2::new(-hy + 0.05, 1.5), Vec2::new(-hy + 0.2, 1.1), Vec2::new(-hy - 0.05, 0.95), Vec2::new(-hy, 0.85)];
    for x in [-hx, hx - 0.03] {
        k.at(Mat4::from_translation(Vec3::X * x) * Mat4::from_rotation_z(FRAC_PI_2) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.mat(paint).prism(&side, 0.0, 0.03, true, true));
    }
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.03, -hy, 0.0), Vec3::new(hx - 0.03, hy, 0.85));
    k.block(Vec3::new(-hx + 0.03, hy - 0.05, 0.85), Vec3::new(hx - 0.03, hy, 1.8));
    k.mat(neon).block(Vec3::new(-hx + 0.03, -hy + 0.15, 1.52), Vec3::new(hx - 0.03, -hy + 0.17, 1.75));
    k.mat(paint).block(Vec3::new(-hx + 0.03, -hy + 0.15, 1.75), Vec3::new(hx - 0.03, hy, 1.8));
    k.quad(Vec3::new(-hx + 0.03, -hy + 0.2, 1.1), Vec3::new(hx - 0.03, -hy + 0.2, 1.1), Vec3::new(hx - 0.03, -hy + 0.05, 1.5), Vec3::new(-hx + 0.03, -hy + 0.05, 1.5));
    k.mat(glow).quad(Vec3::new(-hx + 0.08, -hy + 0.17, 1.15), Vec3::new(hx - 0.08, -hy + 0.17, 1.15), Vec3::new(hx - 0.08, -hy + 0.04, 1.47), Vec3::new(-hx + 0.08, -hy + 0.04, 1.47));
    k.mat(paint).block(Vec3::new(-hx + 0.03, -hy - 0.05, 0.85), Vec3::new(hx - 0.03, -hy + 0.2, 0.95));
    k.mat(ids::IRON).frustum(Vec3::new(-hx * 0.5, -hy + 0.05, 0.95), 0.012, 0.012, 0.07, 6, true);
    k.mat(ids::ROOF).frustum(Vec3::new(-hx * 0.5, -hy + 0.05, 1.02), 0.025, 0.025, 0.03, 8, true);
    for (i, c) in [ids::ROOF, ids::STUCCO, ids::SLATE, ids::FOLIAGE].into_iter().enumerate() {
        let x = hx * 0.05 + (i % 2) as f32 * 0.07 + (i / 2) as f32 * 0.03;
        k.mat(c).frustum(Vec3::new(x, -hy + 0.02 + (i / 2) as f32 * 0.07, 0.95), 0.02, 0.02, 0.012, 8, true);
    }
    k.mat(ids::IRON).block(Vec3::new(-0.12, -hy - 0.005, 0.35), Vec3::new(0.12, -hy, 0.6));
    k.mat(neon).block(Vec3::new(-0.06, -hy - 0.01, 0.48), Vec3::new(-0.02, -hy - 0.005, 0.53));
    k.block(Vec3::new(0.02, -hy - 0.01, 0.48), Vec3::new(0.06, -hy - 0.005, 0.53));
}
