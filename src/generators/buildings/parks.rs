//! Parks and leisure prefabs, part one: playground, sports court, sports
//! field, stadium and swimming pool. Ground at z = 0, front facing -Y.

use super::industry::fence;
use super::services::tree;
use super::shops::flat_box;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, PI};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Playground => playground(b, hx, hy),
        K::SportsCourt => court(b, hx, hy),
        K::SportsField => field(b, hx, hy),
        K::Stadium => stadium(b, hx, hy),
        K::SwimmingPool => pool(b, hx, hy),
        _ => {}
    }
}

/// White court / pitch markings: a rectangle outline at height `z`.
pub(super) fn lines(k: &mut Kit, min: Vec2, max: Vec2, z: f32, t: f32) {
    k.mat(ids::LANE);
    k.block(Vec3::new(min.x, min.y, z), Vec3::new(max.x, min.y + t, z + 0.01));
    k.block(Vec3::new(min.x, max.y - t, z), Vec3::new(max.x, max.y, z + 0.01));
    k.block(Vec3::new(min.x, min.y, z), Vec3::new(min.x + t, max.y, z + 0.01));
    k.block(Vec3::new(max.x - t, min.y, z), Vec3::new(max.x, max.y, z + 0.01));
}

/// A park bench at `p` facing -Y (turned by `yaw`).
pub(super) fn bench(k: &mut Kit, p: Vec3, yaw: f32) {
    k.at(Mat4::from_translation(p) * Mat4::from_rotation_z(yaw), |k| {
        k.mat(ids::IRON);
        for x in [-0.7, 0.7] {
            k.block(Vec3::new(x - 0.04, -0.2, 0.0), Vec3::new(x + 0.04, 0.25, 0.45));
        }
        k.mat(ids::TRIM).block(Vec3::new(-0.9, -0.22, 0.42), Vec3::new(0.9, 0.22, 0.47));
        k.block(Vec3::new(-0.9, 0.2, 0.5), Vec3::new(0.9, 0.25, 0.85));
    });
}

fn playground(b: &mut Ctx, hx: f32, hy: f32) {
    // Rubber-mulch pad on grass, swings, a slide tower, a see-saw, a
    // climbing dome, a sandpit and benches.
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    k.mat(ids::RUST).block(Vec3::new(-hx + 1.0, -hy + 1.0, 0.0), Vec3::new(hx - 1.0, hy - 1.0, 0.1));
    // Swings: an A-frame with two seats.
    let s = Vec3::new(-hx * 0.5, hy * 0.4, 0.1);
    k.mat(ids::STUCCO);
    for x in [-1.6, 1.6] {
        for y in [-0.9, 0.9] {
            k.beam(s + Vec3::new(x, y, 0.0), s + Vec3::new(x, 0.0, 2.4), 0.1, 0.1);
        }
    }
    k.tube(s + Vec3::new(-1.7, 0.0, 2.4), s + Vec3::new(1.7, 0.0, 2.4), 0.07, 8);
    for x in [-0.7, 0.7] {
        k.mat(ids::IRON).beam(s + Vec3::new(x, 0.0, 2.4), s + Vec3::new(x, 0.0, 0.5), 0.02, 0.02);
        k.mat(ids::ROOF).block(s + Vec3::new(x - 0.25, -0.1, 0.45), s + Vec3::new(x + 0.25, 0.1, 0.5));
    }
    // Slide tower with a roof and a chute.
    let t = Vec3::new(hx * 0.4, hy * 0.3, 0.1);
    k.mat(ids::TRIM);
    for (x, y) in [(-0.8, -0.8), (0.8, -0.8), (0.8, 0.8), (-0.8, 0.8)] {
        k.block(t + Vec3::new(x - 0.07, y - 0.07, 0.0), t + Vec3::new(x + 0.07, y + 0.07, 3.0));
    }
    k.mat(ids::STUCCO).block(t + Vec3::new(-0.85, -0.85, 1.4), t + Vec3::new(0.85, 0.85, 1.5));
    k.at(Mat4::from_translation(t + Vec3::Z * 3.0), |k| k.mat(ids::ROOF).hipped(0.9, 0.9, 0.8, 0.1));
    k.mat(ids::SLATE).quad(t + Vec3::new(-0.4, -0.85, 1.5), t + Vec3::new(0.4, -0.85, 1.5), t + Vec3::new(0.4, -3.4, 0.2), t + Vec3::new(-0.4, -3.4, 0.2));
    k.quad(t + Vec3::new(0.4, -3.4, 0.2), t + Vec3::new(0.4, -0.85, 1.5), t + Vec3::new(-0.4, -0.85, 1.5), t + Vec3::new(-0.4, -3.4, 0.2));
    for x in [-0.42, 0.42] {
        k.beam(t + Vec3::new(x, -0.85, 1.65), t + Vec3::new(x, -3.4, 0.35), 0.05, 0.2);
    }
    k.mat(ids::IRON).beam(t + Vec3::new(0.0, 0.85, 0.1), t + Vec3::new(0.0, 2.6, 0.1), 0.6, 0.05);
    // See-saw.
    let w = Vec3::new(-hx * 0.4, -hy * 0.35, 0.1);
    k.mat(ids::IRON).frustum(w, 0.25, 0.1, 0.5, 6, true);
    k.mat(ids::STUCCO).beam(w + Vec3::new(-1.8, 0.0, 0.25), w + Vec3::new(1.8, 0.0, 0.75), 0.3, 0.08);
    // Climbing dome: arcs of bars.
    let c = Vec3::new(hx * 0.35, -hy * 0.4, 0.1);
    k.mat(ids::ROOF);
    for i in 0..4 {
        let a = i as f32 * PI / 4.0;
        let d = Vec2::from_angle(a).extend(0.0);
        let pts: Vec<Vec3> = (0..=8).map(|j| {
            let t = j as f32 / 8.0 * PI;
            c + d * (1.4 * t.cos()) + Vec3::Z * (1.4 * t.sin())
        }).collect();
        for p in pts.windows(2) {
            k.beam(p[0], p[1], 0.05, 0.05);
        }
    }
    // Sandpit and benches.
    let p = Vec3::new(0.0, -hy * 0.05, 0.1);
    k.mat(ids::TRIM).block(p - Vec3::new(1.6, 1.6, 0.0), p + Vec3::new(1.6, 1.6, 0.3));
    k.mat(ids::DUST).block(p - Vec3::new(1.45, 1.45, 0.0), p + Vec3::new(1.45, 1.45, 0.32));
    for x in [-hx * 0.6, hx * 0.6] {
        bench(k, Vec3::new(x, -hy + 1.6, 0.1), 0.0);
    }
    fence(k, Vec2::new(-hx + 0.5, -hy + 0.5), Vec2::new(hx - 0.5, hy - 0.5), 1.0, 2.0);
}

