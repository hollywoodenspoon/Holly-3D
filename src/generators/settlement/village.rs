//! Medieval Village / Hamlet: winding dirt roads, thatched cottages and
//! timber barns along them, irregular fenced farm plots with crop rows, a
//! village well, and a windmill and chapel on the hilltop.

use super::layout::{Lot, Plan, sides};
use super::{props, townhouse};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub fn build(plan: &mut Plan) {
    let p = plan.p.clone();
    let sw = p.street_width.max(2.0);
    let (min, max) = (plan.min, plan.max);
    let size = max - min;
    let hill_c = min + size * Vec2::new(0.78, 0.76);
    plan.hill = Some((hill_c, size.min_element() * 0.3, size.min_element() * 0.06));
    plan.ground(5.0);
    let ph = plan.rng.gen_range(0.0..TAU);
    let main: Vec<Vec2> = (0..=30)
        .map(|i| {
            let x = min.x + size.x * i as f32 / 30.0;
            Vec2::new(x, min.y + size.y * 0.42 + size.y * 0.08 * (x / size.x * 5.0 + ph).sin())
        })
        .collect();
    plan.road_path(&main, sw, 0.04, ids::ROAD);
    let fork = main[18];
    let up: Vec<Vec2> = (0..=12).map(|i| {
        let t = i as f32 / 12.0;
        fork.lerp(hill_c - Vec2::new(0.0, 9.0), t) + Vec2::new((t * 9.0).sin() * 4.0, 0.0)
    }).collect();
    plan.road_path(&up, sw * 0.8, 0.04, ids::ROAD);
    // Hilltop windmill and chapel.
    let mill = Lot::new(hill_c + Vec2::new(-2.0, 2.0), Vec2::splat(4.0), 0.3);
    if plan.claim(mill) {
        let z = plan.h(mill.c) - 0.5;
        plan.kit.at(mill.frame(z), windmill);
    }
    let chapel = Lot::new(hill_c + Vec2::new(12.0, -4.0), Vec2::new(4.5, 9.0), 0.1);
    if plan.claim(chapel) {
        let z = corner_min(plan, &chapel);
        plan.kit.at(chapel.frame(z), |k| self::chapel(k, chapel.half));
    }
    let well = main[12] + Vec2::new(0.0, sw * 0.5 + 3.0);
    if plan.claim(Lot::new(well, Vec2::splat(1.5), 0.0)) {
        let z = plan.h(well);
        props::well(&mut plan.kit, well.extend(z));
    }
    // Cottages and barns along the roads, then farm plots behind them.
    for pts in [main.clone(), up.clone()] {
        let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
        let mut s = 6.0;
        while s < total {
            let (at, dir) = super::suburb::sample(&pts, s);
            for side in [-1.0f32, 1.0] {
                let n = dir.perp() * side;
                let barn = plan.rng.gen_bool(0.2);
                let half = if barn { Vec2::new(6.0, 4.5) } else { Vec2::new(plan.rng.gen_range(3.2..4.5), plan.rng.gen_range(2.6..3.4)) };
                let lot = Lot::new(at + n * (sw * 0.5 + half.y + 3.0), half, n.y.atan2(n.x) - FRAC_PI_2);
                if plan.rng.gen_bool(p.density.clamp(0.0, 1.0) as f64) && plan.claim(lot) {
                    let z = corner_min(plan, &lot);
                    let (kit, r, pitch) = (&mut plan.kit, &mut plan.rng, p.roof_pitch);
                    kit.at(lot.frame(z), |k| if barn { townhouse::barn(k, half, pitch) } else { townhouse::cottage(k, half, pitch, r) });
                }
            }
            s += plan.rng.gen_range(14.0..24.0);
        }
    }
    let plot = p.block_size.clamp(10.0, 80.0);
    for _ in 0..((size.x * size.y) / (plot * plot) * 1.2) as usize {
        let c = min + size * Vec2::new(plan.rng.gen_range(0.05..0.95), plan.rng.gen_range(0.05..0.95));
        let half = Vec2::new(plot * plan.rng.gen_range(0.35..0.55), plot * plan.rng.gen_range(0.25..0.45));
        let lot = Lot::new(c, half, plan.rng.gen_range(-0.4..0.4));
        if plan.h(c) < 0.5 && plan.claim(lot) {
            farm_plot(plan, lot);
        }
    }
    for _ in 0..((size.x * size.y) / 500.0 * p.prop_density.clamp(0.0, 1.0)) as usize {
        let c = min + size * Vec2::new(plan.rng.gen_range(0.0..1.0), plan.rng.gen_range(0.0..1.0));
        if plan.claim(Lot::new(c, Vec2::splat(1.2), 0.0)) {
            let z = plan.h(c);
            let (kit, r) = (&mut plan.kit, &mut plan.rng);
            props::random_tree(kit, c.extend(z), r, 1.0);
        }
    }
}

