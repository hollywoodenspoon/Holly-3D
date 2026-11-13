//! Rural prefabs: barn, greenhouse, polytunnel, henhouse, haystacks,
//! orchard, paddock and beehives. Ground at z = 0, front facing -Y.

use super::garden::shrub;
use super::houses::door;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Barn => barn(b, hx, hy),
        K::Greenhouse => greenhouse(b, hx, hy),
        K::Polytunnel => polytunnel(b, hx, hy),
        K::Henhouse => henhouse(b, hx, hy),
        K::Haystacks => haystacks(b, hx, hy),
        K::Orchard => orchard(b, hx, hy),
        K::Paddock => paddock(b, hx, hy),
        K::Beehives => beehives(b, hx, hy),
        _ => {}
    }
}

/// A post-and-rail farm fence from `a` to `c`.
fn rails(k: &mut Kit, a: Vec2, c: Vec2) {
    let n = ((a.distance(c) / 2.5).ceil() as usize).max(1);
    k.mat(ids::TRIM);
    for i in 0..=n {
        let p = a.lerp(c, i as f32 / n as f32);
        k.block((p - Vec2::splat(0.07)).extend(0.0), (p + Vec2::splat(0.07)).extend(1.3));
    }
    for z in [0.5, 1.1] {
        k.beam(a.extend(z), c.extend(z), 0.06, 0.12);
    }
}

fn barn(b: &mut Ctx, hx: f32, hy: f32) {
    // A red timber barn with a gambrel roof, big sliding doors with
    // cross braces, a hay loft door and a lean-to on one side.
    let paint = if b.r.gen_bool(0.7) { ids::ROOF } else { ids::TRIM };
    let (bx, by) = (hx * 0.7, hy - 0.5);
    let c = Vec2::new(-hx + bx + 0.5, 0.0);
    let h = 5.0;
    let k = &mut b.kit;
    k.mat(paint).block(Vec3::new(c.x - bx, -by, 0.0), Vec3::new(c.x + bx, by, h));
    // Gambrel profile: steep lower slopes, shallow upper slopes.
    let prof = [Vec2::new(-bx - 0.3, h - 0.2), Vec2::new(-bx * 0.62, h + 3.0), Vec2::new(0.0, h + 4.4), Vec2::new(bx * 0.62, h + 3.0), Vec2::new(bx + 0.3, h - 0.2)];
    for w in prof.windows(2) {
        let (p, q) = (w[0], w[1]);
        k.mat(ids::SLATE).quad(Vec3::new(c.x + p.x, by + 0.3, p.y), Vec3::new(c.x + q.x, by + 0.3, q.y), Vec3::new(c.x + q.x, -by - 0.3, q.y), Vec3::new(c.x + p.x, -by - 0.3, p.y));
        k.quad(Vec3::new(c.x + p.x, -by - 0.3, p.y), Vec3::new(c.x + q.x, -by - 0.3, q.y), Vec3::new(c.x + q.x, by + 0.3, q.y), Vec3::new(c.x + p.x, by + 0.3, p.y));
    }
    for y in [-by, by] {
        let s = if y < 0.0 { -1.0 } else { 1.0 };
        let mut g = vec![Vec3::new(c.x + bx, y, h), Vec3::new(c.x + prof[3].x, y, prof[3].y), Vec3::new(c.x, y, prof[2].y - 0.1), Vec3::new(c.x + prof[1].x, y, prof[1].y), Vec3::new(c.x - bx, y, h)];
        if s > 0.0 {
            g.reverse();
        }
        k.mat(paint).poly(&g);
    }
    // Front: sliding doors with white trim and cross braces, loft door.
    let fy = -by - 0.03;
    let dw = bx * 0.5;
    k.mat(ids::PLASTER);
    for x in [c.x - dw, c.x, c.x + dw] {
        k.block(Vec3::new(x - 0.1, fy - 0.05, 0.0), Vec3::new(x + 0.1, fy, 3.8));
    }
    k.block(Vec3::new(c.x - dw - 0.1, fy - 0.05, 3.8), Vec3::new(c.x + dw + 0.1, fy, 4.0));
    for s in [-1.0f32, 1.0] {
        let (x0, x1) = (c.x, c.x + s * dw);
        k.beam(Vec3::new(x0, fy - 0.06, 0.1), Vec3::new(x1, fy - 0.06, 3.7), 0.04, 0.16);
        k.beam(Vec3::new(x1, fy - 0.06, 0.1), Vec3::new(x0, fy - 0.06, 3.7), 0.04, 0.16);
    }
    k.mat(ids::ASPHALT).block(Vec3::new(c.x - 0.8, fy - 0.02, h + 0.6), Vec3::new(c.x + 0.8, fy, h + 2.2));
    k.mat(ids::PLASTER).block(Vec3::new(c.x - 0.9, fy - 0.05, h + 2.2), Vec3::new(c.x + 0.9, fy, h + 2.35));
    k.mat(ids::IRON).beam(Vec3::new(c.x, fy - 0.1, h + 2.6), Vec3::new(c.x, fy - 1.0, h + 2.6), 0.12, 0.12);
    // Lean-to on the right side.
    let lx = c.x + bx;
    if hx - lx > 2.0 {
        let e = hx - 0.2;
        k.mat(ids::TRIM).block(Vec3::new(lx, -by * 0.8, 0.0), Vec3::new(e, by * 0.8, 2.6));
        k.mat(ids::RUST).quad(Vec3::new(e + 0.2, -by * 0.8 - 0.2, 2.6), Vec3::new(e + 0.2, by * 0.8 + 0.2, 2.6), Vec3::new(lx, by * 0.8 + 0.2, 3.8), Vec3::new(lx, -by * 0.8 - 0.2, 3.8));
        k.quad(Vec3::new(lx, -by * 0.8 - 0.2, 3.8), Vec3::new(lx, by * 0.8 + 0.2, 3.8), Vec3::new(e + 0.2, by * 0.8 + 0.2, 2.6), Vec3::new(e + 0.2, -by * 0.8 - 0.2, 2.6));
        door(k, (lx + e) * 0.5, -by * 0.8 - 0.02, 0.0, 1.0, 2.1);
    }
}