fn court(b: &mut Ctx, hx: f32, hy: f32) {
    // A hard court with lines and either a net (tennis) or two hoops
    // (basketball), a mesh fence and a bench.
    let k = &mut b.kit;
    let paint = if b.r.gen_bool(0.5) { ids::SLATE } else { ids::RUST };
    k.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    k.mat(paint).block(Vec3::new(-hx + 1.5, -hy + 1.5, 0.05), Vec3::new(hx - 1.5, hy - 1.5, 0.07));
    let (min, max) = (Vec2::new(-hx + 2.5, -hy + 2.5), Vec2::new(hx - 2.5, hy - 2.5));
    lines(k, min, max, 0.07, 0.08);
    k.block(Vec3::new(-0.04, min.y, 0.07), Vec3::new(0.04, max.y, 0.08));
    if b.r.gen_bool(0.5) {
        // Tennis: a net across the middle.
        for y in [min.y - 0.4, max.y + 0.4] {
            k.mat(ids::IRON).frustum(Vec3::new(0.0, y, 0.07), 0.05, 0.05, 1.07, 6, true);
        }
        k.mat(ids::REBAR).block(Vec3::new(-0.02, min.y - 0.4, 0.15), Vec3::new(0.02, max.y + 0.4, 0.95));
        k.mat(ids::LANE).block(Vec3::new(-0.04, min.y - 0.4, 0.95), Vec3::new(0.04, max.y + 0.4, 1.05));
    } else {
        // Basketball: centre circle and a hoop at each end.
        k.mat(ids::LANE).frustum(Vec3::Z * 0.07, 1.8, 1.8, 0.01, 20, true);
        k.mat(paint).frustum(Vec3::Z * 0.071, 1.7, 1.7, 0.01, 20, true);
        for s in [-1.0f32, 1.0] {
            let x = s * (max.x + 0.6);
            k.mat(ids::IRON).block(Vec3::new(x - 0.1, -0.1, 0.07), Vec3::new(x + 0.1, 0.1, 3.4));
            k.beam(Vec3::new(x, 0.0, 3.2), Vec3::new(x - s * 1.0, 0.0, 3.2), 0.12, 0.12);
            k.mat(ids::PLASTER).block(Vec3::new(x - s * 1.05 - 0.03, -0.9, 2.9), Vec3::new(x - s * 1.05 + 0.03, 0.9, 4.0));
            k.mat(ids::ROOF).at(Mat4::from_translation(Vec3::new(x - s * 1.35, 0.0, 3.05)), |k| k.frustum(Vec3::ZERO, 0.23, 0.23, 0.03, 12, false));
        }
    }
    fence(k, Vec2::new(-hx + 0.3, -hy + 0.3), Vec2::new(hx - 0.3, hy - 0.3), 3.0, 1.4);
    bench(k, Vec3::new(hx * 0.5, -hy + 1.0, 0.05), 0.0);
}

