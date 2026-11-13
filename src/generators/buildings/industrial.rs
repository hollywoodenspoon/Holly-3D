//! Industrial prefabs: factory, truck depot, silo cluster, grain
//! elevator, refinery, tank farm and recycling yard. Ground at z = 0,
//! the gate / main front facing -Y.

use super::houses::framed;
use super::industry::{container, dock_door, fence, heap, ladder, lattice, pipe, pipe_rack, silo, stack, tank};
use super::shops::{flat_box, sign, storefront};
use super::{BuildingKind as K, Ctx, doodads};
use crate::generators::kit::ids;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Factory => factory(b, hx, hy),
        K::TruckDepot => depot(b, hx, hy),
        K::SiloCluster => silos(b, hx, hy),
        K::GrainElevator => elevator(b, hx, hy),
        K::Refinery => refinery(b, hx, hy),
        K::TankFarm => tank_farm(b, hx, hy),
        K::RecyclingYard => recycling(b, hx, hy),
        _ => {}
    }
}

fn yard(b: &mut Ctx, hx: f32, hy: f32, gate: f32) {
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    fence(&mut b.kit, Vec2::new(-hx + 0.3, -hy + 0.3), Vec2::new(hx - 0.3, hy - 0.3), 2.2, gate);
}

fn factory(b: &mut Ctx, hx: f32, hy: f32) {
    // Several gabled brick halls side by side, a flat office block at the
    // front, tall chimneys and a loading door.
    let (h, y0) = (8.0, -hy + 8.0);
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.05));
    let n = ((hx * 2.0 / 12.0).round() as usize).clamp(2, 6);
    let bay = 2.0 * hx / n as f32;
    let (cy, dy) = ((y0 + hy) * 0.5, (hy - y0) * 0.5);
    for i in 0..n {
        let x = -hx + bay * (i as f32 + 0.5);
        b.kit.mat(ids::BRICK).block(Vec3::new(x - bay * 0.5, y0, 0.0), Vec3::new(x + bay * 0.5, hy, h));
        let m = Mat4::from_translation(Vec3::new(x, cy, h)) * Mat4::from_rotation_z(FRAC_PI_2);
        b.kit.at(m, |k| k.mat(ids::RUST).gable(dy, bay * 0.5, bay * 0.28, 0.2, ids::BRICK));
        let pane = b.pane(0.3);
        crate::generators::settlement::window(&mut b.kit, x, y0 - 0.02, 3.2, bay * 0.6, 3.6, pane);
    }
    // Office block.
    flat_box(&mut b.kit, -hx * 0.95, y0 - 6.0, -hx * 0.35, y0, 0.0, 7.0, ids::PLASTER, 0.6);
    storefront(b, -hx * 0.65, hx * 0.4, y0 - 6.0, 0.1, 2.5, true);
    let pane = b.pane(0.4);
    crate::generators::settlement::window(&mut b.kit, -hx * 0.65, y0 - 6.02, 4.0, hx * 0.5, 1.6, pane);
    sign(b, -hx * 0.65, y0 - 6.0, 6.0, hx * 0.4);
    dock_door(&mut b.kit, hx * 0.5, y0, 4.0);
    if b.p.parts.chimneys {
        for (i, x) in [hx * 0.2, hx * 0.7].into_iter().enumerate() {
            stack(&mut b.kit, Vec3::new(x, hy - 3.0, 0.0), 1.4, 26.0 + 6.0 * i as f32, ids::BRICK);
        }
    }
    if b.p.parts.rooftop {
        pipe(&mut b.kit, Vec3::new(-hx * 0.2, y0 + 1.0, h + 1.0), Vec3::new(hx * 0.6, y0 + 1.0, h + 1.0), 0.35);
    }
}

