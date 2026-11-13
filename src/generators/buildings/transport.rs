//! Transport prefabs: bus stop, bus depot, train station, metro entrance,
//! tram stop, airport terminal, control tower and harbour office.
//! Ground at z = 0 (the metro stairs go below it), front facing -Y.

use super::houses::{Roof, door, framed, roof};
use super::industry::{dock_door, ladder};
use super::services::flag;
use super::shops::{canopy, flat_box, planters, sign, storefront};
use super::{BuildingKind as K, Ctx, massing};
use crate::generators::kit::roofs::arch_curve;
use crate::generators::kit::{Kit, ids};
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec3};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::BusStop => bus_stop(b, hx, hy),
        K::BusDepot => bus_depot(b, hx, hy),
        K::TrainStation => train_station(b, hx, hy),
        K::MetroEntrance => metro(b, hx, hy),
        K::TramStop => tram_stop(b, hx, hy),
        K::AirportTerminal => airport(b, hx, hy),
        K::ControlTower => control_tower(b, hx, floors),
        K::HarbourOffice => harbour(b, hx, hy, floors),
        _ => {}
    }
}

/// A track along X from `x0` to `x1` at `y`: sleepers and two rails.
fn track(k: &mut Kit, x0: f32, x1: f32, y: f32, z: f32) {
    k.mat(ids::DUST).block(Vec3::new(x0, y - 1.5, z), Vec3::new(x1, y + 1.5, z + 0.2));
    k.mat(ids::TRIM);
    let n = ((x1 - x0) / 0.7) as usize;
    for i in 0..n {
        let x = x0 + 0.35 + i as f32 * 0.7;
        k.block(Vec3::new(x - 0.12, y - 1.2, z + 0.2), Vec3::new(x + 0.12, y + 1.2, z + 0.32));
    }
    k.mat(ids::IRON);
    for s in [-0.72, 0.72] {
        k.block(Vec3::new(x0, y + s - 0.04, z + 0.32), Vec3::new(x1, y + s + 0.04, z + 0.48));
    }
}

/// A bus `len` long centred at `c` along X: body, window band, wheels.
fn bus(k: &mut Kit, c: Vec3, len: f32, paint: u16) {
    let (hl, hw) = (len * 0.5, 1.25);
    k.mat(paint).block(c + Vec3::new(-hl, -hw, 0.4), c + Vec3::new(hl, hw, 3.1));
    k.mat(ids::GLASS).block(c + Vec3::new(-hl + 0.3, -hw - 0.02, 1.6), c + Vec3::new(hl - 0.3, hw + 0.02, 2.7));
    k.block(c + Vec3::new(hl, -hw + 0.15, 1.2), c + Vec3::new(hl + 0.02, hw - 0.15, 2.8));
    k.mat(ids::ASPHALT);
    for x in [-hl + 1.8, hl - 2.2] {
        for s in [-1.0, 1.0] {
            k.at(Mat4::from_translation(c + Vec3::new(x, s * (hw - 0.1), 0.5)) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.frustum(Vec3::Z * -0.15, 0.5, 0.5, 0.3, 10, true));
        }
    }
}

