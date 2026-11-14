//! Sparks and burning debris on seeded ballistic arcs with gravity and
//! linear drag, seen at the current Stage. Sparks are short glowing streaks
//! stretched along their direction of travel (a motion-blur look); debris
//! leaves a widening smoke trail behind a glowing tip.

use super::PyroParams;
use super::layout::{Ctx, Medium, origin};
use bevy::math::Vec3;

const G: Vec3 = Vec3::new(0.0, 0.0, -9.81);
/// Drag rate (1/s) of sparks and of heavier debris.
const DRAG: f32 = 1.2;
const DEBRIS_DRAG: f32 = 0.4;
/// Most sparks and debris pieces.
pub const MAX_SPARKS: u32 = 1500;
pub const MAX_DEBRIS: u32 = 64;

/// Position after `t` seconds from `p0` at launch velocity `v0`.
pub fn arc(p0: Vec3, v0: Vec3, k: f32, t: f32) -> Vec3 {
    let e = (1.0 - (-k * t).exp()) / k;
    p0 + G * t / k + (v0 - G / k) * e
}

fn launch(cx: &mut Ctx, p: &PyroParams, speed: f32, lift: f32) -> Vec3 {
    let mut d = cx.unit();
    if origin(p).z < p.size * 0.5 {
        // Ground blasts throw into the upper hemisphere.
        d.z = d.z.abs().max(lift);
        d = d.normalize();
    }
    let v = speed * (1.0 + p.speed_spread.clamp(0.0, 1.0) * (cx.f() * 2.0 - 1.0));
    d * v.max(0.5)
}

pub(super) fn sparks(cx: &mut Ctx, p: &PyroParams) {
    let t = p.time();
    if p.sparks == 0 || t <= 0.0 || p.spark_speed <= 0.0 {
        return;
    }
    let r = (p.size * 0.012).clamp(0.01, 0.5);
    let dt = 0.05 * p.streak.max(0.05);
    for _ in 0..p.sparks.min(MAX_SPARKS) {
        let p0 = origin(p) + cx.unit() * p.size * 0.2 * cx.f();
        let v0 = launch(cx, p, p.spark_speed, 0.1);
        let w = (-p.spark_cooling.clamp(0.0, 1.0) * 2.5 * t).exp() * (0.7 + 0.3 * cx.f());
        let (a, b) = (arc(p0, v0, DRAG, (t - dt).max(0.0)), arc(p0, v0, DRAG, t));
        if w < 0.08 || b.z < 0.0 {
            continue;
        }
        let n = ((a.distance(b) / (r * 1.2)).ceil() as usize).clamp(2, 4);
        for i in 0..n {
            let f = i as f32 / (n - 1) as f32;
            // The head is hottest; the tail fades.
            cx.push(a.lerp(b, f), r, 1.0, w * (0.4 + 0.6 * f), Medium::Sparks);
        }
    }
}

pub(super) fn debris(cx: &mut Ctx, p: &PyroParams) {
    let t = p.time();
    if p.debris == 0 || t <= 0.0 {
        return;
    }
    let speed = 0.7 * p.spark_speed.max(20.0);
    let life = 2.0;
    for _ in 0..p.debris.min(MAX_DEBRIS) {
        let p0 = origin(p);
        let k = 0.6 + 0.6 * cx.f();
        let v0 = launch(cx, p, speed * k, 0.5);
        let mut tau = (t - life).max(0.0);
        while tau < t {
            let age = t - tau;
            let q = arc(p0, v0, DEBRIS_DRAG, tau);
            if q.z > 0.0 {
                let r = p.size * 0.05 * (1.0 + 3.0 * age);
                let j = cx.unit() * r * 0.3;
                cx.push(q + j, r, 1.0, 0.7 * (1.0 - age / life), Medium::Smoke);
            }
            tau += 0.08;
        }
        let tip = arc(p0, v0, DEBRIS_DRAG, t);
        if tip.z > 0.0 {
            cx.push(tip, p.size * 0.06, 1.0, (-0.3 * t).exp(), Medium::Fire);
        }
    }
}