/// Lowest ground height under a lot's corners (buildings sink into slopes).
fn corner_min(plan: &Plan, lot: &Lot) -> f32 {
    lot.corners().iter().map(|q| plan.h(*q)).fold(f32::MAX, f32::min)
}

/// Irregular fenced plot (jittered corners) with crop rows inside.
fn farm_plot(plan: &mut Plan, lot: Lot) {
    let jit: Vec<Vec2> = lot.corners().iter().map(|q| *q + Vec2::new(plan.rng.gen_range(-1.5..1.5), plan.rng.gen_range(-1.5..1.5))).collect();
    let hill = plan.hill;
    props::fence_loop(&mut plan.kit, &jit, move |q| super::layout::hill_height(hill, q));
    if plan.rng.gen_bool(0.75) {
        let inner = lot.half * 0.85;
        let spacing = plan.rng.gen_range(1.0..1.6);
        plan.kit.at(lot.frame(0.0), |k| props::crop_rows(k, inner.x, inner.y, spacing));
    }
}

/// Hilltop windmill: tapered tower, conical cap and four lattice sails.
fn windmill(k: &mut Kit) {
    k.mat(ids::PLASTER).frustum(Vec3::ZERO, 3.2, 2.2, 10.0, 12, false);
    k.mat(ids::ROOF).frustum(Vec3::Z * 10.0, 2.6, 0.0, 3.2, 12, true);
    super::window(k, 0.0, -3.25, 0.0, 1.1, 2.0, ids::TRIM);
    let hub = Vec3::new(0.0, -2.9, 9.4);
    k.mat(ids::TRIM).tube(hub, hub - Vec3::Y * 0.8, 0.35, 8);
    for i in 0..4 {
        let rot = Quat::from_rotation_y(i as f32 * FRAC_PI_2 + 0.35);
        let (dir, side) = (rot * Vec3::Z, rot * Vec3::X);
        let hub = hub - Vec3::Y * 0.7;
        k.mat(ids::TRIM).beam(hub, hub + dir * 8.0, 0.25, 0.25);
        let (a, b) = (hub + dir * 1.5, hub + dir * 7.8);
        k.mat(ids::PLASTER).quad(a, a + side * 1.6, b + side * 1.6, b);
        k.mat(ids::PLASTER).quad(b, b + side * 1.6, a + side * 1.6, a);
    }
}

/// Hilltop chapel: nave, gabled roof, front bell tower with a spire and
/// lancet windows.
fn chapel(k: &mut Kit, half: Vec2) {
    let h = 5.5;
    k.mat(ids::FACADE).block((-half).extend(0.0), half.extend(h));
    k.at(Mat4::from_translation(Vec3::Z * h) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::ROOF).gable(half.y, half.x, half.x * 1.2, 0.3, ids::FACADE));
    sides(k, half.x, half.y, |k, side, w| {
        let n = if side % 2 == 1 { 3 } else { 1 };
        for i in 0..n {
            let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
            k.at(Mat4::from_translation(Vec3::new(x, 0.0, 1.5)), |k| k.mat(ids::GLASS).arched_panel(1.0, 3.0, true));
        }
    });
    let t = Vec2::splat(1.6);
    let tc = Vec2::new(0.0, -half.y - t.y);
    k.mat(ids::FACADE).block((tc - t).extend(0.0), (tc + t).extend(h + 5.0));
    k.at(Mat4::from_translation(tc.extend(h + 5.0)), |k| k.mat(ids::ROOF).hipped(t.x, t.y, 5.0, 0.2));
    super::window(k, tc.x, tc.y - t.y - 0.03, 0.0, 1.2, 2.4, ids::TRIM);
}