fn bus_stop(b: &mut Ctx, hx: f32, hy: f32) {
    // Kerbed pavement, a glass shelter with a roof and bench, a timetable
    // and a stop sign on a pole.
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.15));
    b.kit.mat(ids::CURB).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, -hy + 0.25, 0.2));
    let (sx, y0, y1, h) = (hx * 0.6, -hy * 0.3, hy - 0.2, 2.5);
    let k = &mut b.kit;
    k.mat(ids::GLASS).block(Vec3::new(-sx, y1 - 0.05, 0.4), Vec3::new(sx, y1, h));
    for s in [-1.0, 1.0] {
        k.block(Vec3::new(s * sx - 0.03, y0 + 0.4, 0.4), Vec3::new(s * sx + 0.03, y1, h));
        k.mat(ids::IRON).block(Vec3::new(s * sx - 0.06, y0, 0.15), Vec3::new(s * sx + 0.06, y0 + 0.12, h));
        k.block(Vec3::new(s * sx - 0.06, y1 - 0.12, 0.15), Vec3::new(s * sx + 0.06, y1, h));
        k.mat(ids::GLASS);
    }
    k.mat(ids::TRIM).block(Vec3::new(-sx - 0.2, y0 - 0.3, h), Vec3::new(sx + 0.2, y1 + 0.1, h + 0.15));
    k.mat(ids::IRON).block(Vec3::new(-sx * 0.7, y1 - 0.5, 0.45), Vec3::new(sx * 0.7, y1 - 0.1, 0.5));
    let g = b.glow(false);
    let k = &mut b.kit;
    k.mat(g).block(Vec3::new(sx - 1.2, y1 - 0.12, 0.8), Vec3::new(sx - 0.1, y1 - 0.06, 2.2));
    let p = Vec3::new(-hx + 0.5, -hy + 0.6, 0.15);
    k.mat(ids::IRON).frustum(p, 0.05, 0.05, 2.8, 6, true);
    k.mat(ids::ROOF).at(Mat4::from_translation(p + Vec3::Z * 2.8) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.frustum(Vec3::Z * -0.03, 0.3, 0.3, 0.06, 14, true));
}

fn bus_depot(b: &mut Ctx, hx: f32, hy: f32) {
    // A long gabled garage with roller doors, buses in the forecourt and
    // inside, an office block and a fuel island.
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    let (y0, h) = (-hy + 16.0, 8.0);
    let (gx, gy) = (hx - 8.0, (hy - y0) * 0.5);
    let c = Vec3::new(-8.0 + 0.0, y0 + gy, 0.0);
    b.kit.mat(ids::BRICK).block(c - Vec3::new(gx, gy, 0.0), c + Vec3::new(gx, gy, h));
    b.kit.at(Mat4::from_translation(c + Vec3::Z * h) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::RUST).gable(gy, gx, 3.0, 0.3, ids::BRICK));
    let n = ((gx * 2.0 / 5.0) as usize).max(2);
    for i in 0..n {
        let x = c.x - gx + 2.0 * gx * (i as f32 + 0.5) / n as f32;
        b.kit.mat(ids::RUST).quad(Vec3::new(x - 1.8, y0 - 0.03, 0.05), Vec3::new(x + 1.8, y0 - 0.03, 0.05), Vec3::new(x + 1.8, y0 - 0.03, 4.6), Vec3::new(x - 1.8, y0 - 0.03, 4.6));
        b.kit.mat(ids::LANE).quad(Vec3::new(x - 0.06, -hy + 2.0, 0.06), Vec3::new(x + 0.06, -hy + 2.0, 0.06), Vec3::new(x + 0.06, y0, 0.06), Vec3::new(x - 0.06, y0, 0.06));
    }
    for i in 0..n {
        if b.r.gen_bool(0.4) {
            continue;
        }
        let x = c.x - gx + 2.0 * gx * (i as f32 + 0.5) / n as f32;
        let paint = [ids::ROOF, ids::STUCCO, ids::PLASTER][i % 3];
        b.kit.at(Mat4::from_translation(Vec3::new(x, -hy + 2.0 + 6.5, 0.05)) * Mat4::from_rotation_z(FRAC_PI_2), |k| bus(k, Vec3::ZERO, 12.0, paint));
    }
    flat_box(&mut b.kit, hx - 15.0, -hy + 2.0, hx - 1.0, -hy + 10.0, 0.0, 6.5, ids::PLASTER, 0.5);
    storefront(b, hx - 8.0, 8.0, -hy + 2.0, 0.05, 2.5, true);
    sign(b, hx - 8.0, -hy + 2.0, 4.8, 8.0);
    let f = Vec3::new(hx - 4.0, y0 + 6.0, 0.0);
    let k = &mut b.kit;
    k.mat(ids::CURB).block(f - Vec3::new(0.8, 4.0, 0.0), f + Vec3::new(0.8, 4.0, 0.2));
    k.mat(ids::IRON).block(f + Vec3::new(-0.4, -0.5, 0.2), f + Vec3::new(0.4, 0.5, 1.8));
    k.mat(ids::TRIM).block(f + Vec3::new(-3.0, -5.0, 5.0), f + Vec3::new(3.0, 5.0, 5.4));
    for y in [-4.6, 4.6] {
        k.mat(ids::IRON).block(f + Vec3::new(-0.15, y - 0.15, 0.2), f + Vec3::new(0.15, y + 0.15, 5.0));
    }
}