fn field(b: &mut Ctx, hx: f32, hy: f32) {
    // A grass pitch with markings, goals at each end, a
    // small clubhouse and floodlights at the corners.
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    let (min, max) = (Vec2::new(-hx + 4.0, -hy + 6.0), Vec2::new(hx - 4.0, hy - 3.0));
    lines(k, min, max, 0.055, 0.12);
    let cy = (min.y + max.y) * 0.5;
    k.block(Vec3::new(-0.06, min.y, 0.055), Vec3::new(0.06, max.y, 0.065));
    let dy = (max.y - min.y) * 0.22;
    for s in [-1.0f32, 1.0] {
        let x = if s < 0.0 { min.x } else { max.x };
        let bx = x - s * (max.x - min.x) * 0.13;
        lines(k, Vec2::new(x.min(bx), cy - dy), Vec2::new(x.max(bx), cy + dy), 0.056, 0.12);
        // Goal: posts, crossbar and a net box behind.
        k.mat(ids::PLASTER);
        for y in [cy - 3.66, cy + 3.66] {
            k.block(Vec3::new(x - 0.06, y - 0.06, 0.05), Vec3::new(x + 0.06, y + 0.06, 2.44));
        }
        k.block(Vec3::new(x - 0.06, cy - 3.72, 2.44), Vec3::new(x + 0.06, cy + 3.72, 2.56));
        k.mat(ids::REBAR).beam(Vec3::new(x, cy - 3.66, 2.44), Vec3::new(x + s * 2.0, cy - 3.66, 0.05), 0.02, 0.02);
        k.beam(Vec3::new(x, cy + 3.66, 2.44), Vec3::new(x + s * 2.0, cy + 3.66, 0.05), 0.02, 0.02);
        k.beam(Vec3::new(x + s * 2.0, cy - 3.66, 0.05), Vec3::new(x + s * 2.0, cy + 3.66, 0.05), 0.02, 0.02);
    }
    // Floodlights.
    for (x, y) in [(-hx + 1.5, -hy + 1.5), (hx - 1.5, -hy + 1.5), (-hx + 1.5, hy - 1.5), (hx - 1.5, hy - 1.5)] {
        k.mat(ids::IRON).frustum(Vec3::new(x, y, 0.0), 0.3, 0.18, 18.0, 8, true);
        k.mat(ids::GLASS).block(Vec3::new(x - 1.2, y - 0.3, 18.0), Vec3::new(x + 1.2, y + 0.3, 19.4));
    }
    flat_box(k, -6.0, -hy + 0.5, 6.0, -hy + 4.5, 0.0, 3.2, ids::BRICK, 0.3);
    k.at(Mat4::from_translation(Vec3::new(0.0, -hy + 2.5, 3.2)), |k| k.mat(ids::ROOF).gable(6.3, 2.3, 1.2, 0.3, ids::BRICK));
}

fn stadium(b: &mut Ctx, hx: f32, hy: f32) {
    // A pitch ringed by a running track and four tiered stands, the long
    // sides under cantilever roofs, with floodlight masts.
    let k = &mut b.kit;
    k.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    let (px, py) = (hx * 0.55, hy * 0.45);
    k.mat(ids::RUST).block(Vec3::new(-px - 4.0, -py - 4.0, 0.05), Vec3::new(px + 4.0, py + 4.0, 0.08));
    k.mat(ids::FOLIAGE).block(Vec3::new(-px, -py, 0.08), Vec3::new(px, py, 0.1));
    lines(k, Vec2::new(-px + 1.0, -py + 1.0), Vec2::new(px - 1.0, py - 1.0), 0.1, 0.15);
    k.block(Vec3::new(-0.08, -py + 1.0, 0.1), Vec3::new(0.08, py - 1.0, 0.11));
    let rows = 12;
    for side in 0..4 {
        let long = side % 2 == 0;
        let (len, gap, room) = if long { (px + 4.0, py + 4.0, hy - py - 5.0) } else { (py + 4.0, px + 4.0, hx - px - 5.0) };
        let step = (room / rows as f32).min(0.9);
        let m = Mat4::from_rotation_z(side as f32 * FRAC_PI_2);
        let seats = [ids::SLATE, ids::ROOF, ids::STUCCO][side % 3];
        k.at(m, |k| {
            for r in 0..rows {
                let y0 = -gap - r as f32 * step;
                k.mat(if r % 4 == 3 { ids::CURB } else { seats });
                k.block(Vec3::new(-len, y0 - step, 0.05), Vec3::new(len, y0, 0.6 + r as f32 * 0.55));
            }
            let back = -gap - rows as f32 * step;
            let top = 0.6 + rows as f32 * 0.55;
            k.mat(ids::CURB).block(Vec3::new(-len, back - 0.5, 0.0), Vec3::new(len, back, top + 1.5));
            if long {
                k.mat(ids::PLASTER).quad(Vec3::new(-len, back, top + 6.0), Vec3::new(len, back, top + 6.0), Vec3::new(len, -gap + 1.0, top + 4.0), Vec3::new(-len, -gap + 1.0, top + 4.0));
                k.quad(Vec3::new(-len, -gap + 1.0, top + 4.0), Vec3::new(len, -gap + 1.0, top + 4.0), Vec3::new(len, back, top + 6.0), Vec3::new(-len, back, top + 6.0));
                k.mat(ids::IRON);
                let n = ((len * 2.0 / 10.0) as usize).max(2);
                for i in 0..=n {
                    let x = -len + 2.0 * len * i as f32 / n as f32;
                    k.block(Vec3::new(x - 0.2, back - 0.2, top), Vec3::new(x + 0.2, back + 0.2, top + 6.0));
                }
            }
        });
    }
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let p = Vec3::new(x * (hx - 2.0), y * (hy - 2.0), 0.0);
        k.mat(ids::IRON).frustum(p, 0.6, 0.35, 40.0, 8, true);
        k.mat(ids::GLASS).block(p + Vec3::new(-2.5, -0.4, 40.0), p + Vec3::new(2.5, 0.4, 43.0));
    }
}

