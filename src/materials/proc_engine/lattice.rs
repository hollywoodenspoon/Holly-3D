//! Tileable noise on a whole-number lattice: integer hashing, value and
//! gradient (Perlin) noise, fractal sums and cellular (Worley) noise. Every
//! function takes coordinates in cells and a period in cells, so whole-number
//! scales repeat seamlessly across the 0..1 UV square.
//!
//! Ported from MaterialCraft (https://github.com/storytold/materialcraft,
//! `crates/nodes/src/noise.rs` and `noises_more.rs`), copyright (c) 2026
//! ArtCraft Team and the MaterialCraft contributors, MIT OR Apache-2.0.

use std::f32::consts::{SQRT_2, TAU};

/// A 32-bit integer hash (lowbias32 by Chris Wellons, public domain).
#[inline]
pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

#[inline]
pub fn hash3(x: i32, y: i32, seed: u32) -> u32 {
    hash((x as u32).wrapping_mul(0x8da6_b343) ^ hash((y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f)))
}

/// Uniform 0..1 from a lattice point.
#[inline]
pub fn rand2(x: i32, y: i32, seed: u32) -> f32 {
    (hash3(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn wrap(i: i32, p: i32) -> i32 {
    if p <= 0 { i } else { i.rem_euclid(p) }
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in 0..1, lattice period `p` cells.
pub fn value(x: f32, y: f32, p: i32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (fade(x - x0), fade(y - y0));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let r = |i: i32, j: i32| rand2(wrap(i, p), wrap(j, p), seed);
    let a = r(ix, iy) + (r(ix + 1, iy) - r(ix, iy)) * tx;
    let b = r(ix, iy + 1) + (r(ix + 1, iy + 1) - r(ix, iy + 1)) * tx;
    a + (b - a) * ty
}

/// Gradient (Perlin) noise in about -1..1 with a lattice period per axis,
/// so stretched noise still tiles.
pub fn perlin_xy(x: f32, y: f32, px: i32, py: i32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (ix, iy) = (x0 as i32, y0 as i32);
    let g = |i: i32, j: i32, dx: f32, dy: f32| {
        let a = (hash3(wrap(i, px), wrap(j, py), seed) >> 8) as f32 * (TAU / (1u32 << 24) as f32);
        a.cos() * dx + a.sin() * dy
    };
    let (u, v) = (fade(fx), fade(fy));
    let n00 = g(ix, iy, fx, fy);
    let n10 = g(ix + 1, iy, fx - 1.0, fy);
    let n01 = g(ix, iy + 1, fx, fy - 1.0);
    let n11 = g(ix + 1, iy + 1, fx - 1.0, fy - 1.0);
    let a = n00 + (n10 - n00) * u;
    let b = n01 + (n11 - n01) * u;
    (a + (b - a) * v) * SQRT_2
}

/// Gradient (Perlin) noise in about -1..1, lattice period `p` cells.
pub fn perlin(x: f32, y: f32, p: i32, seed: u32) -> f32 {
    perlin_xy(x, y, p, p, seed)
}

/// Fractal sum of `f(level multiplier, octave)` (each 0..1): `octaves`
/// layers, each twice as fine and `gain` times weaker. Normalised to 0..1.
pub fn fractal(octaves: u32, gain: f32, f: impl Fn(f32, u32) -> f32) -> f32 {
    let (mut sum, mut amp, mut norm, mut m) = (0.0, 1.0, 0.0, 1.0);
    for o in 0..octaves.clamp(1, 10) {
        sum += f(m, o) * amp;
        norm += amp;
        amp *= gain;
        m *= 2.0;
    }
    sum / f32::max(norm, 1e-6)
}

/// Fractal Perlin noise in 0..1 from `scale` cells. Tiles when `scale` is whole
/// (it is rounded to a whole number here).
pub fn fbm(u: f32, v: f32, scale: f32, octaves: u32, gain: f32, seed: u32) -> f32 {
    let s = scale.max(1.0).round();
    fractal(octaves, gain, |m, o| perlin(u * s * m, v * s * m, (s * m) as i32, seed.wrapping_add(o * 1013)) * 0.5 + 0.5)
}

/// Stretched fractal Perlin noise in 0..1 with `sx` × `sy` base cells.
pub fn fbm_xy(u: f32, v: f32, sx: f32, sy: f32, octaves: u32, gain: f32, seed: u32) -> f32 {
    let (sx, sy) = (sx.max(1.0).round(), sy.max(1.0).round());
    fractal(octaves, gain, |m, o| {
        let (px, py) = (sx * m, sy * m);
        perlin_xy(u * px, v * py, px as i32, py as i32, seed.wrapping_add(o * 977)) * 0.5 + 0.5
    })
}

/// Ridged (creased) fractal: `1 - |perlin|` layers, 0..1.
pub fn ridged(u: f32, v: f32, scale: f32, octaves: u32, gain: f32, seed: u32) -> f32 {
    let s = scale.max(1.0).round();
    fractal(octaves, gain, |m, o| {
        let n = 1.0 - perlin(u * s * m, v * s * m, (s * m) as i32, seed.wrapping_add(o * 7919)).abs();
        n * n
    })
}

/// A thin bright line per stretched-noise zero crossing (fibres, grain).
pub fn fiber(u: f32, v: f32, along: f32, across: f32, sharp: f32, seed: u32) -> f32 {
    (1.0 - perlin_xy(u * along, v * across, along as i32, across as i32, seed).abs()).powf(sharp)
}

/// A cellular lookup: nearest and second-nearest feature distances (in
/// cells) and the nearest feature's random id (0..1).
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub f1: f32,
    pub f2: f32,
    pub id: f32,
}

/// Worley noise with one jittered feature point per cell (`jitter` 0..1),
/// period `p` cells.
pub fn cellular(x: f32, y: f32, p: i32, jitter: f32, seed: u32) -> Cell {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (mut f1, mut f2, mut id) = (f32::MAX, f32::MAX, 0.0);
    for j in -2..=2 {
        for i in -2..=2 {
            let (cx, cy) = (ix + i, iy + j);
            let (wx, wy) = (wrap(cx, p), wrap(cy, p));
            let px = cx as f32 + 0.5 + (rand2(wx, wy, seed) - 0.5) * jitter;
            let py = cy as f32 + 0.5 + (rand2(wx, wy, seed ^ 0x5bd1_e995) - 0.5) * jitter;
            let d = ((px - x) * (px - x) + (py - y) * (py - y)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
                id = rand2(wx, wy, seed ^ 0x2545_f491);
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    Cell { f1, f2, id }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Samples just inside both edges of the tile must agree.
    fn tiles(f: impl Fn(f32, f32) -> f32) {
        let e = 1e-4;
        for k in 0..32 {
            let t = k as f32 / 32.0 + 0.013;
            assert!((f(e, t) - f(1.0 - e, t)).abs() < 0.02, "u seam at v = {t}");
            assert!((f(t, e) - f(t, 1.0 - e)).abs() < 0.02, "v seam at u = {t}");
        }
    }

    #[test]
    fn whole_number_scales_tile_seamlessly_and_stay_in_range() {
        tiles(|u, v| value(u * 5.0, v * 5.0, 5, 3));
        tiles(|u, v| perlin(u * 4.0, v * 4.0, 4, 1));
        tiles(|u, v| fbm(u, v, 3.0, 5, 0.5, 9));
        tiles(|u, v| fbm_xy(u, v, 1.0, 12.0, 4, 0.45, 2));
        tiles(|u, v| ridged(u, v, 4.0, 4, 0.5, 7));
        tiles(|u, v| cellular(u * 6.0, v * 6.0, 6, 1.0, 5).f1);
        // Ids jump at cell borders, so check that they repeat exactly one tile away.
        for k in 0..64 {
            let (x, y) = (k as f32 * 0.37 % 6.0, k as f32 * 0.61 % 6.0);
            assert_eq!(cellular(x, y, 6, 1.0, 5).id, cellular(x + 6.0, y - 6.0, 6, 1.0, 5).id);
        }
        for k in 0..400 {
            let (u, v) = (k as f32 * 0.137 % 1.0, k as f32 * 0.291 % 1.0);
            for x in [value(u * 7.0, v * 7.0, 7, 1), fbm(u, v, 4.0, 5, 0.5, 2), ridged(u, v, 4.0, 4, 0.5, 2)] {
                assert!((0.0..=1.0).contains(&x), "{x}");
            }
            let c = cellular(u * 5.0, v * 5.0, 5, 1.0, 4);
            assert!(c.f1 <= c.f2 && c.f1 >= 0.0 && (0.0..=1.0).contains(&c.id));
        }
    }

    #[test]
    fn seeds_change_the_noise_and_the_same_seed_repeats_it() {
        let a = fbm(0.31, 0.62, 4.0, 4, 0.5, 1);
        assert_eq!(a, fbm(0.31, 0.62, 4.0, 4, 0.5, 1));
        assert_ne!(a, fbm(0.31, 0.62, 4.0, 4, 0.5, 2));
        assert_ne!(rand2(1, 2, 3), rand2(2, 1, 3));
    }
}
