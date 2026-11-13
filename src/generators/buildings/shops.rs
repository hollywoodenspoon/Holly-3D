//! Shared pieces for the commercial and office prefabs: glazed
//! shopfronts, flat-roofed boxes with parapets, signs, pylon signs,
//! parking lots with painted bays and parked cars, and café tables.

use super::houses::{Roof, door, framed, roof};
use super::{Ctx, doodads, parts};
use crate::generators::kit::{Kit, ids};
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;

/// A glazed shopfront `w` wide on a wall facing -Y at `y`, from `z` up
/// `h`, with mullions every ~1.6 m, a transom bar and an optional door.
pub(super) fn storefront(b: &mut Ctx, x: f32, w: f32, y: f32, z: f32, h: f32, with_door: bool) {
    let pane = b.pane(0.7);
    window(&mut b.kit, x, y, z, w, h, pane);
    let k = &mut b.kit;
    k.mat(ids::IRON);
    let n = ((w / 1.6).ceil() as usize).max(1);
    for i in 0..=n {
        let px = x - w * 0.5 + w * i as f32 / n as f32;
        k.block(Vec3::new(px - 0.05, y - 0.08, z), Vec3::new(px + 0.05, y, z + h));
    }
    k.block(Vec3::new(x - w * 0.5, y - 0.08, z + h - 0.06), Vec3::new(x + w * 0.5, y, z + h + 0.06));
    k.block(Vec3::new(x - w * 0.5, y - 0.08, z - 0.06), Vec3::new(x + w * 0.5, y, z + 0.06));
    if with_door {
        door(k, x, y - 0.08, z, 1.8, 2.3);
    }
}

/// A flat-roofed box from (`x0`, `y0`) to (`x1`, `y1`), `h` tall, with a
/// parapet `par` high.
#[allow(clippy::too_many_arguments)]
pub(super) fn flat_box(k: &mut Kit, x0: f32, y0: f32, x1: f32, y1: f32, z0: f32, h: f32, wall: u16, par: f32) {
    k.mat(wall).block(Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z0 + h));
    let (hx, hy) = ((x1 - x0) * 0.5, (y1 - y0) * 0.5);
    k.at(Mat4::from_translation(Vec3::new(x0 + hx, y0 + hy, 0.0)), |k| roof(k, hx, hy, z0 + h, Roof::Flat(par), 0.0, ids::TRIM));
}

/// A sign board on a facade facing -Y at `y` (when Signs is on).
pub(super) fn sign(b: &mut Ctx, x: f32, y: f32, z: f32, w: f32) {
    if b.p.parts.signs {
        let g = b.glow(true);
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, y - 0.02, 0.0)), |k| parts::sign(k, x, z, w, g));
    }
}

/// A free-standing pylon sign at `p`, `h` tall (when Signs is on).
pub(super) fn pylon(b: &mut Ctx, p: Vec3, h: f32, w: f32) {
    if b.p.parts.signs {
        let g = b.glow(true);
        b.kit.mat(ids::IRON).block(p - Vec3::new(0.2, 0.2, 0.0), p + Vec3::new(0.2, 0.2, h));
        b.kit.mat(ids::TRIM).block(p + Vec3::new(-w * 0.5, -0.3, h), p + Vec3::new(w * 0.5, 0.3, h + w * 0.55));
        b.kit.mat(g).block(p + Vec3::new(-w * 0.44, -0.32, h + 0.1), p + Vec3::new(w * 0.44, 0.32, h + w * 0.55 - 0.1));
    }
}

