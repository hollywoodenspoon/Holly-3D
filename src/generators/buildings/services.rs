//! Civic and service prefabs: fire station, police station, clinic,
//! university, library, town hall and post office (temple and cemetery
//! are in `services_b.rs`).
//! Ground at z = 0, entrance facing -Y.

use super::houses::{Roof, door, framed, roof};
use super::shops::{canopy, flat_box, parking, planters, sign, storefront};
use super::{BuildingKind as K, Ctx, doodads, massing};
use crate::generators::kit::{Kit, ids};
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::FireStation => fire_station(b, hx, hy),
        K::PoliceStation => police(b, hx, hy, floors),
        K::Clinic => clinic(b, hx, hy),
        K::University => university(b, hx, hy, floors),
        K::Library => library(b, hx, hy),
        K::TownHall => town_hall(b, hx, hy, floors),
        K::PostOffice => post_office(b, hx, hy),
        _ => {}
    }
}

/// A flag pole at `p` with a flag in `paint`.
pub(super) fn flag(k: &mut Kit, p: Vec3, h: f32, paint: u16) {
    k.mat(ids::IRON).frustum(p, 0.07, 0.04, h, 6, true);
    k.mat(paint).block(p + Vec3::new(0.05, -0.02, h - 1.3), p + Vec3::new(1.8, 0.02, h - 0.1));
}

/// A columned portico `w` wide in front of a wall facing -Y at `y`.
fn portico(k: &mut Kit, x: f32, y: f32, w: f32, z: f32, h: f32, n: usize) {
    k.mat(ids::PLASTER);
    for i in 0..n {
        let cx = x - w * 0.5 + w * i as f32 / (n - 1).max(1) as f32;
        k.frustum(Vec3::new(cx, y - 1.6, z), 0.4, 0.34, h, 12, true);
    }
    k.mat(ids::TRIM).block(Vec3::new(x - w * 0.5 - 0.6, y - 2.2, z + h), Vec3::new(x + w * 0.5 + 0.6, y, z + h + 0.8));
    let m = Mat4::from_translation(Vec3::new(x, y - 1.1, z + h + 0.8)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    k.at(m, |k| k.mat(ids::ROOF).gable(1.1, w * 0.5 + 0.6, 1.6, 0.0, ids::PLASTER));
}

/// A tree: trunk and a stack of foliage cones.
pub(super) fn tree(k: &mut Kit, p: Vec3, h: f32) {
    k.mat(ids::TRIM).frustum(p, 0.18, 0.12, h * 0.4, 6, true);
    k.mat(ids::FOLIAGE).frustum(p + Vec3::Z * h * 0.3, h * 0.28, h * 0.05, h * 0.7, 8, true);
}

fn fire_station(b: &mut Ctx, hx: f32, hy: f32) {
    // Brick, tall appliance bays with glazed roller doors, an apron, a
    // hose-drying tower and the crew quarters above.
    let (y0, fh) = (-hy + 8.0, 5.0);
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.05));
    let w = hx * 1.4;
    b.kit.mat(ids::BRICK).block(Vec3::new(-hx, y0, 0.0), Vec3::new(hx * 0.4, hy, fh));
    let bays = ((w / 4.6) as usize).clamp(2, 5);
    for i in 0..bays {
        let x = -hx + w * (i as f32 + 0.5) / bays as f32;
        let pane = b.pane(0.5);
        window(&mut b.kit, x, y0 - 0.02, 0.05, 3.8, 4.2, pane);
        b.kit.mat(ids::IRON);
        for z in [1.1, 2.15, 3.2] {
            b.kit.block(Vec3::new(x - 1.9, y0 - 0.08, z), Vec3::new(x + 1.9, y0, z + 0.08));
        }
        b.kit.mat(ids::LANE).quad(Vec3::new(x - 0.08, -hy, 0.06), Vec3::new(x + 0.08, -hy, 0.06), Vec3::new(x + 0.08, y0, 0.06), Vec3::new(x - 0.08, y0, 0.06));
    }
    framed(b, Mat4::from_translation(Vec3::new(-hx * 0.3, (y0 + hy) * 0.5, 0.0)), |b| {
        let dy = (hy - y0) * 0.5;
        massing(b, hx * 0.7, dy, fh, 1, 3.2, ids::BRICK, 2.2, 1.1, 1.4, 0.5);
        roof(&mut b.kit, hx * 0.7, dy, fh + 3.2, Roof::Flat(0.7), 0.0, ids::TRIM);
    });
    // Office wing on the right with the door.
    flat_box(&mut b.kit, hx * 0.4, y0, hx, hy, 0.0, 7.0, ids::BRICK, 0.6);
    storefront(b, hx * 0.7, hx * 0.4, y0, 0.05, 2.4, true);
    // Hose tower.
    let t = Vec3::new(hx * 0.8, hy - 2.5, 0.0);
    b.kit.mat(ids::BRICK).block(t - Vec3::new(2.0, 2.0, 0.0), t + Vec3::new(2.0, 2.0, 16.0));
    b.kit.at(Mat4::from_translation(t + Vec3::Z * 16.0), |k| k.mat(ids::ROOF).hipped(2.0, 2.0, 2.0, 0.3));
    sign(b, -hx * 0.3, y0, fh - 0.75, w * 0.6);
    if b.p.parts.signs {
        let g = b.glow(true);
        b.kit.mat(g).block(Vec3::new(hx * 0.7 - 0.3, y0 - 0.25, 3.0), Vec3::new(hx * 0.7 + 0.3, y0 - 0.05, 3.4));
    }
}

