//! Lighting & decor: floor lamp, pendant light, rug with pouf, curtains,
//! wall mirror and a big potted plant. Floor at z = 0, front facing -Y;
//! wall pieces hang on a wall at +Y at their real height.

use super::furniture::{cushion, fabric};
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::ids;
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{PI, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::FloorLamp => floor_lamp(b),
        K::PendantLight => pendant(b),
        K::RugAndPouf => rug(b, hx, hy),
        K::Curtains => curtains(b, hx, hy),
        K::WallMirror => mirror(b, hx, hy),
        K::FloorPlant => plant(b),
        _ => {}
    }
}

fn floor_lamp(b: &mut Ctx) {
    // A straight standard lamp with a drum shade, or an arc lamp with a
    // dome shade (by seed).
    let shade = fabric(b);
    let arc = b.r.gen_bool(0.4);
    let glow = b.glow(false);
    let k = &mut b.kit;
    k.mat(ids::IRON).frustum(Vec3::ZERO, 0.16, 0.15, 0.03, 16, true);
    if arc {
        // A straight pole to 1.4 m, then an arc over and down towards -Y.
        k.frustum(Vec3::new(0.0, 0.0, 0.03), 0.015, 0.015, 1.37, 8, true);
        let n = 12;
        let pts: Vec<Vec3> = (0..=n).map(|i| {
            let a = PI * 0.8 * i as f32 / n as f32;
            Vec3::new(0.0, -0.6 + 0.6 * a.cos(), 1.4 + 0.6 * a.sin())
        }).collect();
        for w in pts.windows(2) {
            k.beam(w[0], w[1], 0.025, 0.025);
        }
        let end = *pts.last().unwrap();
        k.mat(glow).frustum(end - Vec3::Z * 0.16, 0.04, 0.04, 0.05, 8, true);
        k.mat(ids::IRON).frustum(end - Vec3::Z * 0.18, 0.2, 0.05, 0.18, 16, false);
    } else {
        k.frustum(Vec3::new(0.0, 0.0, 0.03), 0.012, 0.012, 1.45, 8, true);
        k.mat(glow).frustum(Vec3::new(0.0, 0.0, 1.38), 0.035, 0.035, 0.08, 8, true);
        k.mat(shade).frustum(Vec3::new(0.0, 0.0, 1.3), 0.22, 0.2, 0.32, 20, false);
    }
}

fn pendant(b: &mut Ctx) {
    // A ceiling rose at 2.5 m, a flex and a dome or cone shade with a
    // bulb, hanging to about 1.9 m.
    let shade = [ids::IRON, ids::PLASTER, ids::SLATE, ids::ROOF][b.r.gen_range(0..4)];
    let cone = b.r.gen_bool(0.5);
    let glow = b.glow(false);
    let k = &mut b.kit;
    k.mat(ids::PLASTER).frustum(Vec3::new(0.0, 0.0, 2.47), 0.06, 0.05, 0.03, 12, true);
    k.mat(ids::ASPHALT).frustum(Vec3::new(0.0, 0.0, 2.15), 0.004, 0.004, 0.32, 4, true);
    k.mat(glow).frustum(Vec3::new(0.0, 0.0, 1.98), 0.04, 0.04, 0.08, 10, true);
    k.mat(shade);
    if cone {
        k.frustum(Vec3::new(0.0, 0.0, 1.92), 0.2, 0.03, 0.24, 20, false);
    } else {
        let n = 6;
        for i in 0..n {
            let (a0, a1) = (FRAC_PI * i as f32 / n as f32, FRAC_PI * (i + 1) as f32 / n as f32);
            let (r0, r1) = (0.22 * a0.cos(), 0.22 * a1.cos());
            k.frustum(Vec3::new(0.0, 0.0, 1.92 + 0.2 * a0.sin()), r0.max(0.03), r1.max(0.03), 0.2 * (a1.sin() - a0.sin()), 20, false);
        }
    }
}

const FRAC_PI: f32 = PI * 0.5;