fn depot(b: &mut Ctx, hx: f32, hy: f32) {
    // Long cross-dock shed with a row of dock doors, trailers backed up to
    // them, a yard, a guard hut and a fence.
    yard(b, hx, hy, 10.0);
    let (y0, y1, h) = (-hy + 18.0, hy - 3.0, 9.0);
    flat_box(&mut b.kit, -hx + 3.0, y0, hx - 3.0, y1, 0.0, h, ids::PLASTER, 0.5);
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx + 2.95, y0 - 0.05, h - 1.0), Vec3::new(hx - 2.95, y0, h - 0.6));
    let n = (((2.0 * hx - 6.0) / 5.0) as usize).max(2);
    for i in 0..n {
        let x = -hx + 3.0 + (2.0 * hx - 6.0) * (i as f32 + 0.5) / n as f32;
        dock_door(&mut b.kit, x, y0, 3.2);
        if b.r.gen_bool(0.55) {
            // A trailer backed to the door.
            b.kit.mat(ids::PLASTER).block(Vec3::new(x - 1.25, y0 - 14.5, 1.2), Vec3::new(x + 1.25, y0 - 1.4, 4.0));
            b.kit.mat(ids::ASPHALT);
            for y in [y0 - 3.5, y0 - 4.9] {
                b.kit.at(Mat4::from_translation(Vec3::new(x, y, 0.5)) * Mat4::from_rotation_y(FRAC_PI_2), |k| k.frustum(Vec3::Z * -1.2, 0.5, 0.5, 2.4, 10, true));
            }
            b.kit.mat(ids::IRON).block(Vec3::new(x - 0.1, y0 - 13.5, 0.0), Vec3::new(x + 0.1, y0 - 13.3, 1.2));
        }
    }
    let g = (hx - 6.0, -hy + 2.0);
    flat_box(&mut b.kit, g.0 - 1.5, g.1, g.0 + 1.5, g.1 + 2.5, 0.0, 2.8, ids::PLASTER, 0.2);
    storefront(b, g.0, 2.4, g.1, 0.9, 1.2, false);
    b.kit.mat(ids::STUCCO).block(Vec3::new(-4.0, -hy + 0.2, 1.0), Vec3::new(4.0, -hy + 0.3, 1.1));
    sign(b, 0.0, y0, h - 0.5, hx * 0.6);
}

fn silos(b: &mut Ctx, hx: f32, hy: f32) {
    // A row (or two) of silos joined by a conveyor gallery on top.
    b.kit.mat(ids::CURB).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.3));
    let rows = if hy > 7.0 { 2 } else { 1 };
    let r = (hy / rows as f32 - 0.3).min(hx * 0.5 - 0.2).clamp(1.0, 4.5);
    let n = ((hx / (r + 0.2)) as usize).max(1);
    let h = 18.0 + b.r.gen_range(0.0..8.0);
    let wall = if b.r.gen_bool(0.5) { ids::PLASTER } else { ids::IRON };
    for row in 0..rows {
        let y = -hy + (2 * row + 1) as f32 * hy / rows as f32;
        for i in 0..n {
            let x = -hx + (2 * i + 1) as f32 * hx / n as f32;
            silo(&mut b.kit, Vec3::new(x, y, 0.3), r, h, wall);
        }
    }
    let top = h + 0.3 + r * 0.5;
    b.kit.mat(ids::RUST).block(Vec3::new(-hx, -0.8, top - 0.6), Vec3::new(hx + 4.0, 0.8, top + 1.4));
    b.kit.mat(ids::RUST).beam(Vec3::new(hx + 4.0, 0.0, top), Vec3::new(hx + 12.0, 0.0, 1.6), 1.2, 1.0);
    lattice(&mut b.kit, Vec3::new(hx + 4.0, 0.0, 0.0), 1.0, 0.8, top - 0.6, 3.0, 0.15);
    ladder(&mut b.kit, Vec3::new(-hx + hx / n as f32, -hy - 0.1, 0.3), h);
}