fn greenhouse(b: &mut Ctx, hx: f32, hy: f32) {
    // A glass house on a low brick wall: aluminium frame, a glazed gable
    // roof, a door, staging benches and plants inside.
    let (h, rise) = (2.0, hx.min(hy) * 0.45);
    let k = &mut b.kit;
    k.mat(ids::BRICK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.6));
    k.mat(ids::DUST).block(Vec3::new(-hx + 0.1, -hy + 0.1, 0.6), Vec3::new(hx - 0.1, hy - 0.1, 0.62));
    k.mat(ids::GLASS);
    for (a, c) in [(Vec2::new(-hx, -hy), Vec2::new(hx, -hy)), (Vec2::new(hx, -hy), Vec2::new(hx, hy)), (Vec2::new(hx, hy), Vec2::new(-hx, hy)), (Vec2::new(-hx, hy), Vec2::new(-hx, -hy))] {
        k.quad(a.extend(0.6), c.extend(0.6), c.extend(h), a.extend(h));
    }
    for y in [-hy, hy] {
        let pts = [Vec3::new(-hx, y, h), Vec3::new(hx, y, h), Vec3::new(0.0, y, h + rise)];
        k.poly(&if y < 0.0 { pts } else { [pts[1], pts[0], pts[2]] });
    }
    for s in [-1.0f32, 1.0] {
        let (e, r) = (Vec3::new(s * hx, 0.0, h), Vec3::new(0.0, 0.0, h + rise));
        let q = [e - Vec3::Y * hy, e + Vec3::Y * hy, r + Vec3::Y * hy, r - Vec3::Y * hy];
        k.quad(q[0], q[1], q[2], q[3]);
        k.quad(q[3], q[2], q[1], q[0]);
    }
    k.mat(ids::PLASTER);
    let n = ((hy * 2.0 / 0.8) as usize).max(2);
    for i in 0..=n {
        let y = -hy + 2.0 * hy * i as f32 / n as f32;
        for s in [-1.0f32, 1.0] {
            k.beam(Vec3::new(s * hx, y, 0.6), Vec3::new(s * hx, y, h), 0.04, 0.04);
            k.beam(Vec3::new(s * hx, y, h), Vec3::new(0.0, y, h + rise), 0.04, 0.04);
        }
    }
    k.beam(Vec3::new(0.0, -hy, h + rise), Vec3::new(0.0, hy, h + rise), 0.06, 0.06);
    for s in [-1.0f32, 1.0] {
        k.beam(Vec3::new(s * hx, -hy, h), Vec3::new(s * hx, hy, h), 0.05, 0.05);
    }
    k.mat(ids::PLASTER).block(Vec3::new(-0.45, -hy - 0.04, 0.0), Vec3::new(0.45, -hy, 2.0));
    k.mat(ids::GLASS).block(Vec3::new(-0.38, -hy - 0.05, 0.1), Vec3::new(0.38, -hy - 0.04, 1.9));
    for s in [-1.0f32, 1.0] {
        let x = s * (hx - 0.5);
        k.mat(ids::TRIM).block(Vec3::new(x - 0.35, -hy + 0.4, 0.95), Vec3::new(x + 0.35, hy - 0.4, 1.0));
        let pots = (((hy * 2.0) - 0.8) / 0.4) as usize;
        for i in 0..pots {
            let p = Vec3::new(x, -hy + 0.6 + i as f32 * 0.4, 1.0);
            k.mat(ids::RUST).frustum(p, 0.08, 0.11, 0.14, 8, true);
            shrub(k, p + Vec3::Z * 0.14, 0.14);
        }
    }
}