fn rug(b: &mut Ctx, hx: f32, hy: f32) {
    // A patterned rug with a border and fringe, and a round pouf on it.
    let (base, band) = (fabric(b), fabric(b));
    let k = &mut b.kit;
    k.mat(base).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.012));
    k.mat(band);
    for (min, max) in [
        (Vec2::new(-hx + 0.08, -hy + 0.08), Vec2::new(hx - 0.08, -hy + 0.16)),
        (Vec2::new(-hx + 0.08, hy - 0.16), Vec2::new(hx - 0.08, hy - 0.08)),
        (Vec2::new(-hx + 0.08, -hy + 0.16), Vec2::new(-hx + 0.16, hy - 0.16)),
        (Vec2::new(hx - 0.16, -hy + 0.16), Vec2::new(hx - 0.08, hy - 0.16)),
    ] {
        k.block(min.extend(0.012), max.extend(0.014));
    }
    for i in 0..5 {
        let r = 0.1 + i as f32 * 0.12;
        if r < hy - 0.2 {
            k.frustum(Vec3::new(0.0, 0.0, 0.012), r, r, 0.001 + i as f32 * 0.0002, 24, true);
        }
    }
    k.mat(ids::STUCCO);
    let n = ((hx * 2.0 / 0.05) as usize).min(80);
    for s in [-1.0f32, 1.0] {
        for i in 0..n {
            let y = -hy + (i as f32 + 0.5) * hy * 2.0 / n as f32;
            k.block(Vec3::new(s * hx - 0.03 * (1.0 - s) * 0.5, y - 0.008, 0.0), Vec3::new(s * hx + 0.03 * (1.0 + s) * 0.5, y + 0.008, 0.006));
        }
    }
    let p = Vec3::new(hx * 0.45, -hy * 0.3, 0.012);
    k.mat(band).frustum(p, 0.25, 0.27, 0.2, 16, false);
    k.frustum(p + Vec3::Z * 0.2, 0.27, 0.22, 0.2, 16, true);
}

fn curtains(b: &mut Ctx, hx: f32, hy: f32) {
    // A curtain pole with finials and two gathered drapes (pleated
    // panels) tied back, with a sheer between them.
    let cloth = fabric(b);
    let k = &mut b.kit;
    let (top, y) = (2.3, hy - 0.08);
    k.mat(ids::IRON).tube(Vec3::new(-hx, y, top), Vec3::new(hx, y, top), 0.018, 8);
    for x in [-hx, hx] {
        k.frustum(Vec3::new(x, y, top - 0.04), 0.035, 0.035, 0.08, 10, true);
        k.block(Vec3::new(x * 0.94 - 0.01, y, top - 0.02), Vec3::new(x * 0.94 + 0.01, hy, top + 0.02));
    }
    let panel = (hx * 0.35).max(0.3);
    let folds = 7;
    for s in [-1.0f32, 1.0] {
        let x0 = s * hx;
        let x1 = s * (hx - panel);
        k.mat(cloth);
        for i in 0..folds {
            let (t0, t1) = (i as f32 / folds as f32, (i + 1) as f32 / folds as f32);
            let (a, c) = (x0 + (x1 - x0) * t0, x0 + (x1 - x0) * t1);
            let dy = if i % 2 == 0 { -0.06 } else { 0.0 };
            let (pa, pc) = (Vec2::new(a, y + dy), Vec2::new(c, y - 0.06 - dy));
            let (lo, hi) = (0.02, top - 0.03);
            k.quad(pa.extend(lo), pc.extend(lo), pc.extend(hi), pa.extend(hi));
            k.quad(pa.extend(hi), pc.extend(hi), pc.extend(lo), pa.extend(lo));
        }
        k.mat(ids::STUCCO).tube(Vec3::new(x0, y - 0.1, 1.0), Vec3::new(x1, y - 0.1, 1.0), 0.02, 6);
    }
    k.mat(ids::GLASS);
    let (a, c) = (Vec2::new(-hx + panel, y), Vec2::new(hx - panel, y));
    k.quad(a.extend(0.05), c.extend(0.05), c.extend(top - 0.03), a.extend(top - 0.03));
    k.quad(a.extend(top - 0.03), c.extend(top - 0.03), c.extend(0.05), a.extend(0.05));
}