fn elevator(b: &mut Ctx, hx: f32, hy: f32) {
    // Tall concrete headhouse at the front, a row of bins behind, a leg
    // tower and a spout down to the truck bay.
    b.kit.mat(ids::CURB).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.3));
    let r = (hx * 0.5 - 0.1).min(3.5);
    let h = 28.0;
    let mut y = -hy + 8.0 + r;
    while y + r <= hy + 0.01 {
        for s in [-1.0, 1.0] {
            silo(&mut b.kit, Vec3::new(s * r, y, 0.3), r, h, ids::PLASTER);
        }
        y += 2.0 * r;
    }
    flat_box(&mut b.kit, -hx * 0.8, -hy + 2.0, hx * 0.8, -hy + 8.0, 0.0, h + 10.0, ids::PLASTER, 0.4);
    b.kit.mat(ids::IRON).block(Vec3::new(-hx * 0.5, -hy + 3.0, h + 10.0), Vec3::new(hx * 0.5, -hy + 7.0, h + 14.0));
    let pane = b.pane(0.2);
    for z in [h + 2.0, h + 6.0] {
        crate::generators::settlement::window(&mut b.kit, 0.0, -hy + 1.98, z, hx * 1.2, 1.2, pane);
    }
    // Truck bay with a spout.
    b.kit.mat(ids::ROOF).block(Vec3::new(-hx, -hy - 4.0, 5.0), Vec3::new(hx, -hy + 2.0, 5.3));
    for s in [-1.0, 1.0] {
        b.kit.mat(ids::IRON).block(Vec3::new(s * (hx - 0.2) - 0.15, -hy - 3.9, 0.0), Vec3::new(s * (hx - 0.2) + 0.15, -hy - 3.6, 5.0));
    }
    pipe(&mut b.kit, Vec3::new(0.0, -hy + 2.0, h), Vec3::new(0.0, -hy - 2.0, 5.3), 0.3);
    sign(b, 0.0, -hy + 2.0, h + 7.0, hx * 1.2);
}

fn refinery(b: &mut Ctx, hx: f32, hy: f32) {
    // Distillation columns with platforms, a flare stack, heaters, a pipe
    // rack, spheres and tanks, inside a fence.
    yard(b, hx, hy, 8.0);
    let k = &mut b.kit;
    pipe_rack(k, Vec2::new(-hx + 4.0, 0.0), Vec2::new(hx - 4.0, 0.0), 6.0, 5);
    let cols = ((hx / 8.0) as usize).clamp(2, 6);
    for i in 0..cols {
        let x = -hx + 6.0 + (hx - 8.0) * i as f32 / cols.max(2) as f32 * 1.0;
        let (r, h) = (1.1 + (i % 2) as f32 * 0.7, 26.0 + (i * 7 % 13) as f32);
        let c = Vec3::new(x, -6.0, 0.0);
        k.mat(ids::PLASTER).frustum(c, r, r, h, 14, true);
        for z in (6..h as usize).step_by(7) {
            k.mat(ids::RUST).frustum(c + Vec3::Z * z as f32, r + 1.0, r + 1.0, 0.15, 14, true);
        }
        ladder(k, c + Vec3::new(0.0, -r - 0.2, 0.0), h);
    }
    if b.p.parts.chimneys {
        stack(&mut b.kit, Vec3::new(hx - 6.0, hy - 6.0, 0.0), 0.8, 45.0, ids::IRON);
        let g = b.glow(true);
        b.kit.mat(g).frustum(Vec3::new(hx - 6.0, hy - 6.0, 45.0), 0.5, 0.0, 2.5, 8, false);
    }
    let k = &mut b.kit;
    // Heaters: boxes with short stacks.
    for i in 0..2 {
        let x = hx * 0.2 + i as f32 * 9.0;
        k.mat(ids::BRICK).block(Vec3::new(x - 3.0, -12.0, 0.0), Vec3::new(x + 3.0, -6.0, 9.0));
        stack(k, Vec3::new(x, -9.0, 9.0), 0.6, 12.0, ids::IRON);
    }
    for i in 0..3 {
        let c = Vec3::new(-hx + 10.0 + i as f32 * 12.0, hy * 0.5, 0.0);
        tank(k, c, 5.0, 9.0, ids::PLASTER, i == 0);
    }
}

