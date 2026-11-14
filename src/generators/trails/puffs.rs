//! Puff layout per trail type: tubes of puffs that age along the path.

use super::path::Path;
use super::{TrailKind, TrailParams};
use crate::generators::clouds::Puff;
use crate::generators::clouds::mesh::sphere;
use crate::generators::kit::rng;
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

/// Most puffs one trail lays out (spacing widens past it).
pub const MAX_PUFFS: usize = 6000;
/// Material IDs of the preview mesh (Smoke & Fire pack).
pub const FLAME_ID: u16 = 1;
pub const SMOKE_ID: u16 = 2;

/// One round puff of a trail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailPuff {
    pub c: Vec3,
    pub r: f32,
    /// Peak density 0..=1 (fades with age; flames: heat).
    pub w: f32,
    pub flame: bool,
}

struct Ctx<'a> {
    path: &'a Path,
    rng: ChaCha8Rng,
    /// Spacing between puffs as a share of their radius.
    step: f32,
    out: Vec<TrailPuff>,
}

/// One tube of puffs from arc length `s0` to `s1`; `rad`, `wt` and `off`
/// (side, up offset) take the age 0..=1 and the arc length.
struct Tube<R, W, O> {
    s0: f32,
    s1: f32,
    flame: bool,
    breakup: f32,
    spread: f32,
    rad: R,
    wt: W,
    off: O,
}

impl Ctx<'_> {
    fn tube<R: Fn(f32) -> f32, W: Fn(f32, f32) -> f32, O: Fn(f32, f32) -> Vec2>(&mut self, t: Tube<R, W, O>) {
        let mut s = t.s0;
        let span = (t.s1 - t.s0).max(1e-4);
        while s < t.s1 {
            let f = (s - t.s0) / span;
            let r = ((t.rad)(f) * (1.0 + (t.breakup * f * 0.35 + 0.06) * self.rng.gen_range(-1.0..1.0))).max(0.01);
            let skip = self.rng.gen_range(0.0f32..1.0) < t.breakup * f * 0.45;
            let j = Vec2::new(self.rng.gen_range(-1.0..1.0), self.rng.gen_range(-1.0..1.0)) * t.spread * f * r * 1.5;
            if !skip {
                let o = (t.off)(f, s) + j;
                let (side, up) = self.path.frame(s);
                let c = self.path.at(s) + side * o.x + up * o.y;
                self.out.push(TrailPuff { c, r, w: (t.wt)(f, s).clamp(0.05, 1.0), flame: t.flame });
            }
            s += (r * self.step).max(0.01);
        }
    }
}

/// Lateral offsets of `n` sources `spacing` apart.
fn sources(n: u32, spacing: f32) -> impl Iterator<Item = f32> {
    let n = n.max(1);
    (0..n).map(move |i| (i as f32 - (n - 1) as f32 * 0.5) * spacing)
}

