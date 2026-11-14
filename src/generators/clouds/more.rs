//! Clouds PR 5: the type rules for Altocumulus, Mammatus, Lenticular and
//! Cloud Bank / Wall. Like the first four, each lays body puffs (level 0)
//! with the base at z = 0; detail puffs and wind shear are added after.

use super::puffs::{Ctx, Puff};
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use std::f32::consts::TAU;

/// A fine sheet of small puffs in slightly wavy rows, with gaps.
pub(super) fn altocumulus(cx: &mut Ctx) {
    let (w, h) = (cx.p.size.max(1.0), cx.p.height.max(1.0));
    let l = w * 0.6;
    let (cols, rows) = (12usize, 7usize);
    let cell = w / cols as f32;
    let wave = cx.r.gen_range(0.0..TAU);
    for j in 0..rows {
        for i in 0..cols {
            if cx.r.gen_range(0.0f32..1.0) < 0.12 {
                continue;
            }
            let r = cell * 0.45 * cx.lump(0.6);
            let top = h * cx.r.gen_range(0.8..1.0);
            let sq = (top * 0.5 / r).min(1.0);
            let x = (i as f32 + 0.5) / cols as f32 * w - w * 0.5 + cx.r.gen_range(-0.15..0.15) * cell;
            let y0 = (j as f32 + 0.5) / rows as f32 * l - l * 0.5;
            let y = y0 + (x / w * TAU * 1.5 + wave).sin() * cell * 0.3;
            cx.out.push(Puff { c: Vec3::new(x, y, r * sq), r, sq, level: 0 });
        }
    }
}

/// A flat anvil layer on top, with rounded pouches hanging below it.
pub(super) fn mammatus(cx: &mut Ctx) {
    let (w, h) = (cx.p.size.max(1.0), cx.p.height.max(1.0));
    let cols = 8usize;
    let cell = w / cols as f32;
    // The layer: 0.6 h .. h.
    for j in 0..cols - 2 {
        for i in 0..cols {
            let r = cell * 0.62 * cx.lump(0.4);
            let sq = (h * 0.2 / r).min(1.0);
            let x = (i as f32 + 0.5) * cell - w * 0.5;
            let y = (j as f32 + 0.5) * cell - (cols - 2) as f32 * cell * 0.5;
            let z = h * 0.8 + cx.r.gen_range(-0.03..0.03) * h;
            cx.out.push(Puff { c: Vec3::new(x, y, z), r, sq, level: 0 });
        }
    }
    // Pouches on an offset grid, each joined to the layer by a neck puff.
    for j in 0..cols - 3 {
        for i in 0..cols - 1 {
            if cx.r.gen_range(0.0f32..1.0) < 0.1 {
                continue;
            }
            let r = cell * 0.4 * cx.lump(0.5);
            let sq = 1.1;
            let x = (i as f32 + 1.0) * cell - w * 0.5 + cx.r.gen_range(-0.1..0.1) * cell;
            let y = (j as f32 + 1.0) * cell - (cols - 2) as f32 * cell * 0.5;
            let low = r * sq + cx.r.gen_range(0.0..0.12) * h;
            cx.out.push(Puff { c: Vec3::new(x, y, low), r, sq, level: 0 });
            let neck = (low + h * 0.62) * 0.5;
            cx.out.push(Puff { c: Vec3::new(x, y, neck), r: r * 0.85, sq: 1.0, level: 0 });
        }
    }
}

/// Two or three smooth lens discs stacked with gaps, each smaller than the
/// one below.
pub(super) fn lenticular(cx: &mut Ctx) {
    let (w, h) = (cx.p.size.max(1.0), cx.p.height.max(1.0));
    let n = 3usize;
    let rings = 5usize;
    let lean = Vec2::from_angle(cx.r.gen_range(0.0..TAU)) * w * 0.03;
    for k in 0..n {
        let big = w * 0.5 * (1.0 - 0.18 * k as f32);
        let half = h / n as f32 * 0.3;
        let zc = (k as f32 + 0.5) * h / n as f32;
        let r = big / rings as f32 * 0.9;
        let at = lean * k as f32;
        for m in 0..=rings {
            let rho = big * m as f32 / rings as f32 * 0.92;
            let count = (6 * m).max(1);
            let phase = cx.r.gen_range(0.0..TAU);
            // Thickest in the middle, thin at the rim.
            let thick = half * (1.0 - (rho / big).powi(2)).max(0.05).sqrt();
            for q in 0..count {
                let a = phase + TAU * q as f32 / count as f32;
                let xy = at + Vec2::from_angle(a) * rho;
                let sq = (thick / r).max(0.06) * cx.lump(0.15);
                cx.out.push(Puff { c: xy.extend(zc), r, sq, level: 0 });
            }
        }
    }
}

/// A long ridge of cumulus towers along X, of varying height, piled close.
pub(super) fn bank(cx: &mut Ctx) {
    let (len, h) = (cx.p.size.max(1.0), cx.p.height.max(1.0));
    let half_w = len * 0.11;
    let n = 9usize;
    for i in 0..n {
        let x = (i as f32 + 0.5) / n as f32 * len - len * 0.5 + cx.r.gen_range(-0.2..0.2) * len / n as f32;
        // The middle tower is the tallest, so the bank reaches its height.
        let top = if i == n / 2 { h } else { h * cx.r.gen_range(0.7..0.95) };
        let k = 5usize;
        for j in 0..k {
            let t = j as f32 / (k - 1) as f32;
            let r = (half_w * (1.0 - 0.45 * t) * cx.lump(0.6)).max(half_w * 0.2);
            let z = r + (top - 2.0 * r).max(0.0) * t;
            let xy = Vec2::new(x, 0.0) + if j == 0 { Vec2::ZERO } else { cx.polar(half_w * 0.4) };
            cx.out.push(Puff { c: xy.extend(z), r, sq: 1.0, level: 0 });
        }
    }
}
