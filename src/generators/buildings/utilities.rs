//! Utility prefabs: power plant, wind turbine, solar farm, pump station,
//! substation, power pylon and cell tower. Ground at z = 0, front -Y.

use super::industry::{fence, heap, ladder, lattice, pipe, stack, tank};
use super::shops::{flat_box, sign, storefront};
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::PowerPlant => power_plant(b, hx, hy),
        K::WindTurbine => wind_turbine(b, hx),
        K::SolarFarm => solar_farm(b, hx, hy),
        K::PumpStation => pump_station(b, hx, hy),
        K::Substation => substation(b, hx, hy),
        K::PowerPylon => pylon(&mut b.kit, hx),
        K::CellTower => cell_tower(b, hx, hy),
        _ => {}
    }
}

/// A hyperbolic cooling tower at `c`, `r` wide at the base, `h` tall.
fn cooling_tower(k: &mut Kit, c: Vec3, r: f32, h: f32) {
    let n = 10;
    let rad = |t: f32| r * (0.62 + 0.38 * ((t - 0.78) / 0.78).powi(2).min(1.0)).sqrt().max(0.6);
    k.mat(ids::PLASTER);
    for i in 0..n {
        let (t0, t1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
        k.frustum(c + Vec3::Z * (h * t0), rad(t0), rad(t1), h / n as f32, 28, false);
    }
    // Inner wall and the legs ring at the base.
    k.mat(ids::CHARRED).frustum(c + Vec3::Z * (h - 2.0), rad(1.0) - 0.3, rad(1.0) - 0.3, 1.9, 28, true);
    k.mat(ids::CURB).frustum(c, r + 0.6, r + 0.6, 0.6, 28, true);
}

fn power_plant(b: &mut Ctx, hx: f32, hy: f32) {
    // Turbine hall and taller boiler house at the front, a tall stack,
    // two cooling towers behind, transformers and a coal heap.
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    let (y0, y1) = (-hy + 6.0, -hy + 26.0);
    flat_box(&mut b.kit, -hx + 4.0, y0, hx * 0.2, y1, 0.0, 18.0, ids::BRICK, 0.8);
    flat_box(&mut b.kit, hx * 0.2, y0 + 2.0, hx * 0.55, y1, 0.0, 32.0, ids::PLASTER, 0.8);
    let n = (((hx * 1.2 - 4.0) / 6.0) as usize).max(2);
    for i in 0..n {
        let x = -hx + 4.0 + (hx * 1.2 - 4.0) * (i as f32 + 0.5) / n as f32;
        let pane = b.pane(0.3);
        crate::generators::settlement::window(&mut b.kit, x, y0 - 0.02, 4.0, 3.0, 11.0, pane);
    }
    storefront(b, hx * 0.375, 6.0, y0 + 2.0, 0.05, 3.0, true);
    sign(b, -hx * 0.4, y0, 15.5, hx * 0.5);
    let r = ((hy - 13.0) * 0.5 - 2.0).clamp(8.0, 18.0);
    for i in 0..2 {
        let c = Vec3::new(-hx * 0.55 + i as f32 * (2.0 * r + 4.0), hy - r - 3.0, 0.0);
        cooling_tower(&mut b.kit, c, r, r * 3.0);
    }
    if b.p.parts.chimneys {
        stack(&mut b.kit, Vec3::new(hx * 0.72, y1 + 6.0, 0.0), 3.0, 90.0, ids::PLASTER);
    }
    for i in 0..3 {
        transformer(&mut b.kit, Vec3::new(hx * 0.7 + (i as f32 - 1.0) * 5.0, -hy + 4.0, 0.0));
    }
    heap(&mut b.kit, Vec3::new(hx - 10.0, hy - 10.0, 0.05), 8.0, 6.0, ids::CHARRED);
    pipe(&mut b.kit, Vec3::new(hx * 0.55, y1 - 4.0, 20.0), Vec3::new(hx * 0.72, y1 + 4.0, 20.0), 1.0);
}

fn transformer(k: &mut Kit, c: Vec3) {
    k.mat(ids::CURB).block(c - Vec3::new(1.8, 1.4, 0.0), c + Vec3::new(1.8, 1.4, 0.3));
    k.mat(ids::IRON).block(c + Vec3::new(-1.2, -0.9, 0.3), c + Vec3::new(1.2, 0.9, 2.6));
    for i in 0..6 {
        let x = -1.1 + i as f32 * 0.44;
        k.block(c + Vec3::new(x - 0.05, -1.3, 0.6), c + Vec3::new(x + 0.05, -0.9, 2.3));
    }
    k.mat(ids::SLATE);
    for x in [-0.7, 0.0, 0.7] {
        k.frustum(c + Vec3::new(x, 0.2, 2.6), 0.14, 0.08, 1.4, 6, true);
    }
}

fn wind_turbine(b: &mut Ctx, hx: f32) {
    // Rotor radius from the width; the hub sits 1.9 radii up.
    let r = hx.max(3.0);
    let hub = r * 1.9;
    let k = &mut b.kit;
    k.mat(ids::CURB).frustum(Vec3::ZERO, r * 0.12, r * 0.12, 0.4, 12, true);
    k.mat(ids::PLASTER).frustum(Vec3::Z * 0.4, r * 0.055, r * 0.032, hub - 0.4, 16, true);
    ladder(k, Vec3::new(0.0, -r * 0.056, 0.4), 2.4);
    let (nw, nl) = (r * 0.06, r * 0.22);
    k.block(Vec3::new(-nw, -nw * 1.4, hub - nw), Vec3::new(nw, nl, hub + nw * 1.1));
    let hc = Vec3::new(0.0, -nw * 1.4, hub);
    k.at(Mat4::from_translation(hc) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.frustum(Vec3::ZERO, nw * 0.9, 0.0, nw * 2.0, 12, true));
    let a0 = b.r.gen_range(0.0..TAU);
    let k = &mut b.kit;
    for i in 0..3 {
        let a = a0 + i as f32 * TAU / 3.0;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let p = hc - Vec3::Y * 0.6;
        k.beam(p, p + d * r * 0.3, r * 0.02, r * 0.09);
        k.beam(p + d * r * 0.3, p + d * r, r * 0.012, r * 0.05);
    }
    if b.p.parts.rooftop {
        let g = b.glow(false);
        b.kit.mat(g).block(Vec3::new(-0.3, nl - 0.6, hub + nw * 1.1), Vec3::new(0.3, nl - 0.1, hub + nw * 1.1 + 0.3));
    }
}

fn solar_farm(b: &mut Ctx, hx: f32, hy: f32) {
    // Rows of tilted panels facing -Y on posts, an inverter cabin, a fence.
    b.kit.mat(ids::LOT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    let tilt = 0.45f32;
    let depth = 3.4;
    let rows = (((2.0 * hy - 8.0) / 6.5) as usize).max(1);
    let pane = ids::GLASS;
    for j in 0..rows {
        let y = -hy + 6.0 + j as f32 * 6.5;
        let (x0, x1) = (-hx + 2.0, hx - 2.0);
        let m = Mat4::from_translation(Vec3::new(0.0, y, 1.0)) * Mat4::from_rotation_x(tilt);
        let segs = (((x1 - x0) / 4.0) as usize).max(1);
        b.kit.at(m, |k| {
            for s in 0..segs {
                let a = x0 + (x1 - x0) * s as f32 / segs as f32;
                let e = x0 + (x1 - x0) * (s + 1) as f32 / segs as f32;
                k.mat(pane).block(Vec3::new(a + 0.05, 0.0, 0.0), Vec3::new(e - 0.05, depth, 0.08));
            }
            k.mat(ids::IRON).block(Vec3::new(x0, depth * 0.45, -0.12), Vec3::new(x1, depth * 0.55, 0.0));
        });
        b.kit.mat(ids::IRON);
        let posts = ((x1 - x0) / 4.0) as usize;
        for i in 0..=posts {
            let x = x0 + (x1 - x0) * i as f32 / posts.max(1) as f32;
            let yy = y + depth * 0.5 * tilt.cos();
            b.kit.block(Vec3::new(x - 0.06, yy - 0.06, 0.0), Vec3::new(x + 0.06, yy + 0.06, 1.0 + depth * 0.5 * tilt.sin()));
        }
    }
    flat_box(&mut b.kit, -hx + 2.0, -hy + 1.0, -hx + 8.0, -hy + 3.5, 0.0, 2.8, ids::PLASTER, 0.1);
    fence(&mut b.kit, Vec2::new(-hx + 0.3, -hy + 0.3), Vec2::new(hx - 0.3, hy - 0.3), 2.0, 5.0);
}

fn pump_station(b: &mut Ctx, hx: f32, hy: f32) {
    // A gabled brick pump house, big pipes in and out, valve wheels, a
    // small surge tank and an intake basin.
    b.kit.mat(ids::CURB).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.2));
    let (bx, by, h) = (hx * 0.6, hy * 0.6, 5.5);
    let c = Vec3::new(-hx * 0.2, 0.0, 0.0);
    b.kit.mat(ids::BRICK).block(c + Vec3::new(-bx, -by, 0.0), c + Vec3::new(bx, by, h));
    b.kit.at(Mat4::from_translation(c + Vec3::Z * h), |k| k.mat(ids::SLATE).gable(bx, by, by * 0.6, 0.4, ids::BRICK));
    super::houses::door(&mut b.kit, c.x, c.y - by - 0.02, 0.2, 2.4, 3.0);
    for s in [-1.0, 1.0] {
        let pane = b.pane(0.3);
        crate::generators::settlement::window(&mut b.kit, c.x + s * bx * 0.6, c.y - by - 0.02, 2.0, 1.2, 2.2, pane);
    }
    let k = &mut b.kit;
    for (i, y) in [-by * 0.4, by * 0.4].into_iter().enumerate() {
        let a = Vec3::new(c.x + bx, y, 1.2);
        let e = Vec3::new(hx - 0.6, y, 1.2);
        pipe(k, a, e, 0.45);
        pipe(k, e, e - Vec3::Z * 1.0, 0.45);
        let v = a.lerp(e, 0.5);
        k.mat(ids::RUST).at(Mat4::from_translation(v + Vec3::Z * 0.9) * Mat4::from_rotation_x(0.0), |k| k.frustum(Vec3::ZERO, 0.4, 0.4, 0.06, 12, true));
        k.mat(ids::IRON).block(v + Vec3::new(-0.04, -0.04, 0.4), v + Vec3::new(0.04, 0.04, 0.9));
        if i == 0 {
            tank(k, Vec3::new(hx - 1.8, hy - 1.8, 0.2), 1.4, 4.0, ids::IRON, false);
        }
    }
    k.mat(ids::CURB).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(-hx + 0.3, hy, 1.0));
    k.mat(ids::WATER).block(Vec3::new(-hx + 0.3, -hy + 0.4, 0.05), Vec3::new(c.x - bx - 0.4, hy - 0.4, 0.25));
}