fn lay(p: &TrailParams, path: &Path, step: f32) -> Vec<TrailPuff> {
    let mut cx = Ctx { path, rng: rng(p.seed, 0x7E41 + p.kind as u64), step, out: Vec::new() };
    let l = path.len().min(p.dissipation.max(0.01));
    let (r0, r1) = (p.start_radius.max(0.01), p.end_radius.max(0.01));
    let grow = move |f: f32| r0 + (r1 - r0) * f.powf(0.8);
    let fade = |f: f32, _s: f32| 1.0 - p.fade * f.powf(1.5);
    let (breakup, spread) = (p.breakup.clamp(0.0, 1.0), p.spread.clamp(0.0, 1.0));
    // Smoke wanders slowly sideways and up as it ages.
    let ph = p.seed as f32 * 0.37;
    let wander = move |f: f32, s: f32| Vec2::new((s * 0.031 + ph).sin(), (s * 0.023 + ph * 1.7).cos() * 0.6) * spread * f * r1;
    let gap = p.start_gap.clamp(0.0, l);
    match p.kind {
        TrailKind::Contrail => {
            for x in sources(p.sources, p.spacing) {
                cx.tube(Tube { s0: gap, s1: l, flame: false, breakup, spread, rad: grow, wt: fade, off: |f, _| Vec2::new(x * (1.0 - 0.3 * f), 0.0) });
            }
        }
        TrailKind::WingtipVortices => {
            let tw = p.twist * TAU / 100.0;
            for (i, x) in sources(2, p.spacing).enumerate() {
                let (sign, sr) = (if i == 0 { -1.0 } else { 1.0 }, r0 * 3.0);
                // Stronger where the path turns hard.
                let wt = |f: f32, s: f32| (1.0 - p.fade * f) * (0.35 + 0.65 * (path.curvature(s, 5.0) * 150.0).min(1.0));
                let off = move |_f: f32, s: f32| Vec2::new(x + sr * (s * tw * sign).cos(), sr * (s * tw * sign).sin());
                cx.tube(Tube { s0: gap, s1: l, flame: false, breakup, spread, rad: grow, wt, off });
            }
        }
        TrailKind::Afterburner => {
            let t = p.throttle.clamp(0.0, 1.0);
            let lf = l * (0.4 + 0.6 * t);
            let cone = move |f: f32| r0 + (r1 - r0) * f;
            for x in sources(p.sources, p.spacing) {
                let off = move |_, _| Vec2::new(x, 0.0);
                cx.tube(Tube { s0: 0.0, s1: lf, flame: true, breakup, spread: 0.0, rad: cone, wt: |f, _| 0.55 * (1.0 - p.fade * f), off });
                // Shock diamonds: hot knots spaced along the core.
                let n = p.diamonds.min(12);
                for k in 0..n {
                    let f = 0.12 + 0.72 * k as f32 / n.max(1) as f32;
                    let (side, _) = path.frame(f * lf);
                    cx.out.push(TrailPuff { c: path.at(f * lf) + side * x, r: cone(f) * 0.8, w: 1.0 - 0.5 * f, flame: true });
                }
            }
        }
        TrailKind::VaporCone => {
            let mut s = 0.0;
            while s < l {
                let f = s / l.max(1e-4);
                let ring = r0 + (r1 - r0) * f;
                let th = (0.3 + 0.08 * ring) * 1.2;
                let n = ((TAU * ring / (th * step * 1.6)).ceil() as usize).clamp(6, 256);
                let (side, up) = path.frame(s);
                for k in 0..n {
                    if cx.rng.gen_range(0.0f32..1.0) < breakup * f * 0.6 {
                        continue;
                    }
                    let a = TAU * (k as f32 + cx.rng.gen_range(0.0f32..1.0) * 0.4) / n as f32;
                    let c = path.at(s) + (side * a.cos() + up * a.sin()) * ring;
                    cx.out.push(TrailPuff { c, r: th, w: (1.0 - p.fade * f).max(0.05), flame: false });
                }
                s += th * step * 1.2;
            }
        }
        TrailKind::DamageSmoke => {
            cx.tube(Tube { s0: gap, s1: l, flame: false, breakup, spread, rad: grow, wt: fade, off: wander });
            if p.fire {
                let lf = (r0 * 6.0).min(l * 0.3);
                cx.tube(Tube { s0: 0.0, s1: lf, flame: true, breakup: 0.0, spread: 0.0, rad: |f| r0 * (1.1 - 0.6 * f), wt: |f, _| 0.9 - 0.6 * f, off: |_, _| Vec2::ZERO });
            }
        }
        TrailKind::MissileTrail => {
            let t = p.throttle.clamp(0.0, 1.0);
            let lf = (r0 * 4.0 * (0.5 + 0.5 * t)).min(l);
            cx.tube(Tube { s0: 0.0, s1: lf, flame: true, breakup: 0.0, spread: 0.0, rad: |f| r0 * 0.8 * (1.0 - 0.5 * f), wt: |f, _| 1.0 - 0.6 * f, off: |_, _| Vec2::ZERO });
            cx.tube(Tube { s0: gap, s1: l, flame: false, breakup, spread, rad: grow, wt: fade, off: wander });
        }
    }
    cx.out
}

/// Every puff of `p`; spacing widens until at most `MAX_PUFFS` remain.
pub fn layout(p: &TrailParams) -> Vec<TrailPuff> {
    let Some(path) = Path::new(&p.path_points()) else { return Vec::new() };
    let mut step = 0.55;
    loop {
        let out = lay(p, &path, step);
        if out.len() <= MAX_PUFFS || step > 8.0 {
            return out;
        }
        step *= (out.len() as f32 / MAX_PUFFS as f32).max(1.1);
    }
}

/// Preview mesh: one small sphere per puff.
pub fn mesh(puffs: &[TrailPuff]) -> EditablePoly {
    let mut m = EditablePoly::new();
    for q in puffs {
        let id = if q.flame { FLAME_ID } else { SMOKE_ID };
        sphere(&mut m, &Puff { c: q.c, r: q.r, sq: 1.0, level: 2 }, 0.0, id);
    }
    m
}
