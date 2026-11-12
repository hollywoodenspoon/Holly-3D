//! Medieval working buildings: windmill, watermill (with wheel), blacksmith
//! forge, stable, granary and well house.

use super::parts;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{ids, sides};
use crate::generators::settlement::timber_frame;
use bevy::math::{Mat4, Quat, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::Windmill => windmill(b, hx, floors),
        K::Watermill => watermill(b, hx, hy, floors),
        K::Forge => forge(b, hx, hy),
        K::Stable => stable(b, hx, hy),
        K::Granary => granary(b, hx, hy, floors),
        K::WellHouse => well_house(b, hx, hy),
        _ => {}
    }
}

fn windmill(b: &mut Ctx, hx: f32, floors: u32) {
    // Tapered stone tower, conical cap, four lattice sails on a hub.
    let h = 3.2 * floors as f32 + 2.0;
    b.kit.mat(ids::PLASTER).frustum(Vec3::ZERO, hx, hx * 0.7, h, 12, true);
    b.kit.mat(ids::ROOF).frustum(Vec3::Z * h, hx * 0.8, 0.0, hx * 0.9, 12, false);
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hx * 0.72, 0.0)), |k| k.mat(ids::TRIM).arched_panel(1.0, 2.0, false));
    let pane = b.pane(0.5);
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hx * 0.68, h * 0.55)), |k| k.mat(pane).arched_panel(0.6, 0.9, false));
    let hub = Vec3::new(0.0, -hx * 0.8, h - 0.4);
    b.kit.mat(ids::TRIM).tube(hub + Vec3::Y * 1.0, hub - Vec3::Y * 0.4, 0.25, 8);
    let len = h * 0.8;
    let turn: f32 = b.r.gen_range(0.0..FRAC_PI_2);
    for i in 0..4 {
        let a = turn + i as f32 * FRAC_PI_2;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        let side = Vec3::new(-a.sin(), 0.0, a.cos());
        let tip = hub - Vec3::Y * 0.4 + dir * len;
        b.kit.mat(ids::TRIM).beam(hub - Vec3::Y * 0.4, tip, 0.15, 0.15);
        // Lattice frame and cloth on one side of the stock.
        let (a0, a1) = (hub - Vec3::Y * 0.45 + dir * len * 0.2, tip - Vec3::Y * 0.05);
        b.kit.beam(a1, a1 + side * len * 0.22, 0.06, 0.06);
        b.kit.mat(ids::PLASTER).quad(a0, a0 + side * len * 0.22, a1 + side * len * 0.22, a1);
    }
}

fn watermill(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let h = 3.0 * floors as f32;
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 3.0));
    if floors > 1 {
        b.kit.mat(ids::PLASTER).block(Vec3::new(-hx, -hy, 3.0), Vec3::new(hx, hy, h));
        sides(&mut b.kit, hx, hy, |k, _, w| timber_frame(k, w, 3.0, h - 3.0));
    }
    b.kit.at(Mat4::from_translation(Vec3::Z * h), |k| k.mat(ids::ROOF).gable(hx, hy, hy * 1.0, 0.5, ids::PLASTER));
    // Mill race (water channel) along the +X side and an undershot wheel.
    let r = (h * 0.55).clamp(2.0, 4.5);
    let c = Vec3::new(hx + 0.9, 0.0, r * 0.6);
    b.kit.mat(ids::WATER).block(Vec3::new(hx + 0.2, -hy - 4.0, -0.5), Vec3::new(hx + 1.6, hy + 4.0, r * 0.6 - r + 0.3));
    b.kit.at(Mat4::from_translation(c) * Mat4::from_rotation_z(FRAC_PI_2) * Mat4::from_rotation_x(FRAC_PI_2), |k| {
        k.mat(ids::TRIM);
        let n = 12;
        for s in [-0.5, 0.5] {
            for i in 0..n {
                let (a0, a1) = (i as f32 / n as f32 * TAU, (i + 1) as f32 / n as f32 * TAU);
                let p = |a: f32| Vec3::new(a.cos() * r, a.sin() * r, s);
                k.beam(p(a0), p(a1), 0.15, 0.15);
            }
        }
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            let d = Vec3::new(a.cos(), a.sin(), 0.0);
            k.beam(d * 0.3, d * r, 0.1, 0.1);
            k.cuboid(d * (r - 0.35), Vec3::new(0.35, 0.05, 0.55), Quat::from_rotation_z(a));
        }
        k.tube(Vec3::Z * -0.8, Vec3::Z * 0.8, 0.2, 8);
    });
    b.kit.mat(ids::TRIM).beam(c, Vec3::new(hx, 0.0, r * 0.6), 0.25, 0.25);
    let p = b.p.parts;
    if p.chimneys {
        parts::chimney(&mut b.kit, Vec3::new(-hx * 0.5, hy * 0.3, h), hy + 1.0);
    }
    if p.porches {
        let st = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::porch(k, -hx * 0.3, 1.8, 1.2, 0.3, st));
    }
}