fn train_station(b: &mut Ctx, hx: f32, hy: f32) {
    // A station building with a barrel-vaulted hall, wings and a clock;
    // behind it two platforms under canopies with tracks.
    b.kit.mat(ids::COBBLE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, -hy + 5.0, 0.08));
    let (y0, y1) = (-hy + 5.0, -hy + 15.0);
    let wall = if b.r.gen_bool(0.5) { ids::BRICK } else { ids::STUCCO };
    for s in [-1.0f32, 1.0] {
        let (x0, x1) = if s < 0.0 { (-hx, -7.0) } else { (7.0, hx) };
        let dx = (x1 - x0) * 0.5;
        framed(b, Mat4::from_translation(Vec3::new((x0 + x1) * 0.5, (y0 + y1) * 0.5, 0.0)), |b| {
            massing(b, dx, (y1 - y0) * 0.5, 0.0, 2, 4.0, wall, 2.4, 1.2, 2.0, 0.5);
            roof(&mut b.kit, dx, (y1 - y0) * 0.5, 8.0, Roof::Hip(30.0), 0.4, wall);
        });
    }
    // Barrel-vaulted hall: arch walls front and back plus the vault.
    let curve = arch_curve(14.0, 6.0, false, 12);
    let (zh, k) = (8.0, &mut b.kit);
    k.mat(wall).block(Vec3::new(-7.0, y0, 0.0), Vec3::new(-6.2, y1, zh));
    k.block(Vec3::new(6.2, y0, 0.0), Vec3::new(7.0, y1, zh));
    for w in curve.windows(2) {
        let (p, q) = (w[0], w[1]);
        k.mat(ids::SLATE).quad(Vec3::new(p.x, y1, zh + p.y), Vec3::new(q.x, y1, zh + q.y), Vec3::new(q.x, y0, zh + q.y), Vec3::new(p.x, y0, zh + p.y));
        let pane = ids::GLASS;
        k.mat(pane).poly(&[Vec3::new(p.x, y0 - 0.02, zh + p.y), Vec3::new(q.x, y0 - 0.02, zh + q.y), Vec3::new(q.x, y0 - 0.02, zh), Vec3::new(p.x, y0 - 0.02, zh)]);
    }
    storefront(b, 0.0, 12.0, y0, 0.08, zh - 0.5, true);
    let k = &mut b.kit;
    k.mat(ids::TRIM).block(Vec3::new(-7.2, y0 - 0.3, zh - 0.3), Vec3::new(7.2, y0, zh + 0.2));
    k.at(Mat4::from_translation(Vec3::new(0.0, y0 - 0.1, zh + 3.6)) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.mat(ids::PLASTER).frustum(Vec3::ZERO, 1.2, 1.2, 0.1, 16, true));
    canopy(k, 0.0, y0, 14.0, 3.0, 4.2);
    // Platforms and tracks behind.
    for (i, y) in [y1 + 4.0, y1 + 14.0].into_iter().enumerate() {
        if y + 3.0 > hy + 14.0 {
            continue;
        }
        k.mat(ids::CURB).block(Vec3::new(-hx, y - 2.5, 0.0), Vec3::new(hx, y + 2.5, 1.0));
        k.mat(ids::LANE).block(Vec3::new(-hx, y + 2.2, 1.0), Vec3::new(hx, y + 2.5, 1.02));
        k.mat(ids::ROOF).block(Vec3::new(-hx * 0.8, y - 2.0, 4.6), Vec3::new(hx * 0.8, y + 2.0, 4.8));
        for j in 0..((hx * 1.6 / 8.0) as usize).max(2) {
            let x = -hx * 0.8 + 1.0 + j as f32 * 8.0;
            k.mat(ids::IRON).block(Vec3::new(x - 0.12, y - 0.12, 1.0), Vec3::new(x + 0.12, y + 0.12, 4.6));
        }
        track(k, -hx, hx, y + 5.0, 0.0);
        let _ = i;
    }
}

