//! PR 6 historic presets: Square Walled City, River Town, Castle & Town and
//! Star Fort, mixing burgher townhouses with Buildings & Props pieces.

use super::layout::{Lot, Plan};
use super::props;
use crate::generators::buildings::BuildingKind::{self, *};
use crate::generators::kit::ids;
use crate::modifiers::medieval::parts::{self, Masonry};
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// A curtain wall around the counter-clockwise loop `pts`, split into
/// ~14 m panels, a gatehouse in the middle of each edge listed in
/// `gates`, and round towers on the corners.
pub(super) fn wall_loop(plan: &mut Plan, pts: &[Vec2], h: f32, t: f32, gates: &[usize]) {
    let m = Masonry { bricks: false, ..Masonry::default() };
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let n = ((a.distance(b) / 14.0).round() as usize).max(1);
        for j in 0..n {
            let (p, q) = (a.lerp(b, j as f32 / n as f32), a.lerp(b, (j + 1) as f32 / n as f32));
            if !plan.dry(p) || !plan.dry(q) {
                continue;
            }
            plan.used.push(Lot::new((p + q) * 0.5, Vec2::new(p.distance(q) * 0.5, t * 0.5 + 1.0), (q - p).y.atan2((q - p).x)));
            if gates.contains(&i) && j == n / 2 {
                parts::gatehouse(&mut plan.kit, p, q, h, t, &m);
            } else {
                parts::wall_segment(&mut plan.kit, q, p, 0.0, h, t, &m, true);
            }
        }
        if plan.dry(a) {
            parts::round_tower(&mut plan.kit, a.extend(0.0), t * 1.5, h + 4.0, &m, true);
        }
    }
}

fn medieval(plan: &Plan, drop: &[BuildingKind]) -> Vec<BuildingKind> {
    plan.p.era.palette().iter().copied().filter(|k| !drop.contains(k)).collect()
}

pub fn square_walled(plan: &mut Plan) {
    let (sw, c) = (plan.p.street_width.max(2.0), (plan.min + plan.max) * 0.5);
    let s = (plan.max - plan.min).min_element() * 0.45;
    plan.ground(6.0);
    let corners = [Vec2::new(-s, -s), Vec2::new(s, -s), Vec2::new(s, s), Vec2::new(-s, s)].map(|q| c + q);
    if plan.p.features.walls {
        wall_loop(plan, &corners, 9.0, 2.4, &[0, 1, 2, 3]);
    }
    // Cross streets through the gates, a lane grid and the market square.
    let inner = s - 4.0;
    for (a, b) in [(Vec2::new(-s - 6.0, 0.0), Vec2::new(s + 6.0, 0.0)), (Vec2::new(0.0, -s - 6.0), Vec2::new(0.0, s + 6.0))] {
        plan.road(c + a, c + b, sw * 1.3, 0.04, ids::COBBLE);
    }
    let sq = (s * 0.22).max(8.0);
    if plan.p.features.market {
        plan.kit.mat(ids::COBBLE).block((c - Vec2::splat(sq)).extend(0.0), (c + Vec2::splat(sq)).extend(0.05));
        plan.used.push(Lot::new(c, Vec2::splat(sq), 0.0));
        if plan.allowed(MarketHall) {
            plan.piece(Lot::new(c + Vec2::new(0.0, sq * 0.4), Vec2::new(6.0, 4.0), 0.0), MarketHall, Some(1), Some(0.05));
        }
        props::well(&mut plan.kit, (c - Vec2::new(0.0, sq * 0.5)).extend(0.05));
        if plan.p.features.trees {
            for q in [Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)] {
                props::tree(&mut plan.kit, (c + q * (sq - 1.5)).extend(0.05), 5.5, false);
            }
        }
    }
    let lane = plan.p.block_size.max(8.0) * 2.0 + sw;
    let lines = super::downtown::grid_lines(c.x - inner, c.x + inner, lane);
    let kinds = medieval(plan, &[MarketHall]);
    for &o in &lines {
        let (x, y) = (o, o - c.x + c.y);
        plan.road(Vec2::new(x, c.y - inner), Vec2::new(x, c.y + inner), sw, 0.04, ids::ROAD);
        plan.road(Vec2::new(c.x - inner, y), Vec2::new(c.x + inner, y), sw, 0.04, ids::ROAD);
    }
    for &o in &lines {
        let (x, y) = (o, o - c.x + c.y);
        plan.frontage(Vec2::new(x, c.y - inner), Vec2::new(x, c.y + inner), sw, plan.p.block_size, &kinds, 0.75);
        plan.frontage(Vec2::new(c.x - inner, y), Vec2::new(c.x + inner, y), sw, plan.p.block_size, &kinds, 0.75);
    }
    trees(plan, 600.0);
}

