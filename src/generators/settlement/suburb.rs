//! Modern Suburbs: a curving collector road with cul-de-sac branches ending
//! in turning bulbs, and 1–2 storey L / T-shaped houses with pitched
//! shingle roofs, attached garages, porches, driveways, fenced backyards
//! and yard trees.

use super::layout::{Lot, Plan, sides};
use super::props;
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub fn build(plan: &mut Plan) {
    let p = plan.p.clone();
    let sw = p.street_width.max(3.0);
    plan.ground(6.0);
    let (min, max) = (plan.min, plan.max);
    let amp = (max.y - min.y) * 0.06;
    let spine: Vec<Vec2> = (0..=24)
        .map(|i| {
            let x = min.x + (max.x - min.x) * i as f32 / 24.0;
            Vec2::new(x, (x / (max.x - min.x) * TAU).sin() * amp)
        })
        .collect();
    plan.road_path(&spine, sw, 0.03, ids::ROAD);
    // Cul-de-sacs alternate north / south of the spine.
    let lot_w = (p.block_size * 0.45).clamp(10.0, 40.0);
    let gap = lot_w * 2.4;
    let mut branches = vec![];
    let mut x = min.x + gap * 0.75;
    let mut north = true;
    while x < max.x - gap * 0.5 {
        let y0 = (x / (max.x - min.x) * TAU).sin() * amp;
        let s = if north { 1.0 } else { -1.0 };
        let end_y = if north { max.y - lot_w * 1.2 } else { min.y + lot_w * 1.2 };
        let len = (end_y - y0) * s;
        if len > lot_w * 1.5 {
            let bend = plan.rng.gen_range(-0.15..0.15) * len;
            let pts: Vec<Vec2> = (0..=6)
                .map(|i| {
                    let t = i as f32 / 6.0;
                    Vec2::new(x + bend * (t * PI).sin() * t, y0 + s * len * t)
                })
                .collect();
            plan.road_path(&pts, sw * 0.85, 0.035, ids::ROAD);
            let end = *pts.last().unwrap_or(&Vec2::new(x, y0));
            plan.kit.mat(ids::ROAD).frustum(end.extend(0.035), sw * 1.6, sw * 1.6, 0.005, 16, false);
            plan.used.push(Lot::new(end, Vec2::splat(sw * 1.6), 0.0));
            branches.push(pts);
        }
        x += gap;
        north = !north;
    }
    let depth = (lot_w * 1.3).min(30.0);
    for pts in branches.iter().chain(std::iter::once(&spine)) {
        frontage(plan, pts, sw, lot_w, depth);
    }
}

/// Houses along both sides of a road polyline, facing it.
fn frontage(plan: &mut Plan, pts: &[Vec2], sw: f32, lot_w: f32, depth: f32) {
    let off = sw * 0.5 + plan.p.sidewalk_width + depth * 0.5 + 0.5;
    let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut s = lot_w * 0.5;
    while s < total {
        let (at, dir) = sample(pts, s);
        for side in [-1.0f32, 1.0] {
            let n = dir.perp() * side;
            let c = at + n * off;
            // Local -Y (the front) points back at the road.
            let yaw = n.y.atan2(n.x) - FRAC_PI_2;
            let lot = Lot::new(c, Vec2::new(lot_w * 0.5 - 0.5, depth * 0.5), yaw);
            if plan.rng.gen_bool(plan.p.density.clamp(0.0, 1.0) as f64) && plan.claim(lot) {
                let floors = plan.rng.gen_range(plan.p.min_floors.max(1)..=plan.p.max_floors.max(plan.p.min_floors.max(1)));
                let (pitch, props) = (plan.p.roof_pitch, plan.p.prop_density);
                plan.wrecked(lot, 0.0, false, |k, r| house(k, lot.half, floors, pitch, props, r));
            }
        }
        s += lot_w;
    }
}

/// Point and unit direction at arc length `s` along a polyline.
pub(super) fn sample(pts: &[Vec2], mut s: f32) -> (Vec2, Vec2) {
    for w in pts.windows(2) {
        let l = w[0].distance(w[1]);
        if s <= l || l < 1e-5 {
            return (w[0].lerp(w[1], (s / l.max(1e-5)).min(1.0)), (w[1] - w[0]).normalize_or_zero());
        }
        s -= l;
    }
    let n = pts.len();
    (pts[n - 1], (pts[n - 1] - pts[n.saturating_sub(2)]).normalize_or_zero())
}

