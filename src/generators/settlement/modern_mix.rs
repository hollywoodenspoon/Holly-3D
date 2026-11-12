//! PR 6 modern presets assembled from Buildings & Props pieces: River City,
//! Harbour Town, Industrial District and Old Town + Modern Sprawl.

use super::downtown::grid_lines;
use super::layout::{Lot, Plan};
use super::{Era, props};
use crate::generators::buildings::BuildingKind::{self, *};
use crate::generators::kit::ids;
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::TAU;

/// A street grid over `lo..hi` (asphalt with lane lines); returns the
/// street centrelines across X and Y.
pub(super) fn grid(plan: &mut Plan, lo: Vec2, hi: Vec2, sw: f32, pitch: f32, id: u16) -> (Vec<f32>, Vec<f32>) {
    let (xs, ys) = (grid_lines(lo.x, hi.x, pitch), grid_lines(lo.y, hi.y, pitch));
    for &x in &xs {
        plan.road(Vec2::new(x, lo.y), Vec2::new(x, hi.y), sw, 0.03, id);
    }
    for &y in &ys {
        plan.road(Vec2::new(lo.x, y), Vec2::new(hi.x, y), sw, 0.03, id);
    }
    if id == ids::ASPHALT {
        plan.kit.mat(ids::LANE);
        for &x in &xs {
            let mut y = lo.y;
            while y < hi.y {
                let y1 = (y + 3.0).min(hi.y);
                if plan.contains(Vec2::new(x, (y + y1) * 0.5)) {
                    plan.kit.block(Vec3::new(x - 0.08, y, 0.035), Vec3::new(x + 0.08, y1, 0.04));
                }
                y += 4.5;
            }
        }
    }
    (xs, ys)
}

/// Frontage along every grid street between its crossings.
pub(super) fn grid_frontage(plan: &mut Plan, xs: &[f32], ys: &[f32], sw: f32, kinds: &[BuildingKind], native: f64) {
    let depth = (plan.p.block_size * 0.5 - 1.0).max(5.0);
    let m = sw * 0.5 + 0.5;
    for &x in xs {
        for w in ys.windows(2) {
            plan.frontage(Vec2::new(x, w[0] + m), Vec2::new(x, w[1] - m), sw, depth, kinds, native);
        }
    }
    for &y in ys {
        for w in xs.windows(2) {
            plan.frontage(Vec2::new(w[0] + m, y), Vec2::new(w[1] - m, y), sw, depth, kinds, native);
        }
    }
}

fn pitch(plan: &Plan) -> f32 {
    plan.p.block_size.max(16.0) + plan.p.street_width.max(3.0)
}

pub fn river_city(plan: &mut Plan) {
    let (sw, size, c) = (plan.p.street_width.max(3.0), plan.max - plan.min, (plan.min + plan.max) * 0.5);
    let f = plan.p.features;
    let rw = (size.x * 0.13).clamp(12.0, 60.0);
    let river = f.river.then(|| plan.river(Vec2::new(c.x, plan.min.y - 4.0), Vec2::new(c.x, plan.max.y + 4.0), rw, true));
    plan.ground(8.0);
    if let (Some(_), true) = (river, f.docks) {
        // Quayside promenades with lamps and trees along both banks.
        for s in [-1.0f32, 1.0] {
            let x = c.x + s * (rw * 0.5 + 3.4);
            let quay = Lot::new(Vec2::new(x, c.y), Vec2::new(2.2, size.y * 0.5), 0.0);
            plan.kit.mat(ids::SIDEWALK).block(Vec3::new(x - 2.2, plan.min.y, 0.0), Vec3::new(x + 2.2, plan.max.y, 0.12));
            plan.used.push(quay);
            let mut y = plan.min.y + 6.0;
            while y < plan.max.y - 4.0 {
                props::streetlamp(&mut plan.kit, Vec3::new(x - s * 1.8, y, 0.12), Vec2::new(-s, 0.0));
                if f.trees && y + 9.0 < plan.max.y - 4.0 {
                    props::tree(&mut plan.kit, Vec3::new(x + s * 0.6, y + 9.0, 0.12), 6.0, false);
                }
                y += 18.0;
            }
        }
    }
    let (xs, ys) = grid(plan, plan.min, plan.max, sw, pitch(plan), ids::ASPHALT);
    if let Some(r) = river {
        for (i, &y) in ys.iter().enumerate() {
            let kind = if i % 2 == 0 { TrussBridge } else { ArchBridge };
            plan.bridge(r, Vec2::new(c.x, y), sw + 2.0, kind);
        }
    }
    grid_frontage(plan, &xs, &ys, sw, Era::RiverCity.palette(), 0.0);
    super::historic::trees(plan, 1400.0);
}