fn metro(b: &mut Ctx, hx: f32, hy: f32) {
    // Stairs going down under a glass canopy, railings and a lit sign mast.
    let (w, run) = (hx * 1.2, hy * 1.4);
    let depth = (run / 0.28 * 0.18).min(3.2);
    // Paving with an opening over the stairwell.
    let (ox, oy0, oy1) = (w * 0.5, hy * 0.7 - 0.28 * (depth / 0.18).round(), hy * 0.7);
    let k = &mut b.kit;
    k.mat(ids::SIDEWALK);
    k.block(Vec3::new(-hx, -hy, -0.1), Vec3::new(hx, oy0, 0.0));
    k.block(Vec3::new(-hx, oy1, -0.1), Vec3::new(hx, hy, 0.0));
    for s in [-1.0f32, 1.0] {
        let (x0, x1) = if s < 0.0 { (-hx, -ox) } else { (ox, hx) };
        k.block(Vec3::new(x0, oy0, -0.1), Vec3::new(x1, oy1, 0.0));
    }
    k.mat(ids::CURB).block(Vec3::new(-ox, oy1 - 0.3, -depth - 0.2), Vec3::new(ox, oy1, -depth));
    k.at(Mat4::from_translation(Vec3::new(0.0, hy * 0.7, -depth)) * Mat4::from_rotation_z(std::f32::consts::PI), |k| {
        super::arch::stairs(k, super::StairKind::Straight, w, depth);
    });
    k.mat(ids::CURB);
    for s in [-1.0, 1.0] {
        k.block(Vec3::new(s * (w * 0.5 + 0.15) - 0.15, -hy * 0.7, -depth), Vec3::new(s * (w * 0.5 + 0.15) + 0.15, hy * 0.7, 0.9));
        k.mat(ids::IRON).block(Vec3::new(s * (w * 0.5 + 0.15) - 0.04, -hy * 0.7, 0.9), Vec3::new(s * (w * 0.5 + 0.15) + 0.04, hy * 0.7, 1.0));
        k.mat(ids::CURB);
    }
    k.block(Vec3::new(-w * 0.5 - 0.3, hy * 0.7, -depth), Vec3::new(w * 0.5 + 0.3, hy * 0.7 + 0.3, 0.9));
    k.mat(ids::GLASS).block(Vec3::new(-w * 0.5 - 0.4, -hy * 0.75, 2.8), Vec3::new(w * 0.5 + 0.4, hy * 0.75, 2.9));
    k.mat(ids::IRON);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let p = Vec3::new(x * (w * 0.5 + 0.3), y * hy * 0.72, 0.0);
        k.block(p - Vec3::new(0.06, 0.06, 0.0), p + Vec3::new(0.06, 0.06, 2.8));
    }
    let g = b.glow(true);
    let p = Vec3::new(-w * 0.5 - 1.0, -hy * 0.7, 0.0);
    let k = &mut b.kit;
    k.mat(ids::IRON).frustum(p, 0.08, 0.08, 3.2, 6, true);
    k.mat(g).block(p + Vec3::new(-0.35, -0.35, 3.2), p + Vec3::new(0.35, 0.35, 3.9));
}