/// A parking lot over `min`..`max` with two rows of bays facing a middle
/// aisle (bays `bay` wide, 5 m deep), some filled with cars (`fill` share).
pub(super) fn parking(b: &mut Ctx, min: Vec2, max: Vec2, bay: f32, fill: f64) {
    b.kit.mat(ids::ASPHALT).block(min.extend(0.0), max.extend(0.05));
    let n = (((max.x - min.x) / bay).floor() as usize).max(1);
    let deep = ((max.y - min.y) * 0.32).min(5.0);
    for (row, (y0, y1)) in [(min.y + 0.3, min.y + 0.3 + deep), (max.y - 0.3 - deep, max.y - 0.3)].into_iter().enumerate() {
        b.kit.mat(ids::LANE);
        for i in 0..=n {
            let x = min.x + (max.x - min.x) * i as f32 / n as f32;
            b.kit.quad(Vec3::new(x - 0.06, y0, 0.055), Vec3::new(x + 0.06, y0, 0.055), Vec3::new(x + 0.06, y1, 0.055), Vec3::new(x - 0.06, y1, 0.055));
        }
        for i in 0..n {
            if !b.r.gen_bool(fill.clamp(0.0, 1.0)) {
                continue;
            }
            let x = min.x + (max.x - min.x) * (i as f32 + 0.5) / n as f32;
            let yaw = if row == 0 { 0.0 } else { std::f32::consts::PI };
            let m = Mat4::from_translation(Vec3::new(x, (y0 + y1) * 0.5, 0.05)) * Mat4::from_rotation_z(yaw);
            framed(b, m, |b| doodads::car(b, 0.92, (deep * 0.44).min(2.2)));
        }
    }
}

/// Café tables with parasols around `c`, spread over `hx` x `hy`.
pub(super) fn tables(b: &mut Ctx, c: Vec2, hx: f32, n: usize) {
    for i in 0..n {
        let p = Vec3::new(c.x - hx + 2.0 * hx * (i as f32 + 0.5) / n as f32, c.y + b.r.gen_range(-0.3..0.3), 0.0);
        let canopy = [ids::STUCCO, ids::ROOF, ids::PLASTER][b.r.gen_range(0..3)];
        let k = &mut b.kit;
        k.mat(ids::IRON).frustum(p, 0.04, 0.04, 2.3, 5, false);
        k.mat(ids::TRIM).frustum(p + Vec3::Z * 0.72, 0.4, 0.4, 0.05, 10, true);
        for a in 0..2 {
            let d = Vec3::new(if a == 0 { -0.65 } else { 0.65 }, 0.0, 0.0);
            k.mat(ids::IRON).block(p + d - Vec3::new(0.2, 0.2, 0.0), p + d + Vec3::new(0.2, 0.2, 0.45));
        }
        k.mat(canopy).frustum(p + Vec3::Z * 2.0, 1.2, 0.05, 0.45, 8, true);
    }
}

/// A row of planters (box + bush) along X from `x0` to `x1` at `y`.
pub(super) fn planters(k: &mut Kit, x0: f32, x1: f32, y: f32, n: usize) {
    for i in 0..n {
        let x = x0 + (x1 - x0) * (i as f32 + 0.5) / n as f32;
        k.mat(ids::CURB).block(Vec3::new(x - 0.4, y - 0.4, 0.0), Vec3::new(x + 0.4, y + 0.4, 0.5));
        k.mat(ids::FOLIAGE).frustum(Vec3::new(x, y, 0.5), 0.38, 0.2, 0.6, 7, true);
    }
}

/// A flat canopy over an entrance: `w` wide, `d` deep, at height `z`,
/// on two posts, in front of a wall facing -Y at `y`.
pub(super) fn canopy(k: &mut Kit, x: f32, y: f32, w: f32, d: f32, z: f32) {
    k.mat(ids::TRIM).block(Vec3::new(x - w * 0.5, y - d, z), Vec3::new(x + w * 0.5, y, z + 0.3));
    k.mat(ids::IRON);
    for s in [-1.0, 1.0] {
        let px = x + s * (w * 0.5 - 0.2);
        k.block(Vec3::new(px - 0.1, y - d + 0.1, 0.0), Vec3::new(px + 0.1, y - d + 0.3, z));
    }
}