pub fn harbour(plan: &mut Plan) {
    let (sw, size) = (plan.p.street_width.max(3.0), plan.max - plan.min);
    let f = plan.p.features;
    let sea = size.y * 0.26;
    let shore = plan.min.y + sea;
    let water = f.river.then(|| plan.river(Vec2::new(plan.min.x - 4.0, plan.min.y + sea * 0.5 - 2.0), Vec2::new(plan.max.x + 4.0, plan.min.y + sea * 0.5 - 2.0), sea + 4.0, false));
    plan.ground(8.0);
    let apron = 12.0;
    if water.is_some() {
        // Quay wall, a paved apron with crane rails, and timber piers.
        plan.kit.mat(ids::FACADE).block(Vec3::new(plan.min.x, shore - 1.5, -3.2), Vec3::new(plan.max.x, shore, 0.1));
        plan.kit.mat(ids::SIDEWALK).block(Vec3::new(plan.min.x, shore, 0.0), Vec3::new(plan.max.x, shore + apron, 0.1));
        plan.used.push(Lot::new(Vec2::new((plan.min.x + plan.max.x) * 0.5, shore + apron * 0.5), Vec2::new(size.x * 0.5, apron * 0.5), 0.0));
        plan.kit.mat(ids::IRON);
        for y in [shore + 2.0, shore + 9.0] {
            plan.kit.block(Vec3::new(plan.min.x, y - 0.1, 0.1), Vec3::new(plan.max.x, y + 0.1, 0.22));
        }
        let mut x = plan.min.x + 16.0;
        while x < plan.max.x - 10.0 {
            if f.docks {
                pier(plan, Vec2::new(x, shore - 1.5), (sea * 0.6).min(30.0));
            }
            if plan.allowed(Crane) {
                plan.piece(Lot::new(Vec2::new(x + 10.0, shore + 5.5), Vec2::new(3.5, 4.0), 0.0), Crane, Some(4), None);
            }
            x += 34.0;
        }
    }
    let y0 = if water.is_some() { shore + apron + sw * 0.5 } else { plan.min.y + sw };
    plan.road(Vec2::new(plan.min.x, y0), Vec2::new(plan.max.x, y0), sw, 0.03, ids::ASPHALT);
    plan.frontage(Vec2::new(plan.min.x + 2.0, y0), Vec2::new(plan.max.x - 2.0, y0), sw, 24.0, &[Warehouse], 0.0);
    let lo = Vec2::new(plan.min.x, y0 + 26.0);
    let (xs, ys) = grid(plan, lo, plan.max, sw, pitch(plan), ids::ASPHALT);
    let town: Vec<BuildingKind> = Era::HarbourTown.palette().iter().copied().filter(|k| !matches!(k, Warehouse | Crane)).collect();
    grid_frontage(plan, &xs, &ys, sw, &town, 0.0);
    super::historic::trees(plan, 1400.0);
}

/// A timber pier on piles running out to sea (-Y) from `root`.
fn pier(plan: &mut Plan, root: Vec2, len: f32) {
    let w = 3.0;
    plan.kit.mat(ids::TRIM).block(Vec3::new(root.x - w * 0.5, root.y - len, -0.3), Vec3::new(root.x + w * 0.5, root.y, 0.0));
    let n = (len / 4.0) as usize;
    for i in 0..=n {
        let y = root.y - len * i as f32 / n.max(1) as f32;
        for x in [root.x - w * 0.5 + 0.2, root.x + w * 0.5 - 0.2] {
            plan.kit.frustum(Vec3::new(x, y, super::compose::WATER_Z - 1.0), 0.16, 0.16, -super::compose::WATER_Z + 0.7, 6, true);
        }
    }
}

pub fn industrial(plan: &mut Plan) {
    let (sw, size) = (plan.p.street_width.max(3.0), plan.max - plan.min);
    plan.ground(10.0);
    let mut hi = plan.max;
    if plan.p.features.rail {
        let band = 24.0f32.min(size.y * 0.3);
        rail_yard(plan, plan.max.y - band, band);
        hi.y -= band + 2.0;
    }
    let (xs, ys) = grid(plan, plan.min, hi, sw, pitch(plan), ids::ASPHALT);
    grid_frontage(plan, &xs, &ys, sw, Era::Industrial.palette(), 0.0);
    // Storage tanks and brick chimneys in the leftover yard space.
    for _ in 0..((size.x * size.y) / 900.0) as usize {
        let at = plan.min + size * Vec2::new(plan.rng.gen_range(0.05..0.95), plan.rng.gen_range(0.05..0.95));
        let tank = plan.rng.gen_bool(0.6);
        let r = if tank { plan.rng.gen_range(3.0..6.0) } else { 1.4 };
        if !plan.claim(Lot::new(at, Vec2::splat(r + 0.5), 0.0)) {
            continue;
        }
        if tank {
            let h = plan.rng.gen_range(5.0..12.0);
            plan.kit.mat(ids::RUST).frustum(at.extend(0.0), r, r, h, 16, true);
            plan.kit.mat(ids::IRON).frustum(at.extend(h), r, r * 0.3, r * 0.35, 16, true);
        } else {
            let h = plan.rng.gen_range(18.0..34.0);
            plan.kit.mat(ids::BRICK).frustum(at.extend(0.0), r, r * 0.7, h, 10, true);
            plan.kit.mat(ids::RUST).frustum(at.extend(h), r * 0.72, r * 0.72, 0.8, 10, true);
        }
    }
    super::historic::trees(plan, 1400.0);
}

