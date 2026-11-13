//! Low-density houses, part one: cottage, bungalow, farmhouse, log cabin,
//! A-frame and beach hut. Ground at z = 0, front (door) facing -Y.

use super::houses::{Roof, Shell, door, lot, rise, roof, walls};
use super::{BuildingKind as K, Ctx, parts};
use crate::generators::kit::{ids, sides};
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec3};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::Cottage => cottage(b, hx, hy, floors),
        K::Bungalow => bungalow(b, hx, hy, floors),
        K::Farmhouse => farmhouse(b, hx, hy, floors),
        K::LogCabin => log_cabin(b, hx, hy),
        K::AFrame => a_frame(b, hx, hy),
        K::BeachHut => beach_hut(b, hx, hy),
        _ => {}
    }
}

/// Front porch with its stairs, centred on the door.
fn porch(b: &mut Ctx, x: f32, w: f32, hy: f32, rise: f32) {
    if b.p.parts.porches {
        let kind = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.02, 0.0)), |k| parts::porch(k, x, w, 1.6, rise, kind));
    }
}

fn chimney(b: &mut Ctx, at: Vec3, h: f32) {
    if b.p.parts.chimneys {
        parts::chimney(&mut b.kit, at, h);
    }
}

fn cottage(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let wall = if b.r.gen_bool(0.5) { ids::STUCCO } else { ids::PLASTER };
    let mut s = Shell::new(hx, hy, floors.min(2), wall);
    (s.fh, s.bay, s.win) = (2.6, 2.0, (0.8, 1.1));
    walls(b, &s);
    let top = s.top();
    roof(&mut b.kit, hx, hy, top, Roof::Gable(48.0), 0.35, wall);
    if b.p.parts.dormers && hx > hy {
        let (w, n) = (1.2, ((hx * 2.0 / 3.2) as usize).max(1));
        let pane = b.pane(0.3);
        for i in 0..n {
            let x = -hx + 2.0 * hx * (i as f32 + 0.5) / n as f32;
            parts::dormer(&mut b.kit, Vec3::new(x, -hy * 0.35, top + rise(hx, hy, 48.0) * 0.25), w, pane);
        }
    }
    chimney(b, Vec3::new(hx * 0.65, hy * 0.25, top), rise(hx, hy, 48.0) + 0.8);
    porch(b, 0.0, 1.8, hy, s.base);
    lot(b, hx, hy, 0.0);
}

fn bungalow(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let mut s = Shell::new(hx, hy, floors.min(2), ids::BRICK);
    (s.fh, s.bay, s.win, s.door) = (2.8, 2.6, (1.4, 1.3), Some(-hx * 0.35));
    walls(b, &s);
    roof(&mut b.kit, hx, hy, s.top(), Roof::Hip(22.0), 0.8, ids::BRICK);
    // A veranda along half the front under its own lean-to.
    if b.p.parts.porches {
        let (x0, x1, d, z) = (-hx + 0.2, 0.4, 2.2, s.base);
        b.kit.mat(ids::TRIM).block(Vec3::new(x0, -hy - d, 0.0), Vec3::new(x1, -hy, z));
        for x in [x0 + 0.1, (x0 + x1) * 0.5, x1 - 0.1] {
            b.kit.block(Vec3::new(x - 0.09, -hy - d + 0.05, z), Vec3::new(x + 0.09, -hy - d + 0.23, z + 2.4));
        }
        b.kit.mat(ids::ROOF).quad(
            Vec3::new(x0 - 0.2, -hy - d - 0.3, z + 2.4),
            Vec3::new(x1 + 0.2, -hy - d - 0.3, z + 2.4),
            Vec3::new(x1 + 0.2, -hy, z + 2.85),
            Vec3::new(x0 - 0.2, -hy, z + 2.85),
        );
    }
    chimney(b, Vec3::new(-hx * 0.5, hy * 0.3, s.top()), rise(hx, hy, 22.0) + 0.9);
    lot(b, hx, hy, -hx * 0.35);
}