/// A detached house filling its lot frame (front yard towards -Y).
fn house(kit: &mut Kit, half: Vec2, floors: u32, pitch: f32, props_d: f32, r: &mut ChaCha8Rng) {
    let fh = super::Era::Suburbs.floor_height();
    let h = floors as f32 * fh;
    let wall = if r.gen_bool(0.5) { ids::TRIM } else { ids::FACADE };
    let main = Vec2::new((half.x * r.gen_range(0.45..0.6)).max(4.0), (half.y * 0.3).max(3.5));
    let mc = Vec2::new(-half.x * 0.12, -half.y * 0.1);
    let rise = |hy: f32| (pitch.clamp(5.0, 70.0)).to_radians().tan() * hy;
    // Main wing, then a wing that makes the L (or the T) shape.
    kit.mat(wall).block((mc - main).extend(0.0), (mc + main).extend(h));
    kit.at(Mat4::from_translation(mc.extend(h)), |k| k.mat(ids::ROOF).gable(main.x, main.y, rise(main.y), 0.4, wall));
    let wing = Vec2::new(main.x * 0.4, main.y * 0.75);
    let wx = if r.gen_bool(0.5) { mc.x + main.x - wing.x } else { mc.x };
    let wc = Vec2::new(wx, mc.y + main.y + wing.y);
    kit.mat(wall).block((wc - wing).extend(0.0), (wc + wing).extend(h));
    kit.at(Mat4::from_translation(wc.extend(h)) * Mat4::from_rotation_z(FRAC_PI_2), |k| {
        k.mat(ids::ROOF).gable(wing.y, wing.x, rise(wing.x), 0.3, wall)
    });
    // Windows on the main wing; one lit room now and then.
    kit.at(Mat4::from_translation(mc.extend(0.0)), |k| {
        sides(k, main.x, main.y, |k, side, w| {
            let n = ((w / 3.2).floor() as usize).max(1);
            for f in 0..floors {
                for i in 0..n {
                    if side == 0 && f == 0 && i == n / 2 {
                        continue; // front door bay
                    }
                    let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
                    super::window(k, x, 0.0, f as f32 * fh + 0.9, 1.1, 1.3, if r.gen_bool(0.15) { ids::LIT } else { ids::GLASS });
                }
            }
            if side == 0 {
                super::window(k, 0.0, 0.0, 0.0, 1.0, 2.1, ids::TRIM);
            }
        });
    });
    // Attached garage beside the main wing with its door and driveway.
    let gx = mc.x + main.x + 3.2;
    if gx + 3.0 < half.x {
        let g = Vec2::new(3.0, 3.4);
        let gc = Vec2::new(gx, mc.y - main.y + g.y);
        kit.mat(wall).block((gc - g).extend(0.0), (gc + g).extend(2.8));
        kit.at(Mat4::from_translation(gc.extend(2.8)), |k| k.mat(ids::ROOF).gable(g.x, g.y, rise(g.y) * 0.6, 0.25, wall));
        super::window(kit, gc.x, gc.y - g.y - 0.03, 0.0, 4.6, 2.3, ids::PLASTER);
        kit.mat(ids::LOT).block(Vec3::new(gc.x - 2.6, -half.y - 1.5, 0.0), Vec3::new(gc.x + 2.6, gc.y - g.y, 0.05));
    }
    // Porch: slab, two posts and a lean-to roof.
    let pz = mc.y - main.y;
    kit.mat(ids::FACADE).block(Vec3::new(mc.x - 2.2, pz - 2.0, 0.0), Vec3::new(mc.x + 2.2, pz, 0.3));
    kit.mat(ids::TRIM);
    for s in [-1.9, 1.9] {
        kit.block(Vec3::new(mc.x + s - 0.1, pz - 1.9, 0.3), Vec3::new(mc.x + s + 0.1, pz - 1.7, 2.6));
    }
    kit.mat(ids::ROOF).quad(
        Vec3::new(mc.x - 2.4, pz - 2.2, 2.6),
        Vec3::new(mc.x + 2.4, pz - 2.2, 2.6),
        Vec3::new(mc.x + 2.4, pz, 3.1),
        Vec3::new(mc.x - 2.4, pz, 3.1),
    );
    // Fenced backyard and yard trees.
    let back = mc.y + main.y + 0.5;
    if back + 3.0 < half.y {
        let yard = [
            Vec2::new(-half.x + 0.3, back),
            Vec2::new(-half.x + 0.3, half.y - 0.3),
            Vec2::new(half.x - 0.3, half.y - 0.3),
            Vec2::new(half.x - 0.3, back),
        ];
        for w in yard.windows(2) {
            props::fence(kit, w[0], w[1], |_| 0.0);
        }
    }
    if r.gen_bool(props_d.clamp(0.0, 1.0) as f64) {
        props::random_tree(kit, Vec3::new(-half.x * 0.7, -half.y * 0.7, 0.0), r, 0.8);
    }
    if r.gen_bool((props_d * 0.7).clamp(0.0, 1.0) as f64) {
        props::random_tree(kit, Vec3::new(half.x * 0.5, half.y * 0.75, 0.0), r, 1.0);
    }
}