fn polytunnel(b: &mut Ctx, hx: f32, hy: f32) {
    // A long half-round plastic tunnel along Y over crop rows, with a
    // framed door at the front.
    let r = hx.min(3.5);
    let n = 12;
    let k = &mut b.kit;
    k.mat(ids::LOT).block(Vec3::new(-r, -hy, 0.0), Vec3::new(r, hy, 0.03));
    let arc: Vec<Vec2> = (0..=n).map(|i| Vec2::from_angle(PI * i as f32 / n as f32) * r).collect();
    for w in arc.windows(2) {
        let (p, q) = (w[0], w[1]);
        k.mat(ids::PLASTER).quad(Vec3::new(p.x, -hy, p.y), Vec3::new(p.x, hy, p.y), Vec3::new(q.x, hy, q.y), Vec3::new(q.x, -hy, q.y));
    }
    for y in [-hy, hy] {
        let mut pts: Vec<Vec3> = arc.iter().map(|p| Vec3::new(p.x, y, p.y)).collect();
        if y > 0.0 {
            pts.reverse();
        }
        k.mat(ids::GLASS).poly(&pts);
    }
    let hoops = ((hy * 2.0 / 2.0) as usize).max(2);
    for i in 0..=hoops {
        let y = -hy + 2.0 * hy * i as f32 / hoops as f32;
        for w in arc.windows(2) {
            k.mat(ids::IRON).beam(Vec3::new(w[0].x, y, w[0].y + 0.02), Vec3::new(w[1].x, y, w[1].y + 0.02), 0.04, 0.04);
        }
    }
    k.mat(ids::TRIM).block(Vec3::new(-0.55, -hy - 0.05, 0.0), Vec3::new(-0.45, -hy, 2.0));
    k.block(Vec3::new(0.45, -hy - 0.05, 0.0), Vec3::new(0.55, -hy, 2.0));
    k.block(Vec3::new(-0.55, -hy - 0.05, 2.0), Vec3::new(0.55, -hy, 2.1));
    for x in [-r * 0.5, 0.0, r * 0.5] {
        k.mat(ids::DUST).block(Vec3::new(x - 0.4, -hy + 0.5, 0.03), Vec3::new(x + 0.4, hy - 0.5, 0.15));
        let m = ((hy * 2.0 - 1.0) / 0.6) as usize;
        for j in 0..m {
            shrub(k, Vec3::new(x, -hy + 0.8 + j as f32 * 0.6, 0.15), 0.2);
        }
    }
}

