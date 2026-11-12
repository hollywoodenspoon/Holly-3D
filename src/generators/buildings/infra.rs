//! Bridges & infrastructure: stone arch bridge, steel truss bridge,
//! drawbridge, quay / pier and a dockside crane. Bridges span along Y
//! (`Length`) with a deck `Width` wide at about 6 m; Floors = arch count
//! for the stone bridge and boom length for the crane.

use super::{BuildingKind as K, Ctx};
use crate::generators::kit::ids;
use bevy::math::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_2;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::ArchBridge => arch_bridge(b, hx, hy, floors.clamp(1, 12)),
        K::TrussBridge => truss_bridge(b, hx, hy),
        K::Drawbridge => drawbridge(b, hx, hy),
        K::Quay => quay(b, hx, hy),
        K::Crane => crane(b, hx, hy, floors),
        _ => {}
    }
}

/// Deck height of the bridges.
pub const DECK: f32 = 6.0;

fn arch_bridge(b: &mut Ctx, hx: f32, hy: f32, arches: u32) {
    let span = 2.0 * hy / arches as f32;
    let pier = (span * 0.15).clamp(0.6, 2.5);
    let rise = (DECK - 1.0).min((span - pier) * 0.5);
    for i in 0..arches {
        let y = -hy + span * (i as f32 + 0.5);
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, y, DECK - 0.6 - rise)) * Mat4::from_rotation_z(FRAC_PI_2), |k| {
            k.mat(ids::FACADE).arch_wall(span - pier, rise, rise + 0.6, 2.0 * hx, false)
        });
    }
    b.kit.mat(ids::FACADE);
    for i in 0..=arches {
        let y = -hy + span * i as f32;
        b.kit.block(Vec3::new(-hx, y - pier * 0.5, -1.0), Vec3::new(hx, y + pier * 0.5, DECK - 0.6 - rise));
        // Cutwaters on the upstream side.
        b.kit.frustum(Vec3::new(-hx, y, -1.0), pier * 0.5, pier * 0.5, DECK - 0.6 - rise + 1.0, 6, true);
    }
    b.kit.mat(ids::COBBLE).block(Vec3::new(-hx, -hy, DECK - 0.6), Vec3::new(hx, hy, DECK));
    b.kit.mat(ids::FACADE);
    for s in [-1.0, 1.0] {
        b.kit.block(Vec3::new(s * hx - 0.3, -hy, DECK), Vec3::new(s * hx + 0.3, hy, DECK + 1.0));
    }
    lamps(b, hx, hy, true);
}

fn truss_bridge(b: &mut Ctx, hx: f32, hy: f32) {
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, DECK - 0.5), Vec3::new(hx, hy, DECK));
    b.kit.mat(ids::LANE).block(Vec3::new(-0.08, -hy, DECK), Vec3::new(0.08, hy, DECK + 0.01));
    // Two Pratt trusses: chords, verticals and alternating diagonals.
    let h = 6.0;
    let n = ((2.0 * hy / 6.0).round() as usize).max(2);
    b.kit.mat(ids::IRON);
    for s in [-1.0, 1.0] {
        let x = s * (hx + 0.2);
        let p = |i: usize, top: bool| Vec3::new(x, -hy + 2.0 * hy * i as f32 / n as f32, DECK + if top { h } else { 0.0 });
        b.kit.beam(p(0, false), p(n, false), 0.4, 0.4);
        b.kit.beam(p(1, true), p(n - 1, true), 0.4, 0.4);
        b.kit.beam(p(0, false), p(1, true), 0.35, 0.35);
        b.kit.beam(p(n, false), p(n - 1, true), 0.35, 0.35);
        for i in 1..n {
            b.kit.beam(p(i, false), p(i, true), 0.25, 0.25);
            if i < n - 1 {
                let (a, c) = if i < n / 2 { (p(i, true), p(i + 1, false)) } else { (p(i, false), p(i + 1, true)) };
                b.kit.beam(a, c, 0.2, 0.2);
            }
        }
    }
    for i in 1..n {
        let y = -hy + 2.0 * hy * i as f32 / n as f32;
        b.kit.beam(Vec3::new(-hx - 0.2, y, DECK + h), Vec3::new(hx + 0.2, y, DECK + h), 0.2, 0.2);
    }
    b.kit.mat(ids::FACADE);
    for y in [-hy, hy] {
        b.kit.block(Vec3::new(-hx - 1.0, y - 1.5, -1.0), Vec3::new(hx + 1.0, y + 1.5, DECK - 0.5));
    }
    lamps(b, hx, hy, false);
}

fn drawbridge(b: &mut Ctx, hx: f32, hy: f32) {
    // Stone abutments, a timber leaf (lowered), chains to a gate tower.
    let gap = hy * 0.55;
    b.kit.mat(ids::FACADE);
    b.kit.block(Vec3::new(-hx, -hy, -2.0), Vec3::new(hx, -gap, DECK));
    b.kit.block(Vec3::new(-hx, gap, -2.0), Vec3::new(hx, hy, DECK));
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx * 0.8, -gap, DECK - 0.35), Vec3::new(hx * 0.8, gap, DECK));
    b.kit.mat(ids::IRON);
    for x in [-hx * 0.8, hx * 0.8] {
        for i in 0..6 {
            b.kit.block(Vec3::new(x - 0.1, -gap + i as f32 * 0.6, DECK), Vec3::new(x + 0.1, -gap + i as f32 * 0.6 + 0.12, DECK + 0.05));
        }
    }
    let t = Vec3::new(0.0, gap + 2.5, DECK);
    b.kit.mat(ids::FACADE).block(t - Vec3::new(hx + 1.0, 2.0, 0.0), t + Vec3::new(hx + 1.0, 2.0, 8.0));
    b.kit.at(Mat4::from_translation(t - Vec3::Y * 2.03), |k| k.mat(ids::TRIM).arched_panel(hx * 1.4, 4.0, false));
    b.kit.mat(ids::IRON);
    for x in [-hx * 0.8, hx * 0.8] {
        b.kit.beam(Vec3::new(x, -gap + 0.2, DECK + 0.1), Vec3::new(x, gap + 0.4, DECK + 6.0), 0.08, 0.08);
    }
    b.kit.at(Mat4::from_translation(t), |k| super::medieval_a::battlements(k, hx + 1.0, 2.0, 8.0));
}