fn substation(b: &mut Ctx, hx: f32, hy: f32) {
    // Gravel pad, gantries with hanging insulators, transformers and a
    // control building, all fenced.
    b.kit.mat(ids::DUST).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.08));
    let k = &mut b.kit;
    let n = ((hx * 2.0 / 9.0) as usize).clamp(2, 5);
    for i in 0..n {
        let x = -hx + 3.0 + (2.0 * hx - 6.0) * i as f32 / (n - 1) as f32;
        transformer(k, Vec3::new(x, -hy * 0.2, 0.08));
    }
    // Two gantries across the yard.
    for y in [hy * 0.35, -hy * 0.65] {
        let (xa, xb) = (-hx + 2.0, hx - 2.0);
        for x in [xa, xb] {
            lattice(k, Vec3::new(x, y, 0.08), 0.5, 0.35, 10.0, 2.5, 0.1);
        }
        k.mat(ids::IRON).beam(Vec3::new(xa, y, 10.1), Vec3::new(xb, y, 10.1), 0.5, 0.5);
        let m = ((xb - xa) / 3.0) as usize;
        for j in 1..m {
            let x = xa + (xb - xa) * j as f32 / m as f32;
            k.mat(ids::SLATE).frustum(Vec3::new(x, y, 8.6), 0.12, 0.12, 1.4, 6, true);
        }
    }
    flat_box(k, hx - 9.0, hy - 6.0, hx - 1.5, hy - 1.5, 0.0, 3.5, ids::BRICK, 0.4);
    super::houses::door(k, hx - 5.25, hy - 6.02, 0.08, 1.2, 2.2);
    fence(k, Vec2::new(-hx + 0.3, -hy + 0.3), Vec2::new(hx - 0.3, hy - 0.3), 2.6, 4.0);
}