fn mirror(b: &mut Ctx, hx: f32, hy: f32) {
    // A framed wall mirror (rectangular or round) hung at eye height,
    // with a small shelf below.
    let frame = [ids::TRIM, ids::IRON, ids::STUCCO][b.r.gen_range(0..3)];
    let round = b.r.gen_bool(0.4);
    let k = &mut b.kit;
    let y = hy - 0.03;
    let (c, r) = (1.5, hx.min(0.45));
    if round {
        let m = bevy::math::Mat4::from_translation(Vec3::new(0.0, y, c)) * bevy::math::Mat4::from_rotation_x(-PI * 0.5);
        k.at(m, |k| {
            k.mat(frame).frustum(Vec3::ZERO, r, r, 0.03, 32, true);
            k.mat(ids::GLASS).frustum(Vec3::new(0.0, 0.0, 0.03), r - 0.04, r - 0.04, 0.002, 32, true);
        });
    } else {
        k.mat(frame);
        cushion(k, Vec3::new(-r, y - 0.03, c - r * 1.4), Vec3::new(r, hy, c + r * 1.4));
        k.mat(ids::GLASS).block(Vec3::new(-r + 0.04, y - 0.032, c - r * 1.4 + 0.04), Vec3::new(r - 0.04, y - 0.03, c + r * 1.4 - 0.04));
    }
    k.mat(frame).block(Vec3::new(-r * 0.8, hy - 0.14, 0.95), Vec3::new(r * 0.8, hy, 0.98));
    k.mat(ids::SLATE).frustum(Vec3::new(r * 0.4, hy - 0.07, 0.98), 0.03, 0.025, 0.12, 8, true);
    k.mat(ids::ROOF).block(Vec3::new(-r * 0.5, hy - 0.1, 0.98), Vec3::new(-r * 0.3, hy - 0.04, 1.02));
}

fn plant(b: &mut Ctx) {
    // A big floor plant: a round planter and a cluster of leaf blades
    // or a small tree crown (by seed).
    let pot = [ids::SLATE, ids::PLASTER, ids::RUST, ids::STUCCO][b.r.gen_range(0..4)];
    let palm = b.r.gen_bool(0.5);
    let k = &mut b.kit;
    k.mat(pot).frustum(Vec3::ZERO, 0.17, 0.22, 0.42, 16, true);
    k.mat(ids::LOT).frustum(Vec3::new(0.0, 0.0, 0.4), 0.2, 0.2, 0.012, 16, true);
    if palm {
        k.mat(ids::TRIM).frustum(Vec3::new(0.0, 0.0, 0.41), 0.03, 0.02, 0.6, 6, true);
        for i in 0..9 {
            let a = i as f32 * TAU / 9.0;
            let d = Vec2::from_angle(a);
            let base = Vec3::new(0.0, 0.0, 0.7 + (i % 3) as f32 * 0.15);
            let tip = base + (d * 0.55).extend(0.2 - (i % 2) as f32 * 0.3);
            let side = Vec3::new(-d.y, d.x, 0.0) * 0.07;
            let mid = (base + tip) * 0.5 + Vec3::Z * 0.12;
            k.mat(ids::FOLIAGE);
            k.poly(&[base, mid - side, tip, mid + side]);
            k.poly(&[base, mid + side, tip, mid - side]);
        }
    } else {
        k.mat(ids::TRIM).frustum(Vec3::new(0.0, 0.0, 0.41), 0.025, 0.02, 0.7, 6, true);
        super::garden::shrub(k, Vec3::new(0.0, 0.0, 1.0), 0.38);
        super::garden::shrub(k, Vec3::new(0.1, -0.05, 1.3), 0.25);
    }
}