fn police(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Flat-roofed block behind a car park, glazed entrance, blue lamp, flag.
    let floors = floors.clamp(1, 5);
    let y0 = -hy + hy * 0.6;
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 2.5), 2.6, 0.6);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 2.5, 0.0), Vec3::new(hx, y0, 0.12));
    let dy = (hy - y0) * 0.5;
    let wall = if b.r.gen_bool(0.5) { ids::BRICK } else { ids::PLASTER };
    framed(b, Mat4::from_translation(Vec3::new(0.0, y0 + dy, 0.0)), |b| {
        massing(b, hx, dy, 0.0, floors, 3.5, wall, 2.4, 1.3, 1.6, 0.6);
        roof(&mut b.kit, hx, dy, 3.5 * floors as f32, Roof::Flat(0.8), 0.0, ids::TRIM);
        flat_box(&mut b.kit, -3.0, -dy - 2.5, 3.0, -dy, 0.0, 3.6, ids::PLASTER, 0.2);
        storefront(b, 0.0, 4.6, -dy - 2.5, 0.12, 2.6, true);
        sign(b, 0.0, -dy - 2.5, 3.65, 5.0);
        if b.p.parts.signs {
            let g = b.glow(true);
            b.kit.mat(g).block(Vec3::new(2.4, -dy - 2.75, 2.6), Vec3::new(2.8, -dy - 2.5, 3.2));
        }
    });
    flag(&mut b.kit, Vec3::new(-hx + 1.0, y0 - 1.2, 0.12), 9.0, ids::STUCCO);
}

fn clinic(b: &mut Ctx, hx: f32, hy: f32) {
    // A single-storey clinic with ribbon windows, a drop-off canopy and a
    // glowing cross, behind a few parking bays.
    let (y0, h) = (-hy + 6.0, 4.2);
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 1.0), 2.6, 0.4);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 1.0, 0.0), Vec3::new(hx, y0, 0.12));
    flat_box(&mut b.kit, -hx, y0, hx, hy, 0.0, h, ids::PLASTER, 0.5);
    let pane = b.pane(0.6);
    window(&mut b.kit, -hx * 0.45, y0 - 0.02, 1.0, hx * 0.8, 1.6, pane);
    window(&mut b.kit, hx * 0.6, y0 - 0.02, 1.0, hx * 0.6, 1.6, pane);
    storefront(b, hx * 0.1, 3.0, y0, 0.12, 2.6, true);
    canopy(&mut b.kit, hx * 0.1, y0, 6.0, 4.0, 3.2);
    if b.p.parts.signs {
        let g = b.glow(true);
        let c = Vec3::new(-hx + 1.5, y0 - 0.2, h - 1.0);
        b.kit.mat(g).block(c - Vec3::new(0.6, 0.1, 0.2), c + Vec3::new(0.6, 0.1, 0.2));
        b.kit.block(c - Vec3::new(0.2, 0.1, 0.6), c + Vec3::new(0.2, 0.1, 0.6));
    }
    planters(&mut b.kit, -hx + 1.0, hx * 0.1 - 3.5, y0 - 0.5, 3);
}

