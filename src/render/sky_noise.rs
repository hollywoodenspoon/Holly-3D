//! Value noise helpers for the procedural sky (clouds, stars, nebulae).

use bevy::math::Vec3;

pub(super) fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(super) fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

/// Smooth 3D value noise in 0..1.
fn vnoise(p: Vec3) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec3::splat(3.0) - f * 2.0);
    let (x, y, z) = (i.x as i32, i.y as i32, i.z as i32);
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c = |dx, dy, dz| hash3(x + dx, y + dy, z + dz);
    l(
        l(l(c(0, 0, 0), c(1, 0, 0), u.x), l(c(0, 1, 0), c(1, 1, 0), u.x), u.y),
        l(l(c(0, 0, 1), c(1, 0, 1), u.x), l(c(0, 1, 1), c(1, 1, 1), u.x), u.y),
        u.z,
    )
}

pub(super) fn fbm(p: Vec3, octaves: u32) -> f32 {
    let (mut s, mut a, mut f, mut n) = (0.0, 0.5, 1.0, 0.0);
    for _ in 0..octaves {
        s += vnoise(p * f) * a;
        n += a;
        a *= 0.5;
        f *= 2.03;
    }
    s / n
}