/// Four tracks (rails on sleepers) with boxcars along the top edge.
fn rail_yard(plan: &mut Plan, y0: f32, band: f32) {
    let (x0, x1) = (plan.min.x, plan.max.x);
    plan.used.push(Lot::new(Vec2::new((x0 + x1) * 0.5, y0 + band * 0.5), Vec2::new((x1 - x0) * 0.5, band * 0.5), 0.0));
    plan.kit.mat(ids::LOT).block(Vec3::new(x0, y0, 0.0), Vec3::new(x1, y0 + band, 0.12));
    let tracks = ((band / 5.0) as usize).max(1);
    for t in 0..tracks {
        let y = y0 + 2.5 + t as f32 * 5.0;
        plan.kit.mat(ids::TRIM);
        let mut x = x0 + 0.5;
        while x < x1 - 0.5 {
            plan.kit.block(Vec3::new(x - 0.12, y - 1.3, 0.12), Vec3::new(x + 0.12, y + 1.3, 0.24));
            x += 2.4;
        }
        plan.kit.mat(ids::IRON);
        for s in [-0.72, 0.72] {
            plan.kit.block(Vec3::new(x0, y + s - 0.05, 0.24), Vec3::new(x1, y + s + 0.05, 0.36));
        }
        let mut x = x0 + plan.rng.gen_range(4.0..20.0);
        while x < x1 - 16.0 && plan.rng.gen_bool(0.75) {
            let id = if plan.rng.gen_bool(0.5) { ids::RUST } else { ids::IRON };
            plan.kit.mat(id).cuboid(Vec3::new(x + 6.0, y, 2.4), Vec3::new(6.0, 1.4, 1.9), Quat::IDENTITY);
            x += 13.0 + plan.rng.gen_range(0.0..14.0);
        }
    }
}

pub fn old_and_new(plan: &mut Plan) {
    let (sw, size, c) = (plan.p.street_width.max(3.0), plan.max - plan.min, (plan.min + plan.max) * 0.5);
    let r = size.min_element() * 0.22;
    plan.ground(8.0);
    // The old core keeps the modern grid out while it is laid.
    let core = Lot::new(c, Vec2::splat(r + sw * 0.5), 0.0);
    plan.wet.push(core);
    let (xs, ys) = grid(plan, plan.min, plan.max, sw, pitch(plan), ids::ASPHALT);
    let modern: Vec<BuildingKind> = Era::OldTownSprawl.palette().iter().copied().filter(|k| !k.is_medieval()).collect();
    grid_frontage(plan, &xs, &ys, sw, &modern, 0.0);
    plan.wet.pop();
    let ring: Vec<Vec2> = (0..=36).map(|i| c + Vec2::from_angle(TAU * i as f32 / 36.0) * (r + sw * 0.5)).collect();
    plan.road_path(&ring, sw, 0.035, ids::ASPHALT);
    let plaza = (r * 0.3).max(8.0);
    plan.kit.mat(ids::COBBLE).frustum(c.extend(0.03), plaza, plaza, 0.01, 24, false);
    plan.used.push(Lot::new(c, Vec2::splat(plaza * 0.9), 0.0));
    if plan.p.features.market && plan.allowed(MarketHall) {
        plan.piece(Lot::new(c, Vec2::new(5.0, 4.0), 0.2), MarketHall, Some(1), Some(0.04));
    }
    let old: Vec<BuildingKind> = Era::OldTownSprawl.palette().iter().copied().filter(|k| k.is_medieval() && *k != MarketHall).collect();
    let k = 6;
    for i in 0..k {
        let a = TAU * i as f32 / k as f32 + 0.3;
        let (p0, p1) = (c + Vec2::from_angle(a) * plaza, c + Vec2::from_angle(a) * r);
        plan.road(p0, p1, sw * 0.6, 0.04, ids::COBBLE);
        plan.frontage(p0 + (p1 - p0).normalize() * 2.0, p1, sw * 0.6, 12.0, &old, 0.65);
    }
    super::historic::trees(plan, 1400.0);
}