pub fn river_town(plan: &mut Plan) {
    let (sw, size, c) = (plan.p.street_width.max(2.0), plan.max - plan.min, (plan.min + plan.max) * 0.5);
    let f = plan.p.features;
    let rw = (size.y * 0.1).clamp(8.0, 30.0);
    let river = f.river.then(|| plan.river(Vec2::new(plan.min.x - 4.0, c.y), Vec2::new(plan.max.x + 4.0, c.y), rw, true));
    plan.ground(6.0);
    // Bank lanes on both sides, a high street across the river.
    let bank = rw * 0.5 + 1.5 + sw * 0.5;
    for s in [-1.0, 1.0] {
        plan.road(Vec2::new(plan.min.x, c.y + s * bank), Vec2::new(plan.max.x, c.y + s * bank), sw, 0.04, ids::COBBLE);
    }
    let xs = [c.x - size.x * 0.22, c.x + size.x * 0.18];
    for &x in &xs {
        plan.road(Vec2::new(x, plan.min.y), Vec2::new(x, plan.max.y), sw, 0.04, ids::ROAD);
    }
    if let Some(r) = river {
        for &x in &xs {
            plan.bridge(r, Vec2::new(x, c.y), sw + 1.5, ArchBridge);
        }
        // Watermills and timber landing stages on the banks.
        let mut x = plan.min.x + 12.0;
        while x < plan.max.x - 12.0 {
            let s = if plan.rng.gen_bool(0.5) { 1.0 } else { -1.0 };
            if xs.iter().all(|b| (x - b).abs() > 16.0) {
                if plan.rng.gen_bool(0.4) {
                    let lot = Lot::new(Vec2::new(x, c.y + s * (rw * 0.5 + 6.5)), Vec2::new(6.0, 5.0), if s > 0.0 { PI } else { 0.0 });
                    plan.building(lot, Watermill, Some(2));
                } else if f.docks {
                    let y = c.y + s * (rw * 0.5 - 1.0);
                    plan.kit.mat(ids::TRIM).block(Vec3::new(x - 3.0, y - 1.6, -0.3), Vec3::new(x + 3.0, y + 1.6, 0.0));
                }
            }
            x += plan.rng.gen_range(18.0..30.0);
        }
    }
    let kinds = medieval(plan, &[Watermill, ArchBridge]);
    for s in [-1.0, 1.0] {
        plan.frontage(Vec2::new(plan.min.x + 2.0, c.y + s * bank), Vec2::new(plan.max.x - 2.0, c.y + s * bank), sw, 14.0, &kinds, 0.6);
    }
    for &x in &xs {
        plan.frontage(Vec2::new(x, plan.min.y + 2.0), Vec2::new(x, plan.max.y - 2.0), sw, 14.0, &kinds, 0.6);
    }
    trees(plan, 900.0);
}

pub fn castle_town(plan: &mut Plan) {
    let (sw, size, c) = (plan.p.street_width.max(2.0), plan.max - plan.min, (plan.min + plan.max) * 0.5);
    let hill_c = c + Vec2::new(0.0, size.y * 0.2);
    let hr = size.min_element() * 0.3;
    plan.hill = Some((hill_c, hr, (hr * 0.32).min(22.0)));
    plan.ground(5.0);
    let top = plan.h(hill_c);
    if plan.allowed(CastleKeep) {
        let keep = Lot::new(hill_c, Vec2::splat((hr * 0.18).clamp(6.0, 14.0)), 0.0);
        plan.used.push(keep);
        plan.piece(keep, CastleKeep, Some(4), Some(top - 0.5));
    }
    if plan.p.features.walls {
        let m = Masonry { bricks: false, ..Masonry::default() };
        let (rr, n) = (hr * 0.36, 10);
        for i in 0..n {
            let (a, b) = (hill_c + Vec2::from_angle(TAU * i as f32 / n as f32) * rr, hill_c + Vec2::from_angle(TAU * (i + 1) as f32 / n as f32) * rr);
            let z = plan.h(a).min(plan.h(b)) - 0.5;
            if i != 7 {
                parts::wall_segment(&mut plan.kit, b, a, z, 6.0, 2.0, &m, true);
            }
            parts::round_tower(&mut plan.kit, a.extend(z), 2.6, 10.0, &m, true);
            plan.used.push(Lot::new((a + b) * 0.5, Vec2::new(a.distance(b) * 0.5, 2.0), (b - a).y.atan2((b - a).x)));
        }
    }
    // A winding castle road from the town square up to the gate.
    let square = c - Vec2::new(0.0, size.y * 0.22);
    let gate = hill_c + Vec2::from_angle(TAU * 7.5 / 10.0) * hr * 0.4;
    let up: Vec<Vec2> = (0..=16).map(|i| {
        let t = i as f32 / 16.0;
        square.lerp(gate, t) + Vec2::new((t * PI * 2.0).sin() * hr * 0.15, 0.0)
    }).collect();
    plan.road_path(&up, sw, 0.05, ids::COBBLE);
    plan.road(Vec2::new(plan.min.x, square.y), Vec2::new(plan.max.x, square.y), sw, 0.04, ids::COBBLE);
    if plan.p.features.market && plan.allowed(MarketHall) {
        plan.building(Lot::new(square + Vec2::new(0.0, -sw * 0.5 - 5.0), Vec2::new(5.5, 4.0), 0.0), MarketHall, Some(1));
    }
    let kinds = medieval(plan, &[CastleKeep, MarketHall]);
    plan.frontage(Vec2::new(plan.min.x + 2.0, square.y), Vec2::new(plan.max.x - 2.0, square.y), sw, 14.0, &kinds, 0.7);
    for x in [square.x - size.x * 0.25, square.x + size.x * 0.25] {
        plan.road(Vec2::new(x, plan.min.y), Vec2::new(x, square.y), sw, 0.04, ids::ROAD);
        plan.frontage(Vec2::new(x, plan.min.y + 2.0), Vec2::new(x, square.y - sw), sw, 14.0, &kinds, 0.7);
    }
    trees(plan, 700.0);
}