fn tank_farm(b: &mut Ctx, hx: f32, hy: f32) {
    // Big squat tanks each in a low bund wall, joined by pipes.
    yard(b, hx, hy, 8.0);
    let nx = ((hx / 10.0) as usize).clamp(1, 4);
    let ny = ((hy / 10.0) as usize).clamp(1, 3);
    let (cx, cy) = (2.0 * (hx - 2.0) / nx as f32, 2.0 * (hy - 4.0) / ny as f32);
    let r = (cx.min(cy) * 0.5 - 2.0).max(2.0);
    for j in 0..ny {
        for i in 0..nx {
            let c = Vec3::new(-hx + 2.0 + cx * (i as f32 + 0.5), -hy + 6.0 + cy * (j as f32 + 0.5), 0.0);
            let h = r * b.r.gen_range(0.8..1.3);
            let wall = if b.r.gen_bool(0.7) { ids::PLASTER } else { ids::IRON };
            let k = &mut b.kit;
            k.mat(ids::CURB);
            let (a, e) = ((c.truncate() - Vec2::splat(r + 1.4)).extend(0.0), (c.truncate() + Vec2::splat(r + 1.4)).extend(1.0));
            k.block(a, Vec3::new(e.x, a.y + 0.3, e.z));
            k.block(Vec3::new(a.x, e.y - 0.3, 0.0), e);
            k.block(a, Vec3::new(a.x + 0.3, e.y, e.z));
            k.block(Vec3::new(e.x - 0.3, a.y, 0.0), e);
            tank(k, c, r, h, wall, (i + j) % 2 == 0);
            pipe(k, c + Vec3::new(0.0, -r, 0.8), Vec3::new(c.x, -hy + 3.0, 0.8), 0.25);
        }
    }
    b.kit.mat(ids::IRON).tube(Vec3::new(-hx + 2.0, -hy + 3.0, 0.8), Vec3::new(hx - 2.0, -hy + 3.0, 0.8), 0.35, 10);
}

fn recycling(b: &mut Ctx, hx: f32, hy: f32) {
    // A fenced yard with heaps of scrap, containers, a shed, a conveyor and
    // a scrap crane.
    yard(b, hx, hy, 8.0);
    let k = &mut b.kit;
    k.mat(ids::RUST).block(Vec3::new(-hx + 2.0, hy - 14.0, 0.0), Vec3::new(-hx + 18.0, hy - 2.0, 7.0));
    k.at(Mat4::from_translation(Vec3::new(-hx + 10.0, hy - 8.0, 7.0)), |k| k.mat(ids::IRON).gable(8.0, 6.0, 1.5, 0.3, ids::RUST));
    for i in 0..6 {
        let c = Vec3::new(b.r.gen_range(-hx * 0.2..hx - 6.0), b.r.gen_range(-hy + 6.0..hy - 6.0), 0.05);
        let mat = [ids::RUST, ids::REBAR, ids::DUST][i % 3];
        heap(&mut b.kit, c, b.r.gen_range(3.0..5.5), b.r.gen_range(2.0..4.5), mat);
    }
    for i in 0..5 {
        let c = Vec3::new(-hx + 4.0 + i as f32 * 3.0, -hy + 6.0, 0.05);
        let paint = [ids::BRICK, ids::STUCCO, ids::IRON, ids::RUST][i % 4];
        container(&mut b.kit, c, 0.0, paint);
    }
    // Conveyor up to a hopper and a crane mast with a boom.
    let k = &mut b.kit;
    k.mat(ids::RUST).beam(Vec3::new(-hx + 18.0, hy - 8.0, 5.0), Vec3::new(-hx + 30.0, hy - 8.0, 0.5), 1.0, 0.4);
    k.mat(ids::IRON).frustum(Vec3::new(-hx + 18.0, hy - 8.0, 5.0), 1.2, 2.0, 2.0, 4, true);
    let m = Vec3::new(hx * 0.3, -hy * 0.2, 0.0);
    k.mat(ids::STUCCO).block(m - Vec3::new(1.5, 2.0, 0.0), m + Vec3::new(1.5, 2.0, 2.5));
    lattice(k, m + Vec3::Z * 2.5, 0.6, 0.5, 8.0, 2.0, 0.12);
    k.mat(ids::STUCCO).beam(m + Vec3::Z * 10.5, m + Vec3::new(-9.0, 4.0, 6.0), 0.5, 0.6);
    k.mat(ids::IRON).tube(m + Vec3::new(-9.0, 4.0, 6.0), m + Vec3::new(-9.0, 4.0, 3.0), 0.05, 4);
    let car = Mat4::from_translation(Vec3::new(hx - 6.0, -hy + 4.0, 0.05)) * Mat4::from_rotation_z(0.3);
    framed(b, car, |b| doodads::car(b, 0.92, 2.2));
}