fn pool(b: &mut Ctx, hx: f32, hy: f32) {
    // An outdoor public pool: paved deck, lane ropes, a diving board,
    // a kiddie pool, loungers, parasols and a changing block.
    let k = &mut b.kit;
    k.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.2));
    let (px, py) = (hx * 0.7, hy * 0.42);
    let c = Vec3::new(0.0, hy * 0.1, 0.0);
    k.mat(ids::CURB).block(c + Vec3::new(-px - 0.4, -py - 0.4, 0.2), c + Vec3::new(px + 0.4, py + 0.4, 0.3));
    k.mat(ids::WATER).block(c + Vec3::new(-px, -py, 0.15), c + Vec3::new(px, py, 0.32));
    let lanes = ((py * 2.0 / 2.5) as usize).max(2);
    for i in 1..lanes {
        let y = c.y - py + 2.0 * py * i as f32 / lanes as f32;
        k.mat(if i % 2 == 0 { ids::ROOF } else { ids::PLASTER }).tube(Vec3::new(-px, y, 0.34), Vec3::new(px, y, 0.34), 0.05, 6);
    }
    k.mat(ids::PLASTER).block(Vec3::new(-px - 0.4, c.y - 0.3, 0.3), Vec3::new(-px + 2.5, c.y + 0.3, 1.0));
    k.mat(ids::IRON).block(Vec3::new(-px - 0.6, c.y - 0.4, 0.3), Vec3::new(-px + 0.2, c.y + 0.4, 0.9));
    let kp = Vec3::new(px * 0.6, -hy + 3.5, 0.0);
    k.mat(ids::CURB).frustum(kp + Vec3::Z * 0.2, 2.2, 2.2, 0.1, 16, true);
    k.mat(ids::WATER).frustum(kp + Vec3::Z * 0.2, 2.0, 2.0, 0.12, 16, true);
    for i in 0..((px * 2.0 / 3.0) as usize).min(8) {
        let x = -px + 1.0 + i as f32 * 3.0;
        let y = c.y + py + 1.6;
        k.mat(ids::PLASTER).beam(Vec3::new(x, y - 0.9, 0.5), Vec3::new(x, y + 0.6, 0.5), 0.6, 0.12);
        k.beam(Vec3::new(x, y + 0.6, 0.5), Vec3::new(x, y + 1.1, 1.1), 0.6, 0.1);
        if i % 3 == 1 && y + 1.4 < hy {
            k.mat(ids::IRON).frustum(Vec3::new(x + 1.5, y, 0.2), 0.04, 0.04, 2.3, 6, true);
            k.mat(ids::ROOF).frustum(Vec3::new(x + 1.5, y, 2.1), 1.3, 0.05, 0.5, 8, true);
        }
    }
    flat_box(k, -hx + 1.0, -hy + 1.0, -hx * 0.2, -hy + 5.0, 0.2, 3.0, ids::STUCCO, 0.3);
    fence(k, Vec2::new(-hx + 0.2, -hy + 0.2), Vec2::new(hx - 0.2, hy - 0.2), 1.8, 0.0);
    for (x, y) in [(hx - 1.5, hy - 1.5), (-hx + 1.5, hy - 1.5)] {
        tree(k, Vec3::new(x, y, 0.2), 6.0);
    }
}
