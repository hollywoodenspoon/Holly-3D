//! Puff layouts per explosion type at the current Stage.

use super::{PyroKind, PyroParams};
use crate::generators::clouds::Puff;
use crate::generators::clouds::mesh::sphere;
use crate::generators::kit::rng;
use crate::mesh::EditablePoly;
use bevy::math::Vec3;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

/// Most puffs one pyro object lays out.
pub const MAX_PUFFS: usize = 8000;
/// Most spark puffs the preview mesh draws.
pub const PREVIEW_SPARKS: usize = 600;

/// What a puff is made of (each medium renders as its own volume).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Medium {
    Fire,
    Smoke,
    Dust,
    Vapor,
    Sparks,
}

impl Medium {
    pub const ALL: [Self; 5] = [Self::Fire, Self::Smoke, Self::Dust, Self::Vapor, Self::Sparks];
    /// Material ID in the preview mesh (Smoke & Fire pack).
    pub fn id(self) -> u16 {
        match self {
            Self::Fire | Self::Sparks => 1,
            Self::Smoke | Self::Dust => 2,
            Self::Vapor => 3,
        }
    }
    /// Glowing media keep a smooth body (little erosion).
    pub fn glows(self) -> bool {
        matches!(self, Self::Fire | Self::Sparks)
    }
}

/// One soft puff of an explosion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyroPuff {
    pub c: Vec3,
    pub r: f32,
    /// Vertical squash (1 = round).
    pub sq: f32,
    /// Peak density 0..=1 (fire: heat).
    pub w: f32,
    pub medium: Medium,
}

pub(super) struct Ctx {
    pub rng: ChaCha8Rng,
    pub out: Vec<PyroPuff>,
    pub billow: f32,
}

/// 1 - (1 - x)², clamped: fast start, gentle end.
pub(super) fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    1.0 - (1.0 - x) * (1.0 - x)
}

impl Ctx {
    pub fn f(&mut self) -> f32 {
        self.rng.gen_range(0.0f32..1.0)
    }
    pub fn unit(&mut self) -> Vec3 {
        loop {
            let v = Vec3::new(self.rng.gen_range(-1.0..1.0), self.rng.gen_range(-1.0..1.0), self.rng.gen_range(-1.0..1.0));
            let l = v.length_squared();
            if l > 1e-4 && l <= 1.0 {
                return v / l.sqrt();
            }
        }
    }
    pub fn push(&mut self, c: Vec3, r: f32, sq: f32, w: f32, medium: Medium) {
        if r > 0.0 && c.is_finite() {
            self.out.push(PyroPuff { c, r, sq, w: w.clamp(0.05, 1.0), medium });
        }
    }

    /// A billowing ball of radius `big` at `c`: the inner `fire` share of it
    /// still burns (heat `hot` at the core), the rest is smoke. `ground`
    /// keeps puffs above z = 0.
    pub fn ball(&mut self, c: Vec3, big: f32, fire: f32, hot: f32, ground: bool) {
        let n = (36.0 + 70.0 * self.billow) as usize;
        let (lo, hi) = (0.42 - 0.2 * self.billow, 0.55 - 0.15 * self.billow);
        for _ in 0..n {
            let d = self.f().powf(0.45);
            let dir = self.unit();
            let r = big * self.rng.gen_range(lo..hi);
            let mut p = c + dir * d * (big - r * 0.6);
            if ground {
                p.z = p.z.max(r * 0.45);
            }
            let rank = d * 0.75 + self.f() * 0.25;
            if rank < fire {
                let w = hot * (1.0 - 0.55 * rank / fire.max(1e-3));
                self.push(p, r, 1.0, w, Medium::Fire);
            } else {
                let w = 0.55 + 0.45 * self.f();
                self.push(p, r, 1.0, w, Medium::Smoke);
            }
        }
    }
}

/// Where the blast starts (sparks and debris launch from here).
pub(super) fn origin(p: &PyroParams) -> Vec3 {
    match p.kind {
        PyroKind::Fireball | PyroKind::FlakBurst | PyroKind::AerialExplosion => Vec3::Z * p.size * 1.2,
        PyroKind::GroundExplosion | PyroKind::Napalm => Vec3::Z * p.size * 0.3,
        PyroKind::Mushroom => Vec3::Z * p.size * 0.5,
    }
}

fn air(cx: &mut Ctx, p: &PyroParams, grow_rate: f32, cool_rate: f32, rise: f32) {
    let (s, size) = (p.stage.clamp(0.0, 1.0), p.size.max(0.01));
    let big = size * (0.35 + 0.65 * ease(s * grow_rate)) * (1.0 + 0.5 * s);
    let c = origin(p) + Vec3::Z * size * rise * s.powf(1.2);
    let fire = (1.0 - s * cool_rate).clamp(0.0, 1.0).powf(0.8);
    cx.ball(c, big, fire, 1.0, false);
    // A short smoke stem trails under the rising ball.
    if s > 0.3 && rise > 0.0 {
        let n = (6.0 * (s - 0.3) / 0.7).ceil() as usize + 1;
        for i in 0..n {
            let f = (i as f32 + 0.5) / n as f32;
            let q = origin(p).lerp(c - Vec3::Z * big * 0.6, f) + cx.unit() * size * 0.08;
            let w = 0.35 + 0.3 * cx.f();
            cx.push(q, size * (0.18 + 0.12 * f), 1.0, w, Medium::Smoke);
        }
    }
}

