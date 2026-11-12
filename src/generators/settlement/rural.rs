//! PR 6 rural presets: Crossroads Hamlet and Monastery.

use super::layout::{Lot, Plan};
use super::{historic, props, townhouse};
use crate::generators::buildings::BuildingKind::{self, *};
use crate::generators::kit::ids;
use crate::modifiers::medieval::parts::{self, Masonry};
use bevy::math::Vec2;
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

/// Fenced field with crop rows (Farm Plots toggle).
fn field(plan: &mut Plan, lot: Lot) {
    if !plan.p.features.farms || !plan.claim(lot) {
        return;
    }
    let hill = plan.hill;
    let corners = lot.corners();
    plan.kit.at(lot.frame(0.0), |k| props::crop_rows(k, lot.half.x, lot.half.y, 1.6));
    props::fence_loop(&mut plan.kit, &corners, move |q| super::layout::hill_height(hill, q));
}

fn cottage(plan: &mut Plan, lot: Lot) {
    if !plan.claim(lot) {
        return;
    }
    let pitch = plan.p.roof_pitch;
    let (kit, r) = (&mut plan.kit, &mut plan.rng);
    kit.at(lot.frame(0.0), |k| townhouse::cottage(k, lot.half, pitch, r));
}

/// Lot of size `half` beside a street `a`-`b` at `t` along it, facing it.
fn beside(a: Vec2, b: Vec2, t: f32, side: f32, sw: f32, half: Vec2) -> Lot {
    let dir = (b - a).normalize();
    let n = dir.perp() * side;
    Lot::new(a.lerp(b, t) + n * (sw * 0.5 + half.y + 1.0), half, n.y.atan2(n.x) - FRAC_PI_2)
}

pub fn crossroads(plan: &mut Plan) {
    let (sw, size, c) = (plan.p.street_width.max(2.0), plan.max - plan.min, (plan.min + plan.max) * 0.5);
    plan.ground(6.0);
    // Two dirt roads crossing at a slight angle.
    let (ew, ns) = ((Vec2::new(plan.min.x, c.y - 3.0), Vec2::new(plan.max.x, c.y + 3.0)), (Vec2::new(c.x + 4.0, plan.min.y), Vec2::new(c.x - 4.0, plan.max.y)));
    plan.road(ew.0, ew.1, sw, 0.04, ids::ROAD);
    plan.road(ns.0, ns.1, sw, 0.04, ids::ROAD);
    if plan.p.features.market {
        props::well(&mut plan.kit, (c + Vec2::new(sw + 2.0, sw + 2.0)).extend(0.0));
        plan.used.push(Lot::new(c + Vec2::new(sw + 2.0, sw + 2.0), Vec2::splat(1.5), 0.0));
    }
    // The four corner plots get the inn, smithy, stable and well house.
    let kinds: Vec<BuildingKind> = [Tavern, Forge, Stable, WellHouse, Granary].into_iter().filter(|k| plan.allowed(*k)).collect();
    let spots = [(ew, 0.42, -1.0), (ew, 0.58, 1.0), (ns, 0.42, 1.0), (ns, 0.58, -1.0), (ew, 0.3, 1.0)];
    for (k, ((a, b), t, s)) in kinds.into_iter().zip(spots) {
        let (w, l, _) = k.defaults();
        let lot = beside(a, b, t, s, sw, Vec2::new(w, l) * 0.5);
        let fl = k.has_floors().then(|| plan.floors());
        plan.building(lot, k, fl);
    }
    // Cottages strung along all four arms, fields behind them.
    for (a, b) in [ew, ns] {
        for side in [-1.0, 1.0] {
            let mut t: f32 = 0.05;
            while t < 0.95 {
                if (t - 0.5).abs() > 0.14 {
                    let half = Vec2::new(plan.rng.gen_range(3.0..4.5), plan.rng.gen_range(2.6..3.6));
                    if plan.rng.gen_bool(plan.p.density.clamp(0.0, 1.0) as f64) {
                        cottage(plan, beside(a, b, t, side, sw, half));
                    }
                    let fh = Vec2::new(plan.rng.gen_range(8.0..14.0), plan.rng.gen_range(6.0..10.0));
                    field(plan, beside(a, b, t, side, sw + half.y * 4.0 + 6.0, fh));
                }
                t += plan.rng.gen_range(0.07..0.12);
            }
        }
    }
    if plan.allowed(Windmill) {
        let at = c + Vec2::new(size.x * 0.3, -size.y * 0.3);
        plan.building(Lot::new(at, Vec2::splat(4.0), 0.3), Windmill, None);
    }
    historic::trees(plan, 450.0);
}