fn farmhouse(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let mut s = Shell::new(hx, hy, floors.clamp(1, 3), ids::PLASTER);
    (s.fh, s.bay, s.win) = (2.9, 2.2, (0.95, 1.45));
    walls(b, &s);
    let top = s.top();
    roof(&mut b.kit, hx, hy, top, Roof::Gable(40.0), 0.45, ids::PLASTER);
    // A full-width porch with a shed roof at first-floor height.
    if b.p.parts.porches {
        let (d, z) = (2.4, s.base);
        b.kit.mat(ids::TRIM).block(Vec3::new(-hx, -hy - d, 0.0), Vec3::new(hx, -hy, z));
        let n = ((hx * 2.0 / 2.4) as usize).max(2);
        for i in 0..=n {
            let x = -hx + 0.12 + (2.0 * hx - 0.24) * i as f32 / n as f32;
            b.kit.block(Vec3::new(x - 0.08, -hy - d + 0.05, z), Vec3::new(x + 0.08, -hy - d + 0.21, z + 2.5));
        }
        b.kit.beam(Vec3::new(-hx, -hy - d + 0.13, z + 0.9), Vec3::new(hx, -hy - d + 0.13, z + 0.9), 0.06, 0.1);
        b.kit.mat(ids::ROOF).quad(
            Vec3::new(-hx - 0.2, -hy - d - 0.3, z + 2.5),
            Vec3::new(hx + 0.2, -hy - d - 0.3, z + 2.5),
            Vec3::new(hx + 0.2, -hy, z + 3.0),
            Vec3::new(-hx - 0.2, -hy, z + 3.0),
        );
        let kind = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - d, 0.0)) * Mat4::from_rotation_z(std::f32::consts::PI), |k| {
            super::arch::stairs(k, kind, 1.4, z);
        });
    }
    let r = rise(hx, hy, 40.0);
    chimney(b, Vec3::new(-hx + 0.6, 0.0, top), r + 0.8);
    chimney(b, Vec3::new(hx - 0.6, 0.0, top), r + 0.8);
    lot(b, hx, hy, 0.0);
}

fn log_cabin(b: &mut Ctx, hx: f32, hy: f32) {
    let (h, lr) = (2.7, 0.15);
    b.kit.mat(ids::CURB).block(Vec3::new(-hx - 0.1, -hy - 0.1, 0.0), Vec3::new(hx + 0.1, hy + 0.1, 0.3));
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx + 0.05, -hy + 0.05, 0.3), Vec3::new(hx - 0.05, hy - 0.05, 0.3 + h));
    // Stacked logs that cross at the corners.
    let n = (h / (lr * 2.0)) as usize;
    for i in 0..n {
        let z = 0.3 + lr + i as f32 * lr * 2.0;
        let (ex, ey) = if i % 2 == 0 { (0.35, 0.0) } else { (0.0, 0.35) };
        for s in [-1.0, 1.0] {
            b.kit.tube(Vec3::new(-hx - ex, s * hy, z), Vec3::new(hx + ex, s * hy, z), lr, 6);
            b.kit.tube(Vec3::new(s * hx, -hy - ey, z + lr * 0.5), Vec3::new(s * hx, hy + ey, z + lr * 0.5), lr, 6);
        }
    }
    let pane = b.pane(0.5);
    for x in [-hx * 0.55, hx * 0.55] {
        b.kit.mat(ids::TRIM).block(Vec3::new(x - 0.55, -hy - 0.2, 1.05), Vec3::new(x + 0.55, -hy - 0.1, 2.15));
        crate::generators::settlement::window(&mut b.kit, x, -hy - 0.21, 1.15, 0.9, 0.9, pane);
    }
    door(&mut b.kit, 0.0, -hy - 0.14, 0.3, 0.95, 2.05);
    roof(&mut b.kit, hx + 0.1, hy + 0.1, 0.3 + h, Roof::Gable(35.0), 0.5, ids::FACADE);
    // A fieldstone chimney up the side wall.
    if b.p.parts.chimneys {
        let top = 0.3 + h + rise(hx, hy, 35.0) + 0.8;
        b.kit.mat(ids::COBBLE).block(Vec3::new(hx + 0.15, -0.6, 0.0), Vec3::new(hx + 1.0, 0.6, 2.2));
        b.kit.block(Vec3::new(hx + 0.2, -0.4, 2.2), Vec3::new(hx + 0.8, 0.4, top));
    }
    porch(b, 0.0, 2.2, hy + 0.15, 0.3);
    lot(b, hx + 0.3, hy + 0.3, 0.0);
}