fn henhouse(b: &mut Ctx, hx: f32, hy: f32) {
    // A raised coop on legs with a ramp, nesting box and a wire run.
    let (w, d, z) = (hx.min(1.2), hy.min(1.0) * 0.6, 0.6);
    let k = &mut b.kit;
    k.mat(ids::TRIM);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        k.block(Vec3::new(x * w * 0.9 - 0.05, y * d * 0.9 - 0.05, 0.0), Vec3::new(x * w * 0.9 + 0.05, y * d * 0.9 + 0.05, z));
    }
    k.mat(ids::STUCCO).block(Vec3::new(-w, -d, z), Vec3::new(w, d, z + 1.0));
    k.at(Mat4::from_translation(Vec3::Z * (z + 1.0)) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::RUST).gable(d, w, 0.5, 0.12, ids::STUCCO));
    k.mat(ids::STUCCO).block(Vec3::new(w, -d * 0.6, z + 0.2), Vec3::new(w + 0.4, d * 0.6, z + 0.6));
    k.mat(ids::RUST).quad(Vec3::new(w, -d * 0.7, z + 0.75), Vec3::new(w + 0.5, -d * 0.7, z + 0.55), Vec3::new(w + 0.5, d * 0.7, z + 0.55), Vec3::new(w, d * 0.7, z + 0.75));
    k.mat(ids::ASPHALT).block(Vec3::new(-0.15, -d - 0.01, z + 0.05), Vec3::new(0.15, -d, z + 0.4));
    k.mat(ids::TRIM).quad(Vec3::new(-0.15, -d - 1.2, 0.05), Vec3::new(0.15, -d - 1.2, 0.05), Vec3::new(0.15, -d, z), Vec3::new(-0.15, -d, z));
    k.quad(Vec3::new(0.15, -d, z), Vec3::new(0.15, -d - 1.2, 0.05), Vec3::new(-0.15, -d - 1.2, 0.05), Vec3::new(-0.15, -d, z));
    let (rx0, rx1, ry0, ry1) = (-hx + 0.1, -w - 0.1, -hy + 0.1, hy - 0.1);
    if rx1 - rx0 > 0.8 {
        k.mat(ids::TRIM);
        for (x, y) in [(rx0, ry0), (rx1, ry0), (rx0, ry1), (rx1, ry1)] {
            k.block(Vec3::new(x - 0.04, y - 0.04, 0.0), Vec3::new(x + 0.04, y + 0.04, 1.2));
        }
        k.mat(ids::REBAR);
        for z in [0.3, 0.6, 0.9, 1.2] {
            for (a, c) in [(Vec2::new(rx0, ry0), Vec2::new(rx1, ry0)), (Vec2::new(rx0, ry1), Vec2::new(rx1, ry1)), (Vec2::new(rx0, ry0), Vec2::new(rx0, ry1))] {
                k.beam(a.extend(z), c.extend(z), 0.01, 0.01);
            }
        }
    }
    for i in 0..3 {
        let p = Vec3::new(-hx * 0.6 + b.r.gen_range(-0.3..0.3), -hy * 0.3 + i as f32 * 0.5, 0.0);
        let k = &mut b.kit;
        k.mat(if i == 0 { ids::TRIM } else { ids::PLASTER }).cuboid(p + Vec3::Z * 0.2, Vec3::new(0.15, 0.1, 0.1), bevy::math::Quat::IDENTITY);
        k.cuboid(p + Vec3::new(0.14, 0.0, 0.33), Vec3::splat(0.05), bevy::math::Quat::IDENTITY);
        k.mat(ids::ROOF).block(p + Vec3::new(0.12, -0.01, 0.38), p + Vec3::new(0.16, 0.01, 0.42));
        k.mat(ids::STUCCO).block(p + Vec3::new(-0.02, -0.02, 0.0), p + Vec3::new(0.02, 0.02, 0.1));
    }
}

fn haystacks(b: &mut Ctx, hx: f32, hy: f32) {
    // Round bales and a stack of square bales on stubble.
    let k = &mut b.kit;
    k.mat(ids::LOT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.03));
    let n = ((hx * hy) / 6.0) as usize;
    for i in 0..n.clamp(3, 12) {
        let p = Vec2::new(b.r.gen_range(-hx + 1.0..hx - 1.0), b.r.gen_range(-hy + 1.0..hy * 0.2));
        let yaw = b.r.gen_range(0.0..TAU);
        let k = &mut b.kit;
        let upright = i % 4 == 3;
        let m = Mat4::from_translation(p.extend(0.03)) * Mat4::from_rotation_z(yaw);
        k.at(m, |k| {
            if upright {
                k.mat(ids::STUCCO).frustum(Vec3::ZERO, 0.75, 0.75, 1.2, 12, true);
            } else {
                k.at(Mat4::from_translation(Vec3::new(0.0, -0.6, 0.75)) * Mat4::from_rotation_x(-FRAC_PI_2), |k| k.mat(ids::STUCCO).frustum(Vec3::ZERO, 0.75, 0.75, 1.2, 12, true));
            }
        });
    }
    let c = Vec3::new(0.0, hy * 0.6, 0.03);
    for l in 0..3 {
        for i in 0..(4 - l) {
            let x = c.x - 1.3 + l as f32 * 0.6 + i as f32 * 1.2;
            k_bale(&mut b.kit, Vec3::new(x, c.y, c.z + l as f32 * 0.45));
        }
    }
}

