//! Debris built by the destruction passes: rubble chunks (with bent rebar
//! in Modern mode), rebar tufts on sheared tops, pancaked slabs, glass
//! shards, charred roof rafters, fallen column drums, talus piles, and the
//! siege clutter of carts, barrels and splintered beams.

use crate::generators::kit::{Kit, ids};
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

fn rand_rot(r: &mut ChaCha8Rng) -> Quat {
    Quat::from_euler(bevy::math::EulerRot::ZXY, r.gen_range(0.0..TAU), r.gen_range(-0.6..0.6), r.gen_range(-0.6..0.6))
}

/// A bent steel bar: up from `a`, then kinked sideways.
fn bar(kit: &mut Kit, r: &mut ChaCha8Rng, a: Vec3, len: f32) {
    let k = r.gen_range(0.0..TAU);
    let mid = a + Vec3::new(r.gen_range(-0.1..0.1), r.gen_range(-0.1..0.1), len * r.gen_range(0.4..0.7));
    let end = mid + Vec3::new(k.cos(), k.sin(), r.gen_range(-0.2..0.4)) * len * 0.5;
    kit.mat(ids::REBAR);
    kit.beam(a, mid, 0.035, 0.035);
    kit.beam(mid, end, 0.035, 0.035);
}

/// `n` chunks around `at` (on the ground) within `spread`.
pub fn rubble(kit: &mut Kit, r: &mut ChaCha8Rng, at: Vec3, spread: f32, n: usize, mat: u16, rebar: bool) {
    for _ in 0..n {
        let off = Vec2::new(r.gen_range(-1.0..1.0), r.gen_range(-1.0..1.0)) * spread;
        let half = Vec3::new(r.gen_range(0.15..0.55), r.gen_range(0.15..0.5), r.gen_range(0.1..0.35));
        let c = at + off.extend(half.z * 0.6);
        kit.mat(mat).cuboid(c, half, rand_rot(r));
        if rebar && r.gen_bool(0.45) {
            let len = r.gen_range(0.5..1.4);
            bar(kit, r, c + Vec3::Z * half.z * 0.5, len);
        }
    }
}

/// Bent rebar sticking out of a sheared concrete top.
pub fn rebar_tuft(kit: &mut Kit, r: &mut ChaCha8Rng, top: Vec3, half: Vec2, n: usize) {
    for _ in 0..n.max(1) {
        let off = Vec2::new(r.gen_range(-0.9..0.9) * half.x, r.gen_range(-0.9..0.9) * half.y);
        let len = r.gen_range(0.6..1.8);
        bar(kit, r, top + off.extend(-0.1), len);
    }
}

/// Collapsed floor slabs stacked at a building's base, each tilted.
pub fn slabs(kit: &mut Kit, r: &mut ChaCha8Rng, base: Vec3, half: Vec2, n: usize) {
    kit.mat(ids::FACADE);
    for i in 0..n {
        let tilt = Quat::from_euler(bevy::math::EulerRot::ZXY, r.gen_range(-0.2..0.2), r.gen_range(-0.12..0.12), r.gen_range(-0.12..0.12));
        let c = base + Vec3::new(r.gen_range(-0.4..0.4), r.gen_range(-0.4..0.4), 0.2 + i as f32 * 0.45);
        kit.cuboid(c, (half * r.gen_range(0.7..1.0)).extend(0.15), tilt);
    }
}

/// Jagged glass shards left around a shattered window's frame.
pub fn shards(kit: &mut Kit, corners: &[Vec3], r: &mut ChaCha8Rng) {
    let n = corners.len();
    if n < 3 {
        return;
    }
    let c = corners.iter().copied().sum::<Vec3>() / n as f32;
    kit.mat(ids::GLASS);
    for i in 0..n {
        let (a, b) = (corners[i], corners[(i + 1) % n]);
        if r.gen_bool(0.35) {
            continue;
        }
        let t = r.gen_range(0.2..0.8);
        let tip = (a + (b - a) * t).lerp(c, r.gen_range(0.15..0.45));
        kit.poly(&[a, b, tip]);
    }
}