fn tram_stop(b: &mut Ctx, hx: f32, hy: f32) {
    // An island platform with a canopy, a ticket machine, rails both sides
    // and poles carrying the overhead wire.
    let k = &mut b.kit;
    k.mat(ids::ASPHALT).block(Vec3::new(-hx - 2.0, -hy - 4.0, 0.0), Vec3::new(hx + 2.0, hy + 4.0, 0.05));
    k.mat(ids::CURB).block(Vec3::new(-hx, -hy * 0.45, 0.0), Vec3::new(hx, hy * 0.45, 0.3));
    for y in [-hy - 1.5, hy + 1.5] {
        track(k, -hx - 2.0, hx + 2.0, y, 0.0);
    }
    k.mat(ids::GLASS).block(Vec3::new(-hx * 0.5, -hy * 0.35, 2.7), Vec3::new(hx * 0.5, hy * 0.35, 2.8));
    k.mat(ids::IRON);
    for x in [-hx * 0.45, hx * 0.45] {
        k.block(Vec3::new(x - 0.07, -0.07, 0.3), Vec3::new(x + 0.07, 0.07, 2.7));
    }
    k.mat(ids::TRIM).block(Vec3::new(-hx * 0.3, -0.25, 0.3), Vec3::new(hx * 0.3, 0.25, 0.75));
    k.mat(ids::IRON).block(Vec3::new(hx * 0.6, -0.3, 0.3), Vec3::new(hx * 0.6 + 0.6, 0.3, 1.9));
    for x in [-hx + 0.5, hx - 0.5] {
        k.mat(ids::IRON).frustum(Vec3::new(x, 0.0, 0.3), 0.12, 0.09, 6.0, 8, true);
        k.beam(Vec3::new(x, -hy - 1.8, 5.8), Vec3::new(x, hy + 1.8, 5.8), 0.1, 0.1);
    }
    for y in [-hy - 1.5, hy + 1.5] {
        k.beam(Vec3::new(-hx - 2.0, y, 5.5), Vec3::new(hx + 2.0, y, 5.5), 0.03, 0.03);
    }
    let g = b.glow(false);
    b.kit.mat(g).block(Vec3::new(hx * 0.6 + 0.1, -0.32, 1.0), Vec3::new(hx * 0.6 + 0.5, -0.3, 1.6));
}

/// A simple airliner nosing in at `c` (nose towards -Y): fuselage, wings,
/// tail fin and engines.
fn plane(k: &mut Kit, c: Vec3, len: f32) {
    let r = len * 0.055;
    let z = c.z + r + 1.6;
    k.mat(ids::PLASTER).tube(Vec3::new(c.x, c.y - len * 0.4, z), Vec3::new(c.x, c.y + len * 0.4, z), r, 14);
    k.frustum(Vec3::ZERO, 0.0, 0.0, 0.0, 3, false);
    k.at(Mat4::from_translation(Vec3::new(c.x, c.y - len * 0.4, z)) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.frustum(Vec3::ZERO, r, 0.0, -r * 1.6, 14, false));
    k.at(Mat4::from_translation(Vec3::new(c.x, c.y + len * 0.4, z)) * Mat4::from_rotation_x(-FRAC_PI_2), |k| k.frustum(Vec3::ZERO, r, r * 0.2, -r * 2.5, 14, false));
    let span = len * 0.48;
    for s in [-1.0f32, 1.0] {
        k.mat(ids::PLASTER).poly(&[Vec3::new(c.x, c.y - len * 0.05, z - r * 0.4), Vec3::new(c.x + s * span, c.y + len * 0.12, z - r * 0.2), Vec3::new(c.x + s * span, c.y + len * 0.17, z - r * 0.2), Vec3::new(c.x, c.y + len * 0.13, z - r * 0.4)]);
        k.poly(&[Vec3::new(c.x, c.y + len * 0.13, z - r * 0.4), Vec3::new(c.x + s * span, c.y + len * 0.17, z - r * 0.2), Vec3::new(c.x + s * span, c.y + len * 0.12, z - r * 0.2), Vec3::new(c.x, c.y - len * 0.05, z - r * 0.4)]);
        k.mat(ids::IRON).tube(Vec3::new(c.x + s * span * 0.35, c.y - len * 0.04, z - r * 1.3), Vec3::new(c.x + s * span * 0.35, c.y + len * 0.06, z - r * 1.3), r * 0.4, 10);
        k.mat(ids::PLASTER).beam(Vec3::new(c.x, c.y + len * 0.4, z + r * 0.3), Vec3::new(c.x + s * len * 0.15, c.y + len * 0.47, z + r * 0.4), 0.15, len * 0.04);
    }
    k.mat(ids::ROOF).poly(&[Vec3::new(c.x, c.y + len * 0.3, z + r), Vec3::new(c.x, c.y + len * 0.47, z + r), Vec3::new(c.x, c.y + len * 0.5, z + r + len * 0.16), Vec3::new(c.x, c.y + len * 0.43, z + r + len * 0.16)]);
    k.mat(ids::ASPHALT);
    for (x, y) in [(0.0, -len * 0.32), (-r, len * 0.05), (r, len * 0.05)] {
        k.frustum(Vec3::new(c.x + x, c.y + y, c.z), 0.5, 0.5, z - r - c.z, 8, true);
    }
}