fn ground(cx: &mut Ctx, p: &PyroParams) {
    let (s, size) = (p.stage.clamp(0.0, 1.0), p.size.max(0.01));
    let big = size * (0.4 + 0.6 * ease(s * 3.0)) * (1.0 + 0.4 * s);
    let c = Vec3::Z * (big * 0.25 + size * 1.6 * s.powf(1.3));
    cx.ball(c, big, (1.0 - s * 1.5).clamp(0.0, 1.0).powf(0.8), 1.0, true);
    // Dust skirt rolling out along the ground.
    let n = 24;
    let ring = big * (0.9 + 1.6 * s);
    for k in 0..n {
        let a = TAU * (k as f32 + cx.f() * 0.6) / n as f32;
        let r = size * (0.25 + 0.2 * s) * (0.8 + 0.4 * cx.f());
        let w = 0.4 + 0.4 * cx.f();
        cx.push(Vec3::new(a.cos() * ring, a.sin() * ring, r * 0.5), r, 0.6, w, Medium::Dust);
    }
    // Ejecta: a dusty cone thrown up through the fire.
    let top = big * 2.0 * ease(s * 3.0);
    for k in 0..10 {
        let f = (k as f32 + 0.5) / 10.0;
        let q = Vec3::new(0.0, 0.0, top * f) + cx.unit() * big * 0.25 * f;
        let w = 0.5 * (1.0 - 0.5 * s);
        cx.push(q, big * 0.3, 1.0, w, Medium::Dust);
    }
}

fn flak(cx: &mut Ctx, p: &PyroParams) {
    let (s, size) = (p.stage.clamp(0.0, 1.0), p.size.max(0.01));
    let big = size * (0.3 + 0.7 * ease(s * 4.0)) * (1.0 + 0.3 * s);
    let c = origin(p);
    cx.billow = cx.billow.max(0.8);
    cx.ball(c, big, (1.0 - s * 6.0).clamp(0.0, 1.0), 1.0, false);
    // Ragged tendrils of smoke flung out of the burst.
    for _ in 0..7 {
        let d = cx.unit();
        for k in 1..=4 {
            let f = k as f32 / 4.0;
            let w = 0.5 * (1.0 - 0.5 * f);
            cx.push(c + d * big * (0.8 + 0.7 * f), big * 0.18 * (1.0 - 0.4 * f), 1.0, w, Medium::Smoke);
        }
    }
}

fn napalm(cx: &mut Ctx, p: &PyroParams) {
    let (s, size) = (p.stage.clamp(0.0, 1.0), p.size.max(0.01));
    let len = size * 5.0;
    let front = len * (0.15 + 2.0 * s).min(1.0);
    let step = size * 0.35;
    let mut x = 0.0;
    let mut i = 0;
    while x < front {
        // Older fire (far from the front) burns lower and cooler.
        let age = 1.0 - x / front.max(1e-3);
        let at = Vec3::X * (x - len * 0.5);
        for _ in 0..3 {
            let q = at + Vec3::new(cx.f() - 0.5, (cx.f() - 0.5) * 1.6, 0.0) * size * 0.6;
            let r = size * (0.35 + 0.25 * cx.f()) * (1.0 - 0.3 * age);
            let w = (1.0 - 0.5 * age) * (0.7 + 0.3 * cx.f());
            cx.push(q + Vec3::Z * r * 0.8, r, 1.0, w, Medium::Fire);
        }
        // Thick smoke rises from every other step.
        if i % 2 == 0 {
            let h = size * (1.5 + 5.0 * s) * (0.5 + 0.5 * age);
            let n = (h / (size * 0.6)).ceil() as usize;
            for k in 0..n {
                let f = (k as f32 + 0.5) / n as f32;
                let r = size * (0.4 + 0.8 * f);
                let q = at + Vec3::Z * (size * 0.9 + h * f) + cx.unit() * r * 0.4;
                let w = 0.5 + 0.4 * cx.f();
                cx.push(q, r, 1.0, w, Medium::Smoke);
            }
        }
        x += step;
        i += 1;
    }
}

/// Every puff of `p` (the main body, then sparks and debris).
pub fn layout(p: &PyroParams) -> Vec<PyroPuff> {
    let mut cx = Ctx { rng: rng(p.seed, 0xB1A5 + p.kind as u64), out: Vec::new(), billow: p.billow.clamp(0.0, 1.0) };
    match p.kind {
        PyroKind::Fireball => air(&mut cx, p, 2.5, 1.3, 2.0),
        PyroKind::AerialExplosion => air(&mut cx, p, 3.0, 1.1, 0.8),
        PyroKind::GroundExplosion => ground(&mut cx, p),
        PyroKind::FlakBurst => flak(&mut cx, p),
        PyroKind::Napalm => napalm(&mut cx, p),
        PyroKind::Mushroom => super::mushroom::lay(&mut cx, p),
    }
    super::sparks::debris(&mut cx, p);
    super::sparks::sparks(&mut cx, p);
    cx.out.truncate(MAX_PUFFS);
    cx.out
}

/// Spark puffs drawn in the preview (every `stride`-th).
pub fn spark_stride(puffs: &[PyroPuff]) -> usize {
    puffs.iter().filter(|q| q.medium == Medium::Sparks).count().div_ceil(PREVIEW_SPARKS).max(1)
}

/// Preview mesh: one small sphere per puff (sparks thinned out).
pub fn mesh(puffs: &[PyroPuff]) -> EditablePoly {
    let stride = spark_stride(puffs);
    let mut m = EditablePoly::new();
    let mut k = 0;
    for q in puffs {
        if q.medium == Medium::Sparks {
            k += 1;
            if (k - 1) % stride != 0 {
                continue;
            }
        }
        sphere(&mut m, &Puff { c: q.c, r: q.r, sq: q.sq, level: 2 }, 0.0, q.medium.id());
    }
    m
}
