//! Medieval Walled City: a cobbled central plaza with a well, organic
//! radial lanes and ring lanes, a perimeter curtain wall with round towers
//! and gates (Medieval Architect masonry), and densely packed burgher
//! townhouses fronting the lanes plus Voronoi-style infill.

use super::layout::{Lot, Plan};
use super::{props, townhouse};
use crate::generators::kit::ids;
use crate::modifiers::medieval::parts::{self, Masonry};
use bevy::math::Vec2;
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub fn build(plan: &mut Plan) {
    let p = plan.p.clone();
    let sw = p.street_width.max(2.0);
    let c = (plan.min + plan.max) * 0.5;
    let big_r = (plan.max - plan.min).min_element() * 0.5 * 0.9;
    plan.ground(6.0);
    let rp = (big_r * 0.16).max(8.0);
    plan.kit.mat(ids::ROAD).frustum(c.extend(0.03), rp, rp, 0.005, 24, false);
    plan.used.push(Lot::new(c, Vec2::splat(rp * 0.95), 0.0));
    props::well(&mut plan.kit, c.extend(0.03));
    // Linden trees ringing the plaza.
    let n = (p.prop_density.clamp(0.0, 1.0) * 8.0).round() as usize;
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU + 0.4;
        let at = c + Vec2::new(a.cos(), a.sin()) * rp * 0.75;
        let (kit, r) = (&mut plan.kit, &mut plan.rng);
        props::random_tree(kit, at.extend(0.03), r, 0.8);
    }
    // Radial lanes (wiggly spokes) and two ring lanes.
    let k = plan.rng.gen_range(6..=8);
    let phase = plan.rng.gen_range(0.0..TAU);
    let angles: Vec<f32> = (0..k).map(|i| phase + TAU * (i as f32 + plan.rng.gen_range(-0.2..0.2)) / k as f32).collect();
    let mut lanes = vec![];
    for &a in &angles {
        let dir = Vec2::from_angle(a);
        let pts: Vec<Vec2> = (0..=6)
            .map(|i| {
                let t = i as f32 / 6.0;
                let wob = if i == 0 || i == 6 { 0.0 } else { plan.rng.gen_range(-1.0..1.0) * big_r * 0.03 };
                c + dir * (rp + (big_r - rp) * t) + dir.perp() * wob
            })
            .collect();
        plan.road_path(&pts, sw, 0.03, ids::ROAD);
        lanes.push(pts);
    }
    for frac in [0.45, 0.72] {
        let n = 40;
        let wob = plan.rng.gen_range(0.0..TAU);
        let pts: Vec<Vec2> = (0..=n)
            .map(|i| {
                let a = TAU * i as f32 / n as f32;
                c + Vec2::from_angle(a) * big_r * frac * (1.0 + 0.05 * (3.0 * a + wob).sin())
            })
            .collect();
        plan.road_path(&pts, sw * 0.9, 0.035, ids::ROAD);
        lanes.push(pts);
    }
    curtain_wall(plan, c, big_r, &angles[..angles.len().min(4)]);
    // Townhouses fronting every lane, then infill facing the plaza.
    let depth = p.block_size.clamp(5.0, 30.0);
    for pts in &lanes {
        let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
        let mut s = 3.0;
        while s < total {
            let w = plan.rng.gen_range(5.5..8.0);
            let (at, dir) = super::suburb::sample(pts, s + w * 0.5);
            for side in [-1.0f32, 1.0] {
                let n = dir.perp() * side;
                let lot = Lot::new(at + n * (sw * 0.5 + depth * 0.5 + 0.2), Vec2::new(w * 0.5, depth * 0.5), n.y.atan2(n.x) - FRAC_PI_2);
                place(plan, lot);
            }
            s += w;
        }
    }
    for _ in 0..(big_r * big_r * 0.06) as usize {
        let at = c + Vec2::from_angle(plan.rng.gen_range(0.0..TAU)) * plan.rng.gen_range(rp..big_r - 8.0);
        let n = (c - at).normalize_or_zero();
        let lot = Lot::new(at, Vec2::new(plan.rng.gen_range(2.8..4.0), depth * 0.5), n.y.atan2(n.x) + FRAC_PI_2);
        place(plan, lot);
    }
}

fn place(plan: &mut Plan, lot: Lot) {
    if !plan.rng.gen_bool(plan.p.density.clamp(0.0, 1.0) as f64) || !plan.claim(lot) {
        return;
    }
    let lo = plan.p.min_floors.max(1);
    let floors = plan.rng.gen_range(lo..=plan.p.max_floors.max(lo));
    let pitch = plan.p.roof_pitch;
    let (kit, r) = (&mut plan.kit, &mut plan.rng);
    kit.at(lot.frame(0.0), |k| {
        townhouse::townhouse(k, lot.half, floors, pitch, r);
    });
}

/// Perimeter curtain wall with round towers; gaps (gatehouses) where the
/// first spokes reach the wall.
fn curtain_wall(plan: &mut Plan, c: Vec2, big_r: f32, gates: &[f32]) {
    let n = ((TAU * big_r / 14.0).round() as usize).max(8);
    let m = Masonry { bricks: false, ..Masonry::default() };
    let (h, t) = (9.0, 2.4);
    let at = |i: usize| c + Vec2::from_angle(TAU * i as f32 / n as f32) * big_r;
    for i in 0..n {
        let (a, b) = (at(i), at(i + 1));
        let mid_a = TAU * (i as f32 + 0.5) / n as f32;
        let gate = gates.iter().any(|g| ((mid_a - g + PI).rem_euclid(TAU) - PI).abs() < PI / n as f32);
        if !plan.contains(a) || !plan.contains(b) {
            continue;
        }
        plan.used.push(Lot::new((a + b) * 0.5, Vec2::new(a.distance(b) * 0.5, t * 0.5 + 1.0), (b - a).y.atan2((b - a).x)));
        if gate {
            parts::gatehouse(&mut plan.kit, a, b, h, t, &m);
        } else {
            // Wall runs clockwise so its outer face points away from the town.
            parts::wall_segment(&mut plan.kit, b, a, 0.0, h, t, &m, true);
        }
        if i % 3 == 0 {
            parts::round_tower(&mut plan.kit, a.extend(0.0), t * 1.5, h + 4.0, &m, true);
        }
    }
}