fn a_frame(b: &mut Ctx, hx: f32, hy: f32) {
    // The roof comes down to the ground; ridge runs front to back.
    let r = hx * 1.9;
    b.kit.mat(ids::CURB).block(Vec3::new(-hx - 0.2, -hy - 0.2, 0.0), Vec3::new(hx + 0.2, hy + 0.2, 0.25));
    let glass = b.pane(0.6);
    b.kit.at(Mat4::from_translation(Vec3::Z * 0.25) * Mat4::from_rotation_z(FRAC_PI_2), |k| {
        k.mat(ids::ROOF).gable(hy, hx, r, 0.0, glass);
    });
    // Mullions and a cross beam over the glass front, a door and a deck.
    b.kit.mat(ids::TRIM);
    let y = -hy - 0.06;
    b.kit.beam(Vec3::new(-hx * 0.86, y, 0.25 + r * 0.14), Vec3::new(hx * 0.86, y, 0.25 + r * 0.14), 0.1, 0.12);
    b.kit.beam(Vec3::new(-hx * 0.45, y, 0.25 + r * 0.55), Vec3::new(hx * 0.45, y, 0.25 + r * 0.55), 0.1, 0.12);
    for x in [-hx * 0.5, hx * 0.5] {
        b.kit.beam(Vec3::new(x, y, 0.25), Vec3::new(x, y, 0.25 + r * (1.0 - x.abs() / hx) - 0.1), 0.1, 0.1);
    }
    b.kit.beam(Vec3::new(0.0, y, 0.25 + 2.3), Vec3::new(0.0, y, 0.25 + r - 0.15), 0.1, 0.1);
    door(&mut b.kit, 0.0, -hy - 0.04, 0.25, 1.0, 2.1);
    if b.p.parts.porches {
        b.kit.mat(ids::TRIM).block(Vec3::new(-hx, -hy - 2.6, 0.0), Vec3::new(hx, -hy - 0.2, 0.25));
        for x in [-hx, hx] {
            b.kit.beam(Vec3::new(x, -hy - 2.6, 1.15), Vec3::new(x, -hy - 0.2, 1.15), 0.06, 0.06);
        }
        b.kit.beam(Vec3::new(-hx, -hy - 2.6, 1.15), Vec3::new(hx, -hy - 2.6, 1.15), 0.06, 0.06);
    }
    chimney(b, Vec3::new(hx * 0.25, hy * 0.5, 0.25 + r * 0.7), r * 0.3 + 0.7);
    lot(b, hx + 0.3, hy + 2.6, 0.0);
}

fn beach_hut(b: &mut Ctx, hx: f32, hy: f32) {
    // Painted plank hut on a low deck with double doors.
    let walls_id = [ids::STUCCO, ids::PLASTER, ids::FACADE, ids::TRIM][b.r.gen_range(0..4)];
    let h = 2.3;
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.1, -hy - 0.9, 0.0), Vec3::new(hx + 0.1, hy + 0.1, 0.25));
    b.kit.mat(walls_id).block(Vec3::new(-hx, -hy, 0.25), Vec3::new(hx, hy, 0.25 + h));
    // Plank seams: thin raised strips across the front.
    let n = ((hx * 2.0 / 0.18) as usize).max(4);
    for i in 1..n {
        let x = -hx + 2.0 * hx * i as f32 / n as f32;
        b.kit.block(Vec3::new(x - 0.012, -hy - 0.015, 0.25), Vec3::new(x + 0.012, -hy, 0.25 + h));
    }
    for s in [-1.0, 1.0] {
        let x = s * hx * 0.27;
        b.kit.mat(ids::TRIM).block(Vec3::new(x - hx * 0.26, -hy - 0.04, 0.3), Vec3::new(x + hx * 0.26, -hy - 0.015, 2.2));
    }
    // A small window in each side wall.
    let pane = b.pane(0.2);
    sides(&mut b.kit, hx, hy, |k, side, _| {
        if side % 2 == 1 {
            window(k, 0.0, 0.0, 1.2, 0.6, 0.6, pane);
        }
    });
    roof(&mut b.kit, hx, hy, 0.25 + h, Roof::Gable(32.0), 0.25, walls_id);
}