fn university(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // A long brick block with a columned entrance and a clock tower, in
    // front of a lawn quad crossed by paths and trees.
    let floors = floors.clamp(2, 8);
    let fh = 3.8;
    let (y0, dy) = (-hy + hy * 0.6, hy * 0.2);
    b.kit.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.06));
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-1.5, -hy, 0.0), Vec3::new(1.5, y0, 0.1));
    b.kit.block(Vec3::new(-hx, -hy + (y0 + hy) * 0.45, 0.0), Vec3::new(hx, -hy + (y0 + hy) * 0.45 + 2.0, 0.1));
    for i in 0..6 {
        let x = -hx + 3.0 + (2.0 * hx - 6.0) * i as f32 / 5.0;
        if x.abs() > 3.0 {
            tree(&mut b.kit, Vec3::new(x, -hy + 2.5, 0.06), 6.0 + b.r.gen_range(0.0..2.0));
        }
    }
    framed(b, Mat4::from_translation(Vec3::new(0.0, y0 + dy, 0.0)), |b| {
        massing(b, hx, dy, 0.0, floors, fh, ids::BRICK, 2.6, 1.3, 2.0, 0.5);
        let top = fh * floors as f32;
        roof(&mut b.kit, hx, dy, top, Roof::Hip(28.0), 0.4, ids::BRICK);
        portico(&mut b.kit, 0.0, -dy, 9.0, 0.0, fh * 2.0, 6);
        door(&mut b.kit, 0.0, -dy - 0.02, 0.0, 2.2, 3.4);
        // Clock tower over the entrance.
        let k = &mut b.kit;
        k.mat(ids::BRICK).block(Vec3::new(-2.5, -2.5, top), Vec3::new(2.5, 2.5, top + 9.0));
        k.at(Mat4::from_translation(Vec3::new(0.0, -2.55, top + 6.0)) * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2), |k| {
            k.mat(ids::PLASTER).frustum(Vec3::ZERO, 1.4, 1.4, 0.08, 16, true)
        });
        k.at(Mat4::from_translation(Vec3::Z * (top + 9.0)), |k| k.mat(ids::SLATE).hipped(2.6, 2.6, 4.5, 0.3));
    });
    sign(b, 0.0, y0 - 0.1, 0.4, 6.0);
}

fn library(b: &mut Ctx, hx: f32, hy: f32) {
    // A glazed ground floor under a cantilevered upper box with vertical
    // fins, a reading-room clerestory and a plaza with benches.
    let (y0, g, up) = (-hy + 5.0, 4.5, 4.5);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.1));
    let (gx, gy) = (hx - 2.0, (hy - y0) * 0.5 - 1.0);
    let c = Vec3::new(0.0, y0 + gy + 1.0, 0.0);
    let pane = b.pane(0.7);
    b.kit.mat(pane).block(c + Vec3::new(-gx, -gy, 0.1), c + Vec3::new(gx, gy, g));
    storefront(b, 0.0, 4.0, c.y - gy - 0.01, 0.1, 2.6, true);
    let k = &mut b.kit;
    k.mat(ids::PLASTER).block(c + Vec3::new(-hx, -gy - 2.0, g), c + Vec3::new(hx, gy + 0.5, g + up));
    let n = ((hx * 2.0 / 1.2) as usize).max(4);
    for i in 0..=n {
        let x = -hx + 2.0 * hx * i as f32 / n as f32;
        k.mat(ids::TRIM).block(c + Vec3::new(x - 0.08, -gy - 2.6, g), c + Vec3::new(x + 0.08, -gy - 2.0, g + up));
    }
    k.mat(ids::GLASS).block(c + Vec3::new(-hx + 0.3, -gy - 2.05, g + 0.8), c + Vec3::new(hx - 0.3, -gy - 1.95, g + up - 0.6));
    k.mat(ids::TRIM).block(c + Vec3::new(-hx - 0.1, -gy - 2.1, g + up), c + Vec3::new(hx + 0.1, gy + 0.6, g + up + 0.4));
    k.mat(ids::GLASS).block(c + Vec3::new(-hx * 0.5, -gy * 0.5, g + up + 0.4), c + Vec3::new(hx * 0.5, gy * 0.5, g + up + 1.8));
    k.mat(ids::ROOF).block(c + Vec3::new(-hx * 0.5 - 0.2, -gy * 0.5 - 0.2, g + up + 1.8), c + Vec3::new(hx * 0.5 + 0.2, gy * 0.5 + 0.2, g + up + 2.0));
    for i in 0..3 {
        let x = -hx * 0.6 + i as f32 * hx * 0.6;
        k.mat(ids::TRIM).block(Vec3::new(x - 1.0, -hy + 1.6, 0.1), Vec3::new(x + 1.0, -hy + 2.1, 0.55));
    }
    planters(k, -hx + 1.0, hx - 1.0, -hy + 0.6, 4);
    sign(b, 0.0, c.y - gy - 2.6, g + 0.4, hx);
}

