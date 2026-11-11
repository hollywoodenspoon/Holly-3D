//! Directional generators: anisotropic wood grain, brushed metal, diamond
//! tread plate, woven threads, chainmail rings, carbon-fibre twill and
//! scratches.
//!
//! Wood grain, tread plate, weave and scratches are ported from
//! MaterialCraft (https://github.com/storytold/materialcraft:
//! `wood_grain` in `noises_more.rs`, `tread_plate` in `patterns_more.rs`,
//! `weave` and `scratches` in `patterns.rs`, before the old procedural
//! library was removed), copyright (c) 2026 ArtCraft Team and the
//! MaterialCraft contributors, MIT OR Apache-2.0. Brushed metal, chainmail
//! and carbon fibre are built here on the same lattice.

use super::generators::Px;
use super::lattice::{fbm_xy, fiber, rand2};
use std::f32::consts::{PI, TAU};

/// Wood grain: bright wood with dark rings. The noise is stretched
/// `stretch` times along V, so the rings run along the board.
pub fn wood_grain(u: f32, v: f32, rings: f32, stretch: f32, sharp: f32, fibres: f32, seed: u32) -> f32 {
    let n = fbm_xy(u, v, stretch, 1.0, 4, 0.45, seed);
    let ring = (n * rings).rem_euclid(1.0).powf(sharp);
    let f = fiber(u, v, 160.0, 2.0, 8.0, seed ^ 0x3b);
    ((1.0 - ring * 0.65) * (1.0 - f * fibres * 0.35)).clamp(0.0, 1.0)
}

/// Brushed metal: `streaks` fine lines across V, each `length` cells long
/// along U, cut `depth` deep around mid-grey.
pub fn brushed(u: f32, v: f32, streaks: f32, length: f32, sharp: f32, depth: f32, seed: u32) -> f32 {
    let n = fbm_xy(u, v, length, streaks, 3, 0.5, seed);
    let f = fiber(u, v, length * 2.0, streaks, sharp.max(0.5) * 4.0, seed ^ 0x5d);
    (0.5 + ((n - 0.5) * 1.2 + f * 0.3 - 0.15) * depth).clamp(0.0, 1.0)
}

fn rot(x: f32, y: f32, turns: f32) -> (f32, f32) {
    let (s, c) = (turns * TAU).sin_cos();
    (x * c - y * s, x * s + y * c)
}

/// Diamond tread plate: raised lenses that alternate ±45° in a checker
/// (`t` must be even to tile).
pub fn diamond_plate(u: f32, v: f32, t: f32, length: f32, width: f32) -> f32 {
    let (cx, cy) = ((u * t).floor() as i32, (v * t).floor() as i32);
    let (x, y) = ((u * t).rem_euclid(1.0) - 0.5, (v * t).rem_euclid(1.0) - 0.5);
    let (x, y) = rot(x, y, if (cx + cy) & 1 == 0 { 0.125 } else { -0.125 });
    // An elongated lens with a flat top and rounded flanks.
    let d = ((x / (length * 0.5).max(1e-3)).powi(2) + (y / (width.max(0.01) * 0.5)).powi(2)).sqrt();
    (1.0 - d.powi(4)).max(0.0).sqrt()
}