/// Charred rafters (ID 9) where a burned roof slope was: a plate along the
/// eave, a ridge beam, and rafters between them, set just under the slope.
pub fn rafters(kit: &mut Kit, corners: &[Vec3], n: Vec3) {
    let k = corners.len();
    if k < 3 {
        return;
    }
    let inset = -n * 0.12;
    let p: Vec<Vec3> = corners.iter().map(|c| *c + inset).collect();
    kit.mat(ids::CHARRED);
    // Eave = the lowest edge; the opposite side is the ridge (or apex).
    let low = (0..k).min_by(|&i, &j| (p[i].z + p[(i + 1) % k].z).total_cmp(&(p[j].z + p[(j + 1) % k].z))).unwrap_or(0);
    let (a, b) = (p[low], p[(low + 1) % k]);
    let (d, c) = if k == 4 { (p[(low + 3) % 4], p[(low + 2) % 4]) } else { let apex = p.iter().copied().max_by(|x, y| x.z.total_cmp(&y.z)).unwrap_or(a); (apex, apex) };
    kit.beam(a, b, 0.18, 0.22);
    if c.distance(d) > 0.1 {
        kit.beam(d, c, 0.2, 0.25);
    }
    let count = (a.distance(b) / 0.7).floor().max(1.0) as usize;
    for i in 0..=count {
        let t = i as f32 / count as f32;
        // Some rafters burned through: every third one stops short.
        let end = d.lerp(c, t);
        let start = a.lerp(b, t);
        let end = if i % 3 == 1 { start.lerp(end, 0.6) } else { end };
        kit.beam(start, end, 0.12, 0.16);
    }
}

/// Column drums lying on the ground along `dir` from the snapped base.
pub fn drums(kit: &mut Kit, base: Vec3, dir: Vec2, rad: f32, len: f32) {
    let rad = rad.max(0.1);
    let n = (len / (rad * 2.2)).ceil().clamp(1.0, 12.0) as usize;
    let piece = len / n as f32;
    kit.mat(ids::FACADE);
    for i in 0..n {
        let s = rad * 1.8 + i as f32 * (piece + 0.25);
        let a = (base.truncate() + dir * s).extend(base.z + rad);
        let wob = Vec2::new(-dir.y, dir.x) * (i as f32 * 0.7).sin() * 0.3;
        kit.tube(a + wob.extend(0.0), a + (dir * piece * 0.95 + wob).extend(0.0), rad, 12);
    }
}

/// A talus pile of fallen stone at a breach's foot.
pub fn talus(kit: &mut Kit, r: &mut ChaCha8Rng, at: Vec3, size: f32, density: f32) {
    kit.mat(ids::FACADE).frustum(at, size * 1.4, 0.0, size, 7, true);
    let n = (3.0 + density * 8.0) as usize;
    rubble(kit, r, at, size * 1.3, n, ids::FACADE, false);
}

/// Carts, barrels and splintered beams strewn around a siege zone.
pub fn siege_clutter(kit: &mut Kit, r: &mut ChaCha8Rng, at: Vec3, spread: f32) {
    let pos = |r: &mut ChaCha8Rng| at + Vec3::new(r.gen_range(-1.0..1.0) * spread, r.gen_range(-1.0..1.0) * spread, 0.0);
    for _ in 0..r.gen_range(1..4) {
        let c = pos(r);
        kit.mat(ids::TRIM).frustum(c, 0.32, 0.32, 0.85, 10, true);
        kit.mat(ids::IRON).frustum(c + Vec3::Z * 0.15, 0.335, 0.335, 0.06, 10, false);
        kit.frustum(c + Vec3::Z * 0.64, 0.335, 0.335, 0.06, 10, false);
    }
    // A broken cart: tilted bed, one wheel off.
    let c = pos(r);
    let yaw = r.gen_range(0.0..TAU);
    let rot = Quat::from_rotation_z(yaw) * Quat::from_rotation_x(0.18);
    kit.mat(ids::TRIM).cuboid(c + Vec3::Z * 0.55, Vec3::new(1.1, 0.6, 0.08), rot);
    let side = Quat::from_rotation_z(yaw) * Vec3::Y;
    kit.tube(c + side * 0.62 + Vec3::Z * 0.45, c + side * 0.72 + Vec3::Z * 0.45, 0.45, 12);
    let flat = c - side * 1.2 + Vec3::X * 0.4;
    kit.tube(flat, flat + Vec3::Z * 0.1, 0.45, 12);
    // Splintered beams.
    for _ in 0..r.gen_range(2..5) {
        let a = pos(r) + Vec3::Z * 0.12;
        let ang = r.gen_range(0.0..TAU);
        let len = r.gen_range(1.2..3.5);
        let b = a + Vec3::new(ang.cos(), ang.sin(), r.gen_range(0.0..0.3)) * len;
        kit.beam(a, b, 0.22, 0.2);
        let tip = b + Vec3::new((ang + FRAC_PI_2).cos(), (ang + FRAC_PI_2).sin(), 0.1) * 0.25;
        kit.poly(&[b - Vec3::Z * 0.1, b + Vec3::Z * 0.1, tip]);
    }
}
