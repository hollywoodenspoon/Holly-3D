//! Shared pieces for the industrial and utility prefabs: post-and-wire
//! fences, storage tanks, silos, pipes and pipe racks, ladders, smoke
//! stacks, lattice towers and loading-dock doors.

use crate::generators::kit::{Kit, ids};
use bevy::math::{Vec2, Vec3};

/// A post-and-wire security fence around `min`..`max`, `h` tall, posts every ~3 m,
/// with a gate gap `gate` wide centred on the front (-Y) side.
pub(super) fn fence(k: &mut Kit, min: Vec2, max: Vec2, h: f32, gate: f32) {
    let c = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
    let mid = (min.x + max.x) * 0.5;
    for i in 0..4 {
        let (a, b) = (c[i], c[(i + 1) % 4]);
        let runs: Vec<(Vec2, Vec2)> = if i == 0 && gate > 0.0 {
            vec![(a, Vec2::new(mid - gate * 0.5, a.y)), (Vec2::new(mid + gate * 0.5, a.y), b)]
        } else {
            vec![(a, b)]
        };
        for (a, b) in runs {
            let len = a.distance(b);
            if len < 0.2 {
                continue;
            }
            k.mat(ids::REBAR);
            let n = ((len / 3.0).ceil() as usize).max(1);
            for j in 0..=n {
                let p = a.lerp(b, j as f32 / n as f32);
                k.block((p - Vec2::splat(0.05)).extend(0.0), (p + Vec2::splat(0.05)).extend(h + 0.1));
            }
            // Wire rails: see-through from any side.
            for t in [0.1, 0.4, 0.7, 1.0] {
                k.beam(a.extend(h * t), b.extend(h * t), 0.03, 0.03);
            }
        }
    }
}

/// A storage tank at `c`: a cylinder `r` wide and `h` tall with a low cone
/// roof, a rim band and (when `ladder`) a caged ladder up one side.
pub(super) fn tank(k: &mut Kit, c: Vec3, r: f32, h: f32, wall: u16, ladder_on: bool) {
    let sides = if r > 6.0 { 24 } else { 16 };
    k.mat(wall).frustum(c, r, r, h, sides, true);
    k.mat(ids::IRON).frustum(c + Vec3::Z * (h - 0.3), r + 0.06, r + 0.06, 0.3, sides, false);
    k.mat(ids::ROOF).frustum(c + Vec3::Z * h, r, 0.0, r * 0.18, sides, false);
    if ladder_on {
        ladder(k, c + Vec3::new(0.0, -r - 0.35, 0.0), h + 1.0);
    }
}

/// A silo: tall cylinder with a cone cap and a dome-ish ring at the top.
pub(super) fn silo(k: &mut Kit, c: Vec3, r: f32, h: f32, wall: u16) {
    k.mat(wall).frustum(c, r, r, h, 16, false);
    for i in 1..((h / 4.0) as usize) {
        k.mat(ids::IRON).frustum(c + Vec3::Z * (i as f32 * 4.0), r + 0.04, r + 0.04, 0.12, 16, false);
    }
    k.mat(ids::ROOF).frustum(c + Vec3::Z * h, r + 0.1, 0.35, r * 0.5, 16, true);
}

/// A straight ladder with side rails up from `p` (on a wall facing -Y).
pub(super) fn ladder(k: &mut Kit, p: Vec3, h: f32) {
    k.mat(ids::IRON);
    for s in [-0.25, 0.25] {
        k.block(p + Vec3::new(s - 0.03, -0.03, 0.0), p + Vec3::new(s + 0.03, 0.03, h));
    }
    let n = (h / 0.6) as usize;
    for i in 1..n {
        let z = i as f32 * 0.6;
        k.block(p + Vec3::new(-0.25, -0.02, z), p + Vec3::new(0.25, 0.02, z + 0.04));
    }
}

/// A pipe from `a` to `b` with flanges at both ends.
pub(super) fn pipe(k: &mut Kit, a: Vec3, b: Vec3, r: f32) {
    k.mat(ids::IRON).tube(a, b, r, 10);
    let d = (b - a).normalize_or_zero() * 0.12;
    k.tube(a, a + d, r * 1.35, 10);
    k.tube(b - d, b, r * 1.35, 10);
}