fn pylon(k: &mut Kit, hx: f32) {
    // A lattice transmission tower with three cross-arms and hanging
    // insulator strings, plus an earth peak.
    let h = 34.0;
    let base = hx.clamp(1.5, 5.0);
    for c in [Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)] {
        k.mat(ids::CURB).block((c * base - Vec2::splat(0.4)).extend(0.0), (c * base + Vec2::splat(0.4)).extend(0.4));
    }
    lattice(k, Vec3::ZERO, base, 0.8, h, 3.4, 0.16);
    for (z, reach) in [(h - 10.0, 7.0), (h - 5.0, 9.0), (h - 0.2, 6.0)] {
        for s in [-1.0f32, 1.0] {
            let tip = Vec3::new(s * reach, 0.0, z);
            k.mat(ids::IRON).beam(Vec3::new(s * 0.8, -0.8, z), tip, 0.12, 0.12);
            k.beam(Vec3::new(s * 0.8, 0.8, z), tip, 0.12, 0.12);
            k.beam(Vec3::new(s * 0.8, 0.0, z - 1.4), tip, 0.1, 0.1);
            k.mat(ids::SLATE).frustum(tip - Vec3::Z * 2.4, 0.12, 0.12, 2.4, 6, true);
        }
    }
    k.mat(ids::IRON).frustum(Vec3::Z * h, 0.6, 0.0, 3.0, 4, false);
}