fn airport(b: &mut Ctx, hx: f32, hy: f32) {
    // A long glazed terminal under a sawtooth wave roof, a drop-off road
    // with a canopy, jet bridges out the back and planes at the gates.
    let (y0, y1, h) = (-hy + 12.0, -hy + 30.0, 12.0);
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 3.0, 0.0), Vec3::new(hx, y0, 0.15));
    let pane = b.pane(0.6);
    let k = &mut b.kit;
    k.mat(pane).block(Vec3::new(-hx + 2.0, y0, 0.15), Vec3::new(hx - 2.0, y1, h - 1.0));
    k.mat(ids::IRON);
    let n = ((hx * 2.0 / 6.0) as usize).max(3);
    for i in 0..=n {
        let x = -hx + 2.0 + (2.0 * hx - 4.0) * i as f32 / n as f32;
        k.block(Vec3::new(x - 0.1, y0 - 0.12, 0.15), Vec3::new(x + 0.1, y0, h - 1.0));
    }
    let waves = 5;
    let d = (y1 - y0 + 6.0) / waves as f32;
    for i in 0..waves {
        let (a, e) = (y0 - 3.0 + d * i as f32, y0 - 3.0 + d * (i + 1) as f32);
        k.mat(ids::SLATE).quad(Vec3::new(-hx, a, h - 1.0), Vec3::new(hx, a, h - 1.0), Vec3::new(hx, e, h + 1.5), Vec3::new(-hx, e, h + 1.5));
        k.quad(Vec3::new(-hx, e, h + 1.5), Vec3::new(hx, e, h + 1.5), Vec3::new(hx, a, h - 1.0), Vec3::new(-hx, a, h - 1.0));
        k.mat(ids::GLASS).quad(Vec3::new(hx, e, h - 1.0), Vec3::new(-hx, e, h - 1.0), Vec3::new(-hx, e, h + 1.5), Vec3::new(hx, e, h + 1.5));
    }
    canopy(k, 0.0, y0 - 3.0, hx * 1.6, 5.0, 5.5);
    k.mat(ids::LANE).block(Vec3::new(-hx, y0 - 8.5, 0.05), Vec3::new(hx, y0 - 8.3, 0.07));
    let gates = (((2.0 * hx) / 40.0) as usize).clamp(1, 5);
    for i in 0..gates {
        let x = -hx + 2.0 * hx * (i as f32 + 0.5) / gates as f32;
        k.mat(ids::PLASTER).block(Vec3::new(x - 1.4, y1, 4.0), Vec3::new(x + 1.4, y1 + 14.0, 7.0));
        k.mat(ids::IRON).block(Vec3::new(x - 0.5, y1 + 12.0, 0.05), Vec3::new(x + 0.5, y1 + 13.0, 4.0));
        k.mat(ids::LANE).quad(Vec3::new(x + 6.0, y1 + 2.0, 0.06), Vec3::new(x + 6.2, y1 + 2.0, 0.06), Vec3::new(x + 6.2, hy, 0.06), Vec3::new(x + 6.0, hy, 0.06));
        plane(k, Vec3::new(x + 6.1, y1 + 24.0, 0.05), 36.0);
    }
    sign(b, 0.0, y0 - 3.0, h - 3.0, hx * 0.5);
    planters(&mut b.kit, -hx + 2.0, hx - 2.0, y0 - 1.5, 6);
}