fn town_hall(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Symmetrical stone hall: a raised centre block with a portico and a
    // domed clock tower, lower wings, steps, flags and a plaza.
    let floors = floors.clamp(1, 4);
    let fh = 4.2;
    let y0 = -hy + 7.0;
    b.kit.mat(ids::COBBLE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.08));
    let dy = (hy - y0) * 0.5;
    let wall = if b.r.gen_bool(0.5) { ids::PLASTER } else { ids::STUCCO };
    framed(b, Mat4::from_translation(Vec3::new(0.0, y0 + dy, 0.0)), |b| {
        b.kit.mat(ids::CURB).block(Vec3::new(-hx, -dy, 0.0), Vec3::new(hx, dy, 1.0));
        for s in [-1.0f32, 1.0] {
            framed(b, Mat4::from_translation(Vec3::new(s * hx * 0.62, 0.0, 0.0)), |b| {
                massing(b, hx * 0.38, dy * 0.9, 1.0, floors, fh, wall, 2.4, 1.2, 2.2, 0.4);
                roof(&mut b.kit, hx * 0.38, dy * 0.9, 1.0 + fh * floors as f32, Roof::Hip(25.0), 0.3, wall);
            });
        }
        let top = 1.0 + fh * (floors + 1) as f32;
        massing(b, hx * 0.25, dy, 1.0, floors + 1, fh, wall, 2.6, 1.4, 2.6, 0.5);
        roof(&mut b.kit, hx * 0.25, dy, top, Roof::Flat(1.0), 0.0, ids::TRIM);
        portico(&mut b.kit, 0.0, -dy, hx * 0.42, 1.0, fh * 2.0, 6);
        door(&mut b.kit, 0.0, -dy - 0.02, 1.0, 2.4, 3.6);
        let k = &mut b.kit;
        k.mat(wall).block(Vec3::new(-3.0, -3.0, top), Vec3::new(3.0, 3.0, top + 7.0));
        k.at(Mat4::from_translation(Vec3::new(0.0, -3.02, top + 4.5)) * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2), |k| {
            k.mat(ids::PLASTER).frustum(Vec3::ZERO, 1.6, 1.6, 0.08, 16, true)
        });
        k.mat(ids::TRIM).frustum(Vec3::Z * (top + 7.0), 3.2, 3.2, 0.5, 16, true);
        for i in 0..5 {
            let t = i as f32 / 5.0;
            let (r0, r1) = (3.0 * (1.0 - t * t).sqrt(), 3.0 * (1.0 - (t + 0.2).powi(2)).max(0.0).sqrt());
            k.mat(ids::SLATE).frustum(Vec3::Z * (top + 7.5 + 3.0 * t), r0, r1.max(0.3), 0.6, 16, i == 4);
        }
        k.mat(ids::IRON).frustum(Vec3::Z * (top + 10.5), 0.1, 0.0, 2.0, 6, false);
    });
    let kind = super::StairKind::Straight;
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, y0 - 2.24, 0.0)), |k| {
        super::arch::stairs(k, kind, hx * 0.5, 1.0);
    });
    for s in [-1.0, 1.0] {
        flag(&mut b.kit, Vec3::new(s * hx * 0.35, -hy + 1.5, 0.08), 10.0, [ids::STUCCO, ids::ROOF][(s > 0.0) as usize]);
    }
}

fn post_office(b: &mut Ctx, hx: f32, hy: f32) {
    // Brick counter hall with a big shopfront, post boxes outside and a
    // delivery van at the side loading door.
    let (y0, h) = (-hy + 3.5, 5.0);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.12));
    flat_box(&mut b.kit, -hx, y0, hx * 0.6, hy, 0.0, h, ids::BRICK, 0.8);
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.05, y0 - 0.1, h - 1.4), Vec3::new(hx * 0.6 + 0.05, y0, h - 1.2));
    storefront(b, -hx * 0.2, hx * 1.2, y0, 0.12, 3.0, true);
    sign(b, -hx * 0.2, y0 - 0.1, h - 1.1, hx);
    for i in 0..3 {
        let x = -hx + 1.0 + i as f32 * 0.9;
        let k = &mut b.kit;
        k.mat(ids::ROOF).block(Vec3::new(x - 0.3, -hy + 0.8, 0.12), Vec3::new(x + 0.3, -hy + 1.3, 1.3));
        k.mat(ids::IRON).block(Vec3::new(x - 0.2, -hy + 0.78, 1.0), Vec3::new(x + 0.2, -hy + 0.8, 1.06));
    }
    // Loading yard on the right.
    b.kit.mat(ids::ASPHALT).block(Vec3::new(hx * 0.6, y0, 0.0), Vec3::new(hx, hy, 0.05));
    b.kit.mat(ids::RUST).quad(
        Vec3::new(hx * 0.6 + 0.02, y0 + 2.0, 0.05),
        Vec3::new(hx * 0.6 + 0.02, y0 + 2.0, 3.2),
        Vec3::new(hx * 0.6 + 0.02, y0 + 5.5, 3.2),
        Vec3::new(hx * 0.6 + 0.02, y0 + 5.5, 0.05),
    );
    let m = Mat4::from_translation(Vec3::new(hx * 0.8, hy - 4.0, 0.05));
    framed(b, m, |b| doodads::car(b, 1.0, 2.6));
}