fn cell_tower(b: &mut Ctx, hx: f32, hy: f32) {
    // A monopole with a three-sector antenna head, a microwave dish, an
    // equipment cabin and a fence.
    b.kit.mat(ids::DUST).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.06));
    let h = 30.0;
    let k = &mut b.kit;
    k.mat(ids::CURB).block(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 0.4));
    k.mat(ids::IRON).frustum(Vec3::Z * 0.4, 0.55, 0.3, h, 12, true);
    ladder(k, Vec3::new(0.0, -0.6, 0.4), h - 1.0);
    k.mat(ids::IRON).frustum(Vec3::Z * (h - 3.0), 2.2, 2.2, 0.2, 3, true);
    for s in 0..3 {
        let a = s as f32 * TAU / 3.0 + FRAC_PI_2 / 3.0;
        let d = Vec2::new(a.cos(), a.sin());
        for j in -1i32..=1 {
            let p = d * 1.9 + d.perp() * (j as f32 * 0.5);
            k.at(Mat4::from_translation(p.extend(h - 3.0)) * Mat4::from_rotation_z(a), |k| {
                k.mat(ids::PLASTER).block(Vec3::new(-0.08, -0.15, 0.2), Vec3::new(0.08, 0.15, 2.6));
            });
        }
    }
    k.at(Mat4::from_translation(Vec3::new(0.0, -0.6, h - 8.0)) * Mat4::from_rotation_x(FRAC_PI_2), |k| {
        k.mat(ids::PLASTER).frustum(Vec3::ZERO, 0.6, 0.65, 0.35, 14, true);
    });
    flat_box(k, hx - 4.5, -hy + 1.0, hx - 1.0, -hy + 3.4, 0.0, 2.6, ids::PLASTER, 0.1);
    fence(k, Vec2::new(-hx + 0.3, -hy + 0.3), Vec2::new(hx - 0.3, hy - 0.3), 2.4, 3.0);
}