fn k_bale(k: &mut Kit, p: Vec3) {
    k.mat(ids::STUCCO).block(p + Vec3::new(-0.58, -0.4, 0.0), p + Vec3::new(0.58, 0.4, 0.44));
    k.mat(ids::TRIM);
    for x in [-0.25, 0.25] {
        k.block(p + Vec3::new(x - 0.02, -0.41, 0.0), p + Vec3::new(x + 0.02, 0.41, 0.45));
    }
}

fn orchard(b: &mut Ctx, hx: f32, hy: f32) {
    // Rows of round-crowned fruit trees on grass, fruit dotted in the
    // crowns, a ladder and crates.
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.03));
    let (nx, ny) = (((hx * 2.0 / 5.0) as usize).max(1), ((hy * 2.0 / 5.0) as usize).max(1));
    let fruit = if b.r.gen_bool(0.5) { ids::ROOF } else { ids::STUCCO };
    for i in 0..nx {
        for j in 0..ny {
            let p = Vec3::new(-hx + 2.5 + i as f32 * 5.0, -hy + 2.5 + j as f32 * 5.0, 0.03);
            let h = b.r.gen_range(2.6..3.6);
            let k = &mut b.kit;
            k.mat(ids::TRIM).frustum(p, 0.14, 0.1, h * 0.5, 6, true);
            k.mat(ids::FOLIAGE).frustum(p + Vec3::Z * h * 0.4, 0.8, 1.6, h * 0.3, 8, false);
            k.frustum(p + Vec3::Z * h * 0.7, 1.6, 0.4, h * 0.4, 8, true);
            for f in 0..6 {
                let a = f as f32 * TAU / 6.0 + i as f32;
                let q = p + (Vec2::from_angle(a) * 1.55).extend(h * 0.7);
                k.mat(fruit).cuboid(q, Vec3::splat(0.07), bevy::math::Quat::IDENTITY);
            }
        }
    }
    let k = &mut b.kit;
    let l = Vec3::new(-hx + 3.3, -hy + 2.5, 0.03);
    k.mat(ids::TRIM);
    for s in [-0.2, 0.2] {
        k.beam(l + Vec3::new(0.6, s, 0.04), l + Vec3::new(0.0, s, 2.6), 0.05, 0.05);
    }
    for c in 0..3 {
        let p = Vec3::new(-hx + 4.2 + c as f32 * 0.6, -hy + 1.2, 0.03);
        k.mat(ids::TRIM).block(p - Vec3::new(0.25, 0.18, 0.0), p + Vec3::new(0.25, 0.18, 0.3));
        k.mat(fruit).block(p - Vec3::new(0.22, 0.15, -0.25), p + Vec3::new(0.22, 0.15, 0.33));
    }
}