fn control_tower(b: &mut Ctx, hx: f32, floors: u32) {
    // A tapered concrete shaft, a flared glass cab and antennas on top,
    // with a low base building.
    let h = 18.0 + floors.clamp(1, 20) as f32 * 2.0;
    flat_box(&mut b.kit, -hx, -hx * 0.7, hx, hx * 0.7, 0.0, 4.0, ids::PLASTER, 0.4);
    storefront(b, 0.0, hx * 0.8, -hx * 0.7, 0.05, 2.6, true);
    let k = &mut b.kit;
    k.mat(ids::PLASTER).frustum(Vec3::Z * 4.0, 2.6, 2.0, h - 4.0, 16, false);
    k.mat(ids::TRIM).frustum(Vec3::Z * h, 2.0, 4.6, 1.2, 16, true);
    let pane = ids::GLASS;
    k.mat(pane).frustum(Vec3::Z * (h + 1.2), 4.4, 5.0, 3.0, 16, false);
    k.mat(ids::TRIM).frustum(Vec3::Z * (h + 4.2), 5.2, 4.0, 0.8, 16, true);
    k.mat(ids::IRON).frustum(Vec3::Z * (h + 5.0), 0.1, 0.05, 6.0, 6, true);
    k.block(Vec3::new(1.5, -0.2, h + 5.0), Vec3::new(2.0, 0.2, h + 7.0));
    ladder(k, Vec3::new(0.0, -2.62, 4.0), 3.0);
    let g = b.glow(false);
    b.kit.mat(g).block(Vec3::new(-0.15, -0.15, h + 11.0), Vec3::new(0.15, 0.15, h + 11.3));
}

fn harbour(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // A hipped-roof office on the quay edge with a lookout, a flag,
    // bollards along the edge and water in front.
    let floors = floors.clamp(1, 4);
    let qy = -hy + 3.0;
    b.kit.mat(ids::WATER).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, qy, 0.05));
    b.kit.mat(ids::CURB).block(Vec3::new(-hx, qy, 0.0), Vec3::new(hx, hy, 1.2));
    let k = &mut b.kit;
    for i in 0..((hx * 2.0 / 4.0) as usize) {
        let x = -hx + 2.0 + i as f32 * 4.0;
        k.mat(ids::IRON).frustum(Vec3::new(x, qy + 0.6, 1.2), 0.22, 0.28, 0.7, 8, true);
    }
    let (bx, by) = (hx * 0.6, (hy - qy) * 0.35);
    framed(b, Mat4::from_translation(Vec3::new(-hx * 0.2, qy + 3.0 + by, 0.0)), |b| {
        massing(b, bx, by, 1.2, floors, 3.2, ids::PLASTER, 2.2, 1.1, 1.5, 0.5);
        let top = 1.2 + 3.2 * floors as f32;
        roof(&mut b.kit, bx, by, top, Roof::Hip(30.0), 0.5, ids::PLASTER);
        b.kit.mat(ids::PLASTER).block(Vec3::new(bx - 2.0, -1.4, top), Vec3::new(bx - 0.2, 0.4, top + 2.6));
        let pane = b.pane(0.4);
        window(&mut b.kit, bx - 1.1, -1.42, top + 1.0, 1.4, 1.2, pane);
        b.kit.at(Mat4::from_translation(Vec3::new(bx - 1.1, -0.5, top + 2.6)), |k| k.mat(ids::ROOF).hipped(1.1, 1.1, 0.9, 0.2));
        door(&mut b.kit, -bx * 0.5, -by - 0.02, 1.2, 1.2, 2.2);
        sign(b, bx * 0.2, -by, 3.4, bx * 0.6);
    });
    flag(&mut b.kit, Vec3::new(hx - 1.5, qy + 1.5, 1.2), 9.0, ids::STUCCO);
    dock_door(&mut b.kit, hx * 0.65, hy - 0.5, 3.0);
}