fn forge(b: &mut Ctx, hx: f32, hy: f32) {
    // Open-fronted workshop: back walls, posts at the front, lean-to roof,
    // a big chimney over the hearth, an anvil on a stump.
    let h = 3.2;
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, hy - 0.5, 0.0), Vec3::new(hx, hy, h + 1.0));
    b.kit.block(Vec3::new(-hx, -hy, 0.0), Vec3::new(-hx + 0.5, hy, h + 0.5));
    b.kit.block(Vec3::new(hx - 0.5, -hy, 0.0), Vec3::new(hx, hy, h + 0.5));
    b.kit.mat(ids::TRIM);
    for x in [-hx * 0.3, hx * 0.3] {
        b.kit.block(Vec3::new(x - 0.12, -hy, 0.0), Vec3::new(x + 0.12, -hy + 0.24, h));
    }
    b.kit.mat(ids::ROOF).quad(Vec3::new(-hx - 0.3, -hy - 0.5, h - 0.2), Vec3::new(hx + 0.3, -hy - 0.5, h - 0.2), Vec3::new(hx + 0.3, hy + 0.3, h + 1.2), Vec3::new(-hx - 0.3, hy + 0.3, h + 1.2));
    let hearth = Vec3::new(-hx * 0.4, hy * 0.5, 0.0);
    b.kit.mat(ids::BRICK).block(hearth - Vec3::new(1.0, 0.8, 0.0), hearth + Vec3::new(1.0, 0.5, 1.0));
    b.kit.block(hearth + Vec3::new(-0.6, -0.2, 1.0), hearth + Vec3::new(0.6, 0.5, h + 3.0));
    let fire = b.glow(true);
    b.kit.mat(fire).block(hearth + Vec3::new(-0.5, -0.5, 1.0), hearth + Vec3::new(0.5, 0.1, 1.15));
    let a = Vec3::new(hx * 0.3, -hy * 0.1, 0.0);
    b.kit.mat(ids::TRIM).frustum(a, 0.35, 0.3, 0.6, 8, true);
    b.kit.mat(ids::IRON).block(a + Vec3::new(-0.35, -0.12, 0.6), a + Vec3::new(0.35, 0.12, 0.85));
    if b.p.parts.signs {
        b.kit.mat(ids::IRON).beam(Vec3::new(hx, -hy, 2.6), Vec3::new(hx + 1.0, -hy, 2.6), 0.05, 0.05);
        b.kit.frustum(Vec3::new(hx + 0.8, -hy, 1.9), 0.3, 0.3, 0.06, 10, true);
    }
}

fn stable(b: &mut Ctx, hx: f32, hy: f32) {
    let h = 3.0;
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
    // Row of split stable doors and a hay loft gable.
    let n = ((2.0 * hx / 3.0) as usize).max(1);
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| {
        for i in 0..n {
            let x = -hx + 2.0 * hx * (i as f32 + 0.5) / n as f32;
            crate::generators::settlement::window(k, x, 0.0, 0.0, 1.3, 1.1, ids::TRIM);
            crate::generators::settlement::window(k, x, -0.02, 1.15, 1.3, 1.0, ids::FACADE);
        }
    });
    b.kit.at(Mat4::from_translation(Vec3::Z * h), |k| k.mat(ids::ROOF).gable(hx, hy, hy * 0.8, 0.5, ids::TRIM));
    b.kit.mat(ids::FOLIAGE).block(Vec3::new(hx + 0.3, -hy, 0.0), Vec3::new(hx + 1.6, -hy + 1.4, 1.2));
}

fn granary(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Raised on staddle stones against rats.
    b.kit.mat(ids::FACADE);
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (0.0, -1.0), (0.0, 1.0)] {
        let c = Vec3::new(sx * (hx - 0.4), sy * (hy - 0.4), 0.0);
        b.kit.frustum(c, 0.25, 0.15, 0.7, 6, false);
        b.kit.frustum(c + Vec3::Z * 0.7, 0.45, 0.4, 0.15, 8, true);
    }
    let h = 2.6 * floors as f32;
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx, -hy, 0.85), Vec3::new(hx, hy, 0.85 + h));
    sides(&mut b.kit, hx, hy, |k, _, w| timber_frame(k, w, 0.85, h));
    b.kit.at(Mat4::from_translation(Vec3::Z * (0.85 + h)), |k| k.mat(ids::ROOF).hipped(hx, hy, hx.min(hy) * 1.1, 0.4));
    if b.p.parts.porches {
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy, 0.0)) * Mat4::from_rotation_z(std::f32::consts::PI), |k| {
            super::arch::stairs(k, super::StairKind::Straight, 0.9, 0.85);
        });
    }
}

fn well_house(b: &mut Ctx, hx: f32, hy: f32) {
    let c = Vec3::ZERO;
    b.kit.mat(ids::FACADE).frustum(c, hx * 0.45, hx * 0.45, 0.9, 12, true);
    b.kit.mat(ids::WATER).frustum(c + Vec3::Z * 0.7, hx * 0.36, hx * 0.36, 0.01, 12, false);
    b.kit.mat(ids::TRIM);
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        b.kit.block(Vec3::new(sx * (hx - 0.2) - 0.1, sy * (hy - 0.2) - 0.1, 0.0), Vec3::new(sx * (hx - 0.2) + 0.1, sy * (hy - 0.2) + 0.1, 2.4));
    }
    b.kit.beam(Vec3::new(-hx * 0.6, 0.0, 1.8), Vec3::new(hx * 0.6, 0.0, 1.8), 0.15, 0.15);
    b.kit.mat(ids::IRON).frustum(Vec3::new(0.0, 0.0, 1.0), 0.18, 0.2, 0.3, 8, true);
    b.kit.at(Mat4::from_translation(Vec3::Z * 2.4), |k| k.mat(ids::ROOF).hipped(hx, hy, hx * 0.8, 0.3));
    if b.p.parts.signs {
        let g = b.glow(false);
        b.kit.mat(g).frustum(Vec3::new(hx - 0.2, -hy + 0.2, 1.9), 0.1, 0.15, 0.3, 6, true);
    }
}