/// A pipe rack: a row of portal frames from `a` to `b` at height `h`
/// carrying `n` pipes along it.
pub(super) fn pipe_rack(k: &mut Kit, a: Vec2, b: Vec2, h: f32, n: usize) {
    let d = (b - a).normalize_or_zero();
    let side = d.perp();
    let len = a.distance(b);
    let bays = ((len / 6.0).ceil() as usize).max(1);
    k.mat(ids::RUST);
    for i in 0..=bays {
        let p = a.lerp(b, i as f32 / bays as f32);
        for s in [-1.5f32, 1.5] {
            let q = p + side * s;
            k.block((q - Vec2::splat(0.15)).extend(0.0), (q + Vec2::splat(0.15)).extend(h));
        }
        k.beam((p - side * 1.6).extend(h), (p + side * 1.6).extend(h), 0.25, 0.3);
    }
    for j in 0..n {
        let o = side * (-1.2 + 2.4 * (j as f32 + 0.5) / n as f32);
        let r = 0.18 + 0.06 * (j % 3) as f32;
        k.mat(ids::IRON).tube((a + o).extend(h + 0.15 + r), (b + o).extend(h + 0.15 + r), r, 8);
    }
}

/// A tapered smoke stack at `c`, `h` tall, with banding near the top.
pub(super) fn stack(k: &mut Kit, c: Vec3, r: f32, h: f32, wall: u16) {
    k.mat(wall).frustum(c, r, r * 0.7, h, 14, true);
    for t in [0.86, 0.93] {
        let rr = r * (1.0 - 0.3 * t) + 0.05;
        k.mat(ids::TRIM).frustum(c + Vec3::Z * (h * t), rr, rr, 1.0, 14, false);
    }
}

/// A four-legged lattice tower at `c`: `base` half-width at the ground
/// tapering to `top` at height `h`, with X bracing every `step` metres.
pub(super) fn lattice(k: &mut Kit, c: Vec3, base: f32, top: f32, h: f32, step: f32, w: f32) {
    let corners = [Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)];
    // Lift by the member size so sloped members don't dip below the base.
    let (c, h) = (c + Vec3::Z * w, h - w);
    let at = |i: usize, z: f32| {
        let half = base + (top - base) * (z / h);
        c + (corners[i % 4] * half).extend(z)
    };
    k.mat(ids::IRON);
    for i in 0..4 {
        k.beam(at(i, 0.0), at(i, h), w, w);
    }
    let n = ((h / step).round() as usize).max(1);
    for l in 0..n {
        let (z0, z1) = (h * l as f32 / n as f32, h * (l + 1) as f32 / n as f32);
        for i in 0..4 {
            k.beam(at(i, z1), at(i + 1, z1), w * 0.7, w * 0.7);
            k.beam(at(i, z0), at(i + 1, z1), w * 0.5, w * 0.5);
            k.beam(at(i + 1, z0), at(i, z1), w * 0.5, w * 0.5);
        }
    }
}

/// A loading-dock door on a wall facing -Y at `y`: a raised dock, a
/// roller door and a bumper pair.
pub(super) fn dock_door(k: &mut Kit, x: f32, y: f32, w: f32) {
    k.mat(ids::CURB).block(Vec3::new(x - w * 0.5 - 0.4, y - 1.2, 0.0), Vec3::new(x + w * 0.5 + 0.4, y, 1.2));
    k.mat(ids::RUST).quad(Vec3::new(x - w * 0.5, y - 0.03, 1.2), Vec3::new(x + w * 0.5, y - 0.03, 1.2), Vec3::new(x + w * 0.5, y - 0.03, 4.4), Vec3::new(x - w * 0.5, y - 0.03, 4.4));
    k.mat(ids::TRIM);
    for s in [-1.0, 1.0] {
        let px = x + s * (w * 0.5 + 0.2);
        k.block(Vec3::new(px - 0.15, y - 1.4, 0.6), Vec3::new(px + 0.15, y - 1.2, 1.2));
    }
    k.block(Vec3::new(x - w * 0.5 - 0.3, y - 0.6, 4.4), Vec3::new(x + w * 0.5 + 0.3, y, 4.6));
}

/// A heap of loose material (gravel, scrap, coal) at `c`.
pub(super) fn heap(k: &mut Kit, c: Vec3, r: f32, h: f32, mat: u16) {
    k.mat(mat).frustum(c, r, r * 0.25, h, 9, true);
}

/// A shipping container at `c` (centre of its base), turned by `yaw`.
pub(super) fn container(k: &mut Kit, c: Vec3, yaw: f32, paint: u16) {
    let (hx, hy, h) = (1.22, 3.05, 2.6);
    k.at(bevy::math::Mat4::from_translation(c) * bevy::math::Mat4::from_rotation_z(yaw), |k| {
        k.mat(paint).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
        k.mat(ids::IRON);
        for i in 0..7 {
            let y = -hy + 0.4 + i as f32 * (2.0 * hy - 0.8) / 6.0;
            for s in [-1.0, 1.0] {
                k.block(Vec3::new(s * hx - 0.04, y - 0.06, 0.1), Vec3::new(s * hx + 0.04, y + 0.06, h - 0.1));
            }
        }
    });
}