fn paddock(b: &mut Ctx, hx: f32, hy: f32) {
    // A grass paddock with a post-and-rail fence, a gate, a water trough
    // and a small field shelter.
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.03));
    let (x0, x1, y0, y1) = (-hx + 0.3, hx - 0.3, -hy + 0.3, hy - 0.3);
    rails(k, Vec2::new(x0, y0), Vec2::new(-1.8, y0));
    rails(k, Vec2::new(1.8, y0), Vec2::new(x1, y0));
    rails(k, Vec2::new(x1, y0), Vec2::new(x1, y1));
    rails(k, Vec2::new(x1, y1), Vec2::new(x0, y1));
    rails(k, Vec2::new(x0, y1), Vec2::new(x0, y0));
    // Five-bar gate, swung open.
    k.at(Mat4::from_translation(Vec3::new(-1.8, y0, 0.0)) * Mat4::from_rotation_z(-0.6), |k| {
        k.mat(ids::PLASTER);
        for i in 0..5 {
            let z = 0.25 + i as f32 * 0.24;
            k.block(Vec3::new(0.0, -0.03, z), Vec3::new(3.5, 0.03, z + 0.08));
        }
        k.beam(Vec3::new(0.05, 0.0, 0.25), Vec3::new(3.4, 0.0, 1.29), 0.06, 0.08);
        k.block(Vec3::new(0.0, -0.05, 0.1), Vec3::new(0.12, 0.05, 1.35));
    });
    let t = Vec3::new(x1 - 3.0, y0 + 3.0, 0.03);
    k.mat(ids::IRON).block(t - Vec3::new(1.2, 0.35, 0.0), t + Vec3::new(1.2, 0.35, 0.6));
    k.mat(ids::WATER).block(t - Vec3::new(1.12, 0.27, -0.45), t + Vec3::new(1.12, 0.27, 0.56));
    let s = Vec3::new(x0 + 3.5, y1 - 2.5, 0.03);
    k.mat(ids::TRIM).block(s - Vec3::new(3.0, 1.8, 0.0), s + Vec3::new(3.0, -1.6, 2.6));
    for x in [-3.0, 3.0] {
        k.block(s + Vec3::new(x - 0.1, -1.8, 0.0), s + Vec3::new(x + 0.1, 1.8, 2.4));
    }
    k.mat(ids::RUST).quad(s + Vec3::new(-3.2, -2.1, 2.3), s + Vec3::new(3.2, -2.1, 2.3), s + Vec3::new(3.2, 2.0, 2.8), s + Vec3::new(-3.2, 2.0, 2.8));
    k.quad(s + Vec3::new(-3.2, 2.0, 2.8), s + Vec3::new(3.2, 2.0, 2.8), s + Vec3::new(3.2, -2.1, 2.3), s + Vec3::new(-3.2, -2.1, 2.3));
    k.mat(ids::TRIM).block(s + Vec3::new(-3.0, 1.6, 0.0), s + Vec3::new(3.0, 1.8, 2.7));
}

fn beehives(b: &mut Ctx, hx: f32, hy: f32) {
    // A row of stacked box hives on stands among wildflowers.
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.03));
    let n = ((hx * 2.0 / 1.4) as usize).clamp(1, 8);
    let paints = [ids::PLASTER, ids::STUCCO, ids::SLATE];
    for i in 0..n {
        let x = -hx + 0.7 + i as f32 * 1.4;
        let boxes = b.r.gen_range(2..4);
        let k = &mut b.kit;
        k.mat(ids::TRIM).block(Vec3::new(x - 0.3, -0.3, 0.0), Vec3::new(x + 0.3, 0.3, 0.35));
        for j in 0..boxes {
            let z = 0.35 + j as f32 * 0.28;
            k.mat(paints[(i + j) % 3]).block(Vec3::new(x - 0.25, -0.25, z), Vec3::new(x + 0.25, 0.25, z + 0.27));
        }
        let top = 0.35 + boxes as f32 * 0.28;
        k.mat(ids::SLATE).block(Vec3::new(x - 0.3, -0.3, top), Vec3::new(x + 0.3, 0.3, top + 0.08));
        k.mat(ids::ASPHALT).block(Vec3::new(x - 0.12, -0.26, 0.38), Vec3::new(x + 0.12, -0.25, 0.42));
    }
    for _ in 0..((hx * hy * 1.5) as usize).min(60) {
        let p = Vec3::new(b.r.gen_range(-hx..hx), b.r.gen_range(-hy..hy), 0.03);
        if p.y.abs() < 0.5 {
            continue;
        }
        let c = [ids::ROOF, ids::STUCCO, ids::PLASTER][b.r.gen_range(0..3)];
        b.kit.mat(ids::FOLIAGE).frustum(p, 0.02, 0.02, 0.3, 4, false);
        b.kit.mat(c).cuboid(p + Vec3::Z * 0.32, Vec3::splat(0.05), bevy::math::Quat::IDENTITY);
    }
}