/// Plain weave: `n` threads each way (even, to tile), `w` thread width.
/// The id tells warp from weft, for two-tone cloth.
pub fn weave(u: f32, v: f32, n: f32, w: f32) -> Px {
    let w = w.clamp(0.01, 1.0);
    let (x, y) = (u * n, v * n);
    let (cx, cy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    // Round thread profiles: horizontal across the cell row, vertical down the column.
    let round = |f: f32| if (f - 0.5).abs() * 2.0 <= w { (1.0 - ((f - 0.5) * 2.0 / w).powi(2)).max(0.0).sqrt() } else { 0.0 };
    // The thread on top bulges along its length; which is on top alternates in a checker.
    let hb = round(fy) * (0.6 + 0.4 * (fx * PI).sin());
    let vb = round(fx) * (0.6 + 0.4 * (fy * PI).sin());
    if (cx + cy) & 1 == 0 { Px { h: hb.max(vb * 0.6), id: 0.25 } } else { Px { h: vb.max(hb * 0.6), id: 0.75 } }
}

/// Chainmail: rows of interlocked wire rings, every other row shifted by
/// half a ring (`n` even, to tile). `ring` is the ring radius in cells × 2.
pub fn chainmail(u: f32, v: f32, n: f32, wire: f32, ring: f32) -> f32 {
    let (x, y) = (u * n, v * n);
    let (r, w) = (ring * 0.5, wire.max(0.01));
    let mut best = 0.0f32;
    for dj in -1..=1 {
        let j = y.floor() as i32 + dj;
        let shift = if j & 1 == 0 { 0.0 } else { 0.5 };
        for di in -1..=1 {
            let cx = (x - shift).floor() + di as f32 + 0.5 + shift;
            let (dx, dy) = (x - cx, y - (j as f32 + 0.5));
            let d = ((dx * dx + dy * dy).sqrt() - r).abs();
            if d < w {
                // Each ring tilts, so one side passes over its neighbour and the other under.
                let tilt = 0.7 + 0.3 * (dy / r.max(1e-3)).clamp(-1.0, 1.0) * if j & 1 == 0 { 1.0 } else { -1.0 };
                best = best.max((1.0 - (d / w).powi(2)).sqrt() * tilt);
            }
        }
    }
    best
}

/// Carbon fibre in a 2 / 2 twill: tows float over two and under two,
/// stepping one cell per row. `tows` is the number of repeats (4 cells
/// each) across the tile.
pub fn carbon(u: f32, v: f32, tows: f32, fibres: f32, gap: f32) -> Px {
    let n = tows * 4.0;
    let (x, y) = (u * n, v * n);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let m = (x.floor() as i32 + y.floor() as i32).rem_euclid(4);
    let w = (1.0 - gap).clamp(0.05, 1.0);
    // `across` the tow gives its round profile and fibres; `along` its two-cell float.
    let tow = |across: f32, along: f32| {
        let p = (across - 0.5) * 2.0 / w;
        if p.abs() > 1.0 {
            return 0.0;
        }
        (1.0 - p * p).sqrt() * (0.85 + 0.15 * (across * fibres * TAU).cos()) * (0.7 + 0.3 * (along * PI).sin())
    };
    if m < 2 { Px { h: tow(fy, (m as f32 + fx) * 0.5), id: 0.3 } } else { Px { h: tow(fx, ((m - 2) as f32 + fy) * 0.5), id: 0.7 } }
}

/// Scratches: two straight scratches per lattice cell (`n` cells across),
/// mostly along U, turned by up to `spread` × 90°. 1 on a scratch.
pub fn scratches(u: f32, v: f32, n: f32, length: f32, width: f32, spread: f32, seed: u32) -> f32 {
    let (x, y) = (u * n, v * n);
    let (ix, iy, ni) = (x.floor() as i32, y.floor() as i32, n as i32);
    let w = (width * n).max(1e-3);
    let mut best = 0.0f32;
    for j in -1..=1 {
        for i in -1..=1 {
            let (cx, cy) = (ix + i, iy + j);
            let (wx, wy) = (cx.rem_euclid(ni), cy.rem_euclid(ni));
            for k in 0..2u32 {
                let r = |q: u32| rand2(wx, wy, seed ^ (k * 97 + q).wrapping_mul(0x9e37_79b9));
                let a = (r(3) - 0.5) * spread * PI;
                let l = length.clamp(0.05, 1.0) * (0.5 + 0.5 * r(4));
                let (dx, dy) = (a.cos() * l, a.sin() * l);
                let (qx, qy) = (x - (cx as f32 + r(1)), y - (cy as f32 + r(2)));
                let t = ((qx * dx + qy * dy) / (l * l)).clamp(0.0, 1.0);
                let dist = ((qx - dx * t).powi(2) + (qy - dy * t).powi(2)).sqrt();
                if dist < w {
                    best = best.max((1.0 - dist / w) * (0.4 + 0.6 * r(5)) * (1.0 - (t * 2.0 - 1.0).abs().powi(4)));
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mean absolute change of `f` one small step along U and along V.
    fn change(f: impl Fn(f32, f32) -> f32) -> (f32, f32) {
        let (mut du, mut dv) = (0.0, 0.0);
        for i in 0..2000 {
            let (u, v) = (i as f32 * 0.618 % 1.0, i as f32 * 0.137 % 1.0);
            du += (f(u + 0.002, v) - f(u, v)).abs();
            dv += (f(u, v + 0.002) - f(u, v)).abs();
        }
        (du, dv)
    }

    #[test]
    fn wood_and_brushed_metal_are_anisotropic() {
        // Wood rings run along V, so the pattern changes faster across U.
        let (du, dv) = change(|u, v| wood_grain(u, v, 10.0, 8.0, 2.0, 0.0, 1));
        assert!(du > dv * 2.0, "wood: {du} vs {dv}");
        // Brushed streaks run along U.
        let (du, dv) = change(|u, v| brushed(u, v, 160.0, 2.0, 2.0, 0.6, 1));
        assert!(dv > du * 4.0, "brushed: {du} vs {dv}");
    }

    #[test]
    fn woven_patterns_alternate_and_rings_are_hollow() {
        // Neighbouring weave cells put the other thread on top.
        assert_ne!(weave(0.03, 0.03, 16.0, 0.85).id, weave(0.03 + 1.0 / 16.0, 0.03, 16.0, 0.85).id);
        let c = carbon(0.01, 0.01, 6.0, 6.0, 0.08);
        assert_ne!(c.id, carbon(0.01 + 2.0 / 24.0, 0.01, 6.0, 6.0, 0.08).id, "two over, two under");
        // A ring's centre is empty, its wire is not.
        let n = 12.0;
        assert_eq!(chainmail(0.5 / n, 0.5 / n, n, 0.12, 0.6), 0.0);
        assert!(chainmail((0.5 + 0.3) / n, 0.5 / n, n, 0.12, 0.6) > 0.5);
        assert!(diamond_plate(0.5 / 8.0, 0.5 / 8.0, 8.0, 0.7, 0.22) > 0.9);
        let hits = (0..4000).filter(|i| scratches(*i as f32 * 0.618 % 1.0, *i as f32 * 0.137 % 1.0, 8.0, 0.6, 0.012, 0.25, 1) > 0.0).count();
        assert!(hits > 40 && hits < 2500, "{hits}");
    }
}
