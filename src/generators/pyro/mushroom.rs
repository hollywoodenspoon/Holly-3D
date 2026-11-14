//! The nuclear mushroom cloud. Stage runs from a ground fireball (0 %)
//! through the stem (30 %) and spreading cap (60 %) to a mature dark cloud
//! (100 %). The cap is a rolling ring (a torus of puffs) with fire glowing
//! on its underside early on; a condensation skirt rings the stem mid-way,
//! an ice cap forms on top late, and a base surge rolls out on the ground.
//! The cap rises fast then slows, as a buoyant thermal does.

use super::PyroParams;
use super::layout::{Ctx, Medium, ease};
use bevy::math::Vec3;
use std::f32::consts::{PI, TAU};

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(super) fn lay(cx: &mut Ctx, p: &PyroParams) {
    let (s, size) = (p.stage.clamp(0.0, 1.0), p.size.max(0.01));
    let e = ease(s);
    // Cap height, ring radius and tube radius.
    let h = size * (0.8 + 5.2 * e);
    let ring = size * (0.5 + 1.5 * e) * smooth(0.08, 0.4, s);
    let tube = size * (0.45 + 0.35 * e);
    let fire = (1.0 - s * 1.4).clamp(0.0, 1.0);
    // Cap: puffs filling the rolling torus.
    let n = (90.0 + 90.0 * cx.billow) as usize;
    for _ in 0..n {
        let (phi, th, rho) = (cx.f() * TAU, cx.f() * TAU, cx.f().powf(0.4));
        let rad = ring + tube * th.cos() * rho;
        let q = Vec3::new(rad * phi.cos(), rad * phi.sin(), h + tube * th.sin() * rho * 0.8);
        let r = tube * (0.32 + 0.2 * cx.f());
        // Fire glows longest on the inner underside.
        let rank = (th.sin() + 1.0) * 0.3 + rho * 0.2 + cx.f() * 0.2;
        if rank < fire {
            let w = 1.0 - 0.5 * rank / fire.max(1e-3);
            cx.push(q, r, 1.0, w, Medium::Fire);
        } else {
            let w = 0.5 + 0.45 * cx.f();
            cx.push(q, r, 1.0, w, Medium::Smoke);
        }
    }
    // Stem: dusty below, smoky above, fiery at its top early on.
    if s > 0.08 {
        let sr = size * (0.25 + 0.25 * e) * smooth(0.08, 0.3, s);
        let top = (h - tube * 0.6).max(0.0);
        let mut z = sr * 0.5;
        while z < top && sr > 0.0 {
            let f = z / top.max(1e-3);
            for k in 0..3 {
                let a = TAU * (k as f32 + cx.f()) / 3.0;
                let q = Vec3::new(a.cos(), a.sin(), 0.0) * sr * 0.4 + Vec3::Z * z;
                let m = if f > 0.75 && fire > 0.4 { Medium::Fire } else if f < 0.35 { Medium::Dust } else { Medium::Smoke };
                let w = if m == Medium::Fire { fire } else { 0.5 + 0.3 * cx.f() };
                cx.push(q, sr * 0.7, 1.0, w, m);
            }
            z += sr * 0.8;
        }
    }
    // Condensation skirt: a white bell around the stem under the cap.
    let skirt = if (0.25..0.75).contains(&s) { (PI * (s - 0.25) / 0.5).sin() } else { 0.0 };
    if skirt > 0.05 {
        let rr = ring * 1.1 + tube * 0.4;
        for k in 0..36 {
            let a = TAU * (k as f32 + cx.f() * 0.5) / 36.0;
            let q = Vec3::new(a.cos() * rr, a.sin() * rr, h - tube * 1.2);
            cx.push(q, tube * 0.35, 0.5, skirt * 0.8, Medium::Vapor);
        }
    }
    // Ice cap: a thin white layer forming on top late.
    let ice = smooth(0.45, 0.7, s);
    if ice > 0.0 {
        let disc = ring * 0.9 + tube * 0.5;
        for _ in 0..24 {
            let (a, d) = (cx.f() * TAU, cx.f().sqrt() * disc);
            let q = Vec3::new(a.cos() * d, a.sin() * d, h + tube * 0.95);
            cx.push(q, tube * 0.5, 0.3, 0.6 * ice, Medium::Vapor);
        }
    }
    // Base surge rolling out over the ground.
    if s > 0.15 {
        let (rr, r) = (size * (0.6 + 5.0 * (s - 0.15)), size * (0.35 + 0.4 * s));
        for k in 0..40 {
            let a = TAU * (k as f32 + cx.f() * 0.6) / 40.0;
            let q = Vec3::new(a.cos() * rr, a.sin() * rr, r * 0.4);
            cx.push(q, r, 0.5, 0.6 * (1.0 - 0.5 * s), Medium::Dust);
        }
    }
}