fn quay(b: &mut Ctx, hx: f32, hy: f32) {
    // Stone quay wall, timber pier on piles, bollards and steps to the water.
    b.kit.mat(ids::WATER).block(Vec3::new(-hx - 12.0, -hy, -1.6), Vec3::new(-hx, hy, -1.5));
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy, -2.5), Vec3::new(hx, hy, 0.0));
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.1));
    let pw = 3.0;
    let len = 10.0;
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - len, -pw * 0.5, -0.3), Vec3::new(-hx, pw * 0.5, 0.0));
    for i in 0..=4 {
        let x = -hx - len * i as f32 / 4.0;
        for y in [-pw * 0.5 + 0.2, pw * 0.5 - 0.2] {
            b.kit.frustum(Vec3::new(x, y, -3.0), 0.18, 0.18, 3.0, 6, true);
        }
    }
    b.kit.mat(ids::IRON);
    let n = ((2.0 * hy / 6.0) as usize).max(1);
    for i in 0..=n {
        let y = -hy + 2.0 * hy * i as f32 / n as f32;
        b.kit.frustum(Vec3::new(-hx + 0.5, y, 0.1), 0.2, 0.25, 0.6, 8, true);
    }
    if b.p.parts.porches {
        let st = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(-hx + 0.6, hy - 2.0, -1.5)) * Mat4::from_rotation_z(-FRAC_PI_2), |k| {
            super::arch::stairs(k, st, 1.2, 1.5);
        });
    }
    lamps(b, hx, hy, true);
}

fn crane(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Portal (gantry) legs on rails, a cab, a lattice boom and a hook.
    let h = 10.0;
    b.kit.mat(ids::RUST);
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        b.kit.beam(Vec3::new(sx * hx, sy * hx, 0.0), Vec3::new(sx * hx * 0.6, sy * hx * 0.6, h), 0.5, 0.5);
    }
    b.kit.mat(ids::IRON);
    for x in [-hx, hx] {
        b.kit.block(Vec3::new(x - 0.15, -hy, 0.0), Vec3::new(x + 0.15, hy, 0.15));
    }
    b.kit.mat(ids::RUST).block(Vec3::new(-hx * 0.7, -hx * 0.7, h), Vec3::new(hx * 0.7, hx * 0.7, h + 1.0));
    b.kit.mat(ids::PLASTER).block(Vec3::new(-1.6, -1.5, h + 1.0), Vec3::new(1.6, 2.5, h + 4.0));
    let pane = b.pane(0.6);
    crate::generators::settlement::window(&mut b.kit, 0.0, -1.52, h + 2.0, 2.4, 1.4, pane);
    let boom = hy * 1.2 + floors as f32;
    let (root, tip) = (Vec3::new(0.0, -1.5, h + 2.0), Vec3::new(0.0, -1.5 - boom * 0.85, h + 2.0 + boom * 0.5));
    let dir = (tip - root).normalize();
    b.kit.mat(ids::RUST);
    for (dx, dz) in [(-0.6, 0.0), (0.6, 0.0)] {
        b.kit.beam(root + Vec3::new(dx, 0.0, dz), tip + Vec3::new(dx * 0.3, 0.0, dz), 0.18, 0.18);
    }
    let m = (boom / 1.6) as usize;
    for i in 0..m {
        let p = root + dir * boom * i as f32 / m as f32;
        b.kit.beam(p + Vec3::X * -0.55, p + dir * boom / m as f32 + Vec3::X * 0.5, 0.08, 0.08);
    }
    b.kit.mat(ids::IRON).beam(tip, tip - Vec3::Z * (h * 0.8), 0.04, 0.04);
    b.kit.block(tip - Vec3::new(0.3, 0.3, h * 0.8 + 0.6), tip - Vec3::new(-0.3, -0.3, h * 0.8));
    if b.p.parts.signs {
        let g = b.glow(true);
        b.kit.mat(g).cuboid(tip + Vec3::Z * 0.3, Vec3::splat(0.2), bevy::math::Quat::IDENTITY);
    }
}

/// Lamps along both parapets of a bridge / quay.
fn lamps(b: &mut Ctx, hx: f32, hy: f32, lantern: bool) {
    let z = if b.p.kind == K::Quay { 0.1 } else { DECK };
    let g = b.glow(false);
    let n = ((2.0 * hy / 12.0) as usize).max(1);
    for i in 0..=n {
        let y = -hy + 2.0 * hy * i as f32 / n as f32;
        for s in [-1.0, 1.0] {
            let at = Vec3::new(s * hx, y, z);
            if lantern {
                b.kit.mat(ids::IRON).frustum(at, 0.08, 0.06, 3.0, 6, true);
                b.kit.mat(g).frustum(at + Vec3::Z * 3.0, 0.18, 0.12, 0.4, 6, true);
            } else {
                super::doodads::streetlamp(&mut b.kit, at, bevy::math::Vec2::new(-s, 0.0), g);
            }
        }
    }
}
