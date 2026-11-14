//! Puff layout rules per cloud type, plus puff-on-puff detail.

use super::{CloudParams, CloudType};
use crate::generators::kit::rng;
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use std::f32::consts::TAU;

/// One puff: a sphere of radius `r` around `c`, squashed vertically by `sq`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    pub c: Vec3,
    pub r: f32,
    pub sq: f32,
    /// 0 = body puff, 1.. = detail level.
    pub level: u8,
}

impl Puff {
    /// Lowest and highest z of the puff.
    pub fn z_range(&self) -> (f32, f32) {
        (self.c.z - self.r * self.sq, self.c.z + self.r * self.sq)
    }
}

/// A cloud's puffs, base at z = 0.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CloudShape {
    pub puffs: Vec<Puff>,
}

impl CloudShape {
    /// Axis-aligned bounds of every puff.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for p in &self.puffs {
            let e = Vec3::new(p.r, p.r, p.r * p.sq);
            (lo, hi) = (lo.min(p.c - e), hi.max(p.c + e));
        }
        (lo, hi)
    }
    pub fn count_at(&self, level: u8) -> usize {
        self.puffs.iter().filter(|p| p.level == level).count()
    }
}

/// Body radius (fraction of the half-width) at normalised height `t`.
fn profile(kind: CloudType, t: f32) -> f32 {
    match kind {
        CloudType::Cumulus => 1.0 - 0.55 * t,
        CloudType::Congestus => 0.85 - 0.35 * t,
        CloudType::Cumulonimbus => 0.42 - 0.1 * t,
        CloudType::Stratocumulus => 1.0,
    }
}

struct Ctx<'a> {
    p: &'a CloudParams,
    r: rand_chacha::ChaCha8Rng,
    out: Vec<Puff>,
}

impl Ctx<'_> {
    fn lump(&mut self, k: f32) -> f32 {
        1.0 + self.p.lumpiness.clamp(0.0, 1.0) * k * self.r.gen_range(-0.5..0.5)
    }
    fn polar(&mut self, d: f32) -> Vec2 {
        let a = self.r.gen_range(0.0..TAU);
        Vec2::from_angle(a) * d * self.r.gen_range(0.0f32..1.0).sqrt()
    }
    /// A stack of `n` body puffs from z = 0 up to `top`, inside the profile.
    fn tower(&mut self, n: usize, t_max: f32, top: f32, scale: f32) {
        let half = self.p.size.max(1.0) * 0.5;
        for i in 0..n {
            let jitter = if i == 0 { 0.0 } else { self.r.gen_range(-0.3..0.3) };
            let t = ((i as f32 + 0.5 + jitter) / n as f32).clamp(0.0, 1.0) * t_max;
            let w = profile(self.p.kind, t) * half;
            let r = (w * scale * self.lump(0.8)).max(half * 0.05);
            let xy = if i == 0 { Vec2::ZERO } else { self.polar((w - r * 0.7).max(0.0)) };
            let z = r + (top - 2.0 * r).max(0.0) * (t / t_max.max(1e-3));
            self.out.push(Puff { c: xy.extend(z), r, sq: 1.0, level: 0 });
        }
    }
    /// The flat anvil spreading downwind near the top (Cumulonimbus).
    fn anvil(&mut self, n: usize) {
        let half = self.p.size.max(1.0) * 0.5;
        let h = self.p.height.max(1.0);
        let wind = Vec2::from_angle(self.p.shear_dir.to_radians()) * half * 0.25;
        for _ in 0..n {
            let r = half * 0.32 * self.lump(0.6);
            let sq = 0.35;
            let xy = self.polar(half * 0.85) + wind;
            let z = h - r * sq - self.r.gen_range(0.0..0.12) * h;
            self.out.push(Puff { c: xy.extend(z), r, sq, level: 0 });
        }
    }
    /// Rows of squashed rolls with gaps (Stratocumulus).
    fn sheet(&mut self) {
        let (w, l) = (self.p.size.max(1.0), self.p.size.max(1.0) * 0.6);
        let (cols, rows) = (6usize, 3usize);
        let cell = w / cols as f32;
        let sq = 0.55;
        let r_max = self.p.height.max(1.0) / (2.0 * sq);
        for j in 0..rows {
            for i in 0..cols {
                if self.r.gen_range(0.0..1.0) < 0.18 {
                    continue; // a gap in the deck
                }
                for k in 0..2 {
                    let r = (cell * 0.5 * self.lump(0.6)).min(r_max);
                    let x = (i as f32 + 0.5) / cols as f32 * w - w * 0.5 + self.r.gen_range(-0.2..0.2) * cell;
                    let y = (j as f32 + 0.5) / rows as f32 * l - l * 0.5 + (k as f32 - 0.5) * cell * 0.5;
                    self.out.push(Puff { c: Vec3::new(x, y, r * sq), r, sq, level: 0 });
                }
            }
        }
    }
    /// Grows smaller puffs on the puffs of `level - 1`.
    fn detail(&mut self, level: u8) {
        let puffy = self.p.puffiness.clamp(0.0, 1.0);
        let k = 3 + (puffy * 2.0).round() as usize;
        let parents: Vec<Puff> = self.out.iter().copied().filter(|q| q.level == level - 1).collect();
        for q in parents {
            for _ in 0..k {
                let up = Vec3::new(self.r.gen_range(-1.0..1.0), self.r.gen_range(-1.0..1.0), self.r.gen_range(-0.3..1.0));
                let d = up.normalize_or(Vec3::Z);
                let r = q.r * (0.3 + 0.25 * puffy) * self.lump(0.5);
                let c = q.c + Vec3::new(d.x, d.y, d.z * q.sq) * q.r * 0.8;
                if c.z >= 0.0 {
                    self.out.push(Puff { c, r, sq: q.sq, level });
                }
            }
        }
    }
}

/// The seeded layout of `p`.
pub fn layout(p: &CloudParams) -> CloudShape {
    let mut cx = Ctx { p, r: rng(p.seed, 0xC10D + p.kind as u64), out: Vec::new() };
    let h = p.height.max(1.0);
    match p.kind {
        CloudType::Cumulus => cx.tower(7, 1.0, h, 0.55),
        CloudType::Congestus => cx.tower(12, 1.0, h, 0.5),
        CloudType::Cumulonimbus => {
            cx.tower(14, 0.75, h * 0.78, 0.6);
            cx.anvil(10);
        }
        CloudType::Stratocumulus => cx.sheet(),
    }
    for level in 1..p.detail.clamp(1, 3) as u8 {
        cx.detail(level);
    }
    // Wind shear: the higher a puff, the further it leans downwind.
    let lean = Vec2::from_angle(p.shear_dir.to_radians()) * p.shear.clamp(0.0, 1.0) * p.size.max(1.0) * 0.5;
    for q in &mut cx.out {
        let t = (q.c.z / h).clamp(0.0, 1.0);
        q.c += (lean * t).extend(0.0);
    }
    CloudShape { puffs: cx.out }
}