pub fn monastery(plan: &mut Plan) {
    let (sw, size, c) = (plan.p.street_width.max(2.0), plan.max - plan.min, (plan.min + plan.max) * 0.5);
    plan.ground(6.0);
    let s = size.min_element() * 0.3;
    let pre = [Vec2::new(-s, -s), Vec2::new(s, -s), Vec2::new(s, s), Vec2::new(-s, s)].map(|q| c + q);
    if plan.p.features.walls {
        let m = Masonry { bricks: false, ..Masonry::default() };
        for i in 0..4 {
            let (a, b) = (pre[i], pre[(i + 1) % 4]);
            let n = ((a.distance(b) / 12.0).round() as usize).max(1);
            for j in 0..n {
                let (p, q) = (a.lerp(b, j as f32 / n as f32), a.lerp(b, (j + 1) as f32 / n as f32));
                // Gate gap in the middle of the south wall.
                if i == 0 && j == n / 2 {
                    continue;
                }
                parts::wall_segment(&mut plan.kit, q, p, 0.0, 4.5, 1.0, &m, false);
            }
            plan.used.push(Lot::new((a + b) * 0.5, Vec2::new(a.distance(b) * 0.5, 1.0), (b - a).y.atan2((b - a).x)));
        }
    }
    plan.road(Vec2::new(c.x, plan.min.y), Vec2::new(c.x, c.y - s * 0.35), sw, 0.04, ids::COBBLE);
    // Cloister garth with an arcade walk, the chapel north of it.
    let g = s * 0.28;
    plan.kit.mat(ids::FOLIAGE).block((c - Vec2::splat(g)).extend(0.0), (c + Vec2::splat(g)).extend(0.04));
    props::well(&mut plan.kit, c.extend(0.04));
    arcade(plan, c, g);
    plan.used.push(Lot::new(c, Vec2::splat(g + 3.0), 0.0));
    let (cw, cl, _) = StoneChapel.defaults();
    plan.building(Lot::new(c + Vec2::new(0.0, g + 3.5 + cw * 0.5), Vec2::new(cl, cw) * 0.5, 0.0), StoneChapel, None);
    let (dw, dl, _) = GuildHall.defaults();
    let fl = plan.floors().min(3);
    plan.building(Lot::new(c + Vec2::new(g + 3.5 + dl * 0.5, 0.0), Vec2::new(dw, dl) * 0.5, FRAC_PI_2), GuildHall, Some(fl));
    for (k, off) in [(Granary, Vec2::new(-g - 9.0, -g * 0.4)), (WellHouse, Vec2::new(-g - 7.0, g * 0.6)), (Stable, Vec2::new(g + 6.0, -s * 0.7))] {
        let (w, l, _) = k.defaults();
        plan.building(Lot::new(c + off, Vec2::new(w, l) * 0.5, -FRAC_PI_2), k, None);
    }
    // Kitchen gardens and an orchard inside the precinct.
    for off in [Vec2::new(-s * 0.6, -s * 0.65), Vec2::new(s * 0.62, s * 0.62)] {
        field(plan, Lot::new(c + off, Vec2::new(s * 0.22, s * 0.14), 0.0));
    }
    if plan.p.features.trees {
        for i in 0..12 {
            let q = c + Vec2::new(-s * 0.75 + (i % 4) as f32 * 5.0, s * 0.45 + (i / 4) as f32 * 5.0);
            if plan.claim(Lot::new(q, Vec2::splat(1.0), 0.0)) {
                props::tree(&mut plan.kit, q.extend(0.0), 4.0, false);
            }
        }
    }
    historic::trees(plan, 1200.0);
}

/// Cloister walk: a lean-to roof on columns around the garth.
fn arcade(plan: &mut Plan, c: Vec2, g: f32) {
    let k = &mut plan.kit;
    let (o, h) = (g + 2.8, 3.2);
    k.mat(ids::COBBLE).block((c - Vec2::splat(o)).extend(0.0), (c + Vec2::splat(o)).extend(0.03));
    k.mat(ids::FACADE);
    let n = ((2.0 * g) / 3.0).round().max(2.0) as usize;
    for i in 0..=n {
        let t = -g + 2.0 * g * i as f32 / n as f32;
        for p in [Vec2::new(t, -g), Vec2::new(t, g), Vec2::new(-g, t), Vec2::new(g, t)] {
            k.block((c + p - Vec2::splat(0.2)).extend(0.0), (c + p + Vec2::splat(0.2)).extend(h));
        }
    }
    k.mat(ids::ROOF);
    for (lo, hi) in [(Vec2::new(-o, -o), Vec2::new(o, -g)), (Vec2::new(-o, g), Vec2::new(o, o)), (Vec2::new(-o, -g), Vec2::new(-g, g)), (Vec2::new(g, -g), Vec2::new(o, g))] {
        k.block((c + lo).extend(h), (c + hi).extend(h + 0.3));
    }
}