pub fn star_fort(plan: &mut Plan) {
    let (sw, c) = (plan.p.street_width.max(2.0), (plan.min + plan.max) * 0.5);
    let f = plan.p.features;
    let r = (plan.max - plan.min).min_element() * 0.36;
    // Five bastions: outer points at r, curtain re-entrants at 0.7 r.
    let star: Vec<Vec2> = (0..10).map(|i| c + Vec2::from_angle(TAU * i as f32 / 10.0 + FRAC_PI_2) * if i % 2 == 0 { r } else { r * 0.7 }).collect();
    let moat = (r * 0.12).clamp(5.0, 14.0);
    if f.moat {
        for i in 0..10 {
            let (a, b) = (star[i], star[(i + 1) % 10]);
            let out = (b - a).perp().normalize() * -(moat * 0.5 + 2.0);
            let ext = (b - a).normalize() * moat * 0.6;
            plan.river(a + out - ext, b + out + ext, moat, false);
        }
    }
    plan.ground(6.0);
    if f.walls {
        wall_loop(plan, &star, 7.0, 3.0, &[]);
    }
    // Gate and drawbridge on the south curtain (edge 4-5 faces -Y).
    let (ga, gb) = (star[4], star[5]);
    let mid = (ga + gb) * 0.5;
    let out = (gb - ga).perp().normalize() * -1.0;
    if plan.allowed(Gatehouse) {
        let yaw = out.y.atan2(out.x) + FRAC_PI_2;
        plan.piece(Lot::new(mid - out * 1.5, Vec2::new(7.0, 4.0), yaw), Gatehouse, Some(2), Some(0.0));
    }
    if f.moat && plan.allowed(Drawbridge) && f.bridges {
        let yaw = out.y.atan2(out.x) + FRAC_PI_2;
        plan.piece(Lot::new(mid + out * (moat * 0.5 + 3.0), Vec2::new(2.5, moat * 0.5 + 4.0), yaw), Drawbridge, None, Some(-super::compose::WATER_Z.abs() - 3.5));
    }
    plan.road(mid - out * 4.0, c, sw, 0.04, ids::COBBLE);
    // Parade ground and barracks ringing it, facing the centre.
    let pg = r * 0.22;
    plan.kit.mat(ids::COBBLE).block((c - Vec2::splat(pg)).extend(0.0), (c + Vec2::splat(pg)).extend(0.05));
    plan.used.push(Lot::new(c, Vec2::splat(pg), 0.0));
    let kinds = medieval(plan, &[Gatehouse, Drawbridge]);
    let n = 9;
    for i in 0..n {
        let a = TAU * (i as f32 + 0.5) / n as f32 + FRAC_PI_2;
        let at = c + Vec2::from_angle(a) * r * 0.44;
        let Some(k) = plan.pick(&kinds) else { break };
        let (w, l, _) = k.defaults();
        let n_in = (c - at).normalize();
        let lot = Lot::new(at, Vec2::new(w.min(16.0), l.min(10.0)) * 0.5, n_in.y.atan2(n_in.x) + FRAC_PI_2);
        let fl = k.has_floors().then(|| plan.floors());
        plan.building(lot, k, fl);
    }
    trees(plan, 500.0);
}

/// Scattered trees (one per `area` square metres of free ground).
pub(super) fn trees(plan: &mut Plan, area: f32) {
    if !plan.p.features.trees {
        return;
    }
    let size = plan.max - plan.min;
    for _ in 0..((size.x * size.y) / area * plan.p.prop_density.clamp(0.0, 1.0)) as usize {
        let q = plan.min + size * Vec2::new(plan.rng.gen_range(0.0..1.0), plan.rng.gen_range(0.0..1.0));
        if plan.claim(Lot::new(q, Vec2::splat(1.2), 0.0)) {
            let z = plan.h(q);
            let (kit, r) = (&mut plan.kit, &mut plan.rng);
            props::random_tree(kit, q.extend(z), r, 1.0);
        }
    }
}
