//! Shared vehicle parts: a car body built from a side profile (extruded
//! across the width, with real wheel arches and wells), a glass cabin
//! with pillars and roof, wheels with tyres and rims, lights, bumpers,
//! number plates, mirrors, door seams and, at Mid detail, seats and a
//! steering wheel. Front faces -Y, ground at z = 0, the length runs
//! along Y.

use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Quat, Vec2, Vec3, Vec4};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// A car body's proportions. Positions along the length are fractions
/// of the half-length (`-1` nose, `1` tail); heights are metres.
#[derive(Clone, Copy)]
pub(super) struct Body {
    pub wr: f32,
    pub axles: (f32, f32),
    pub sill: f32,
    pub nose: f32,
    pub belt: f32,
    pub tail: f32,
    /// Windshield base, roof front, roof rear and rear-glass base.
    pub ws: f32,
    pub roof: (f32, f32),
    pub back: f32,
    pub height: f32,
    pub tumble: f32,
    pub doors: u8,
    /// No roof or side glass: a roadster or an open cockpit.
    pub open: bool,
}

pub(super) const SEDAN: Body = Body {
    wr: 0.33,
    axles: (-0.6, 0.62),
    sill: 0.2,
    nose: 0.72,
    belt: 0.92,
    tail: 0.95,
    ws: -0.32,
    roof: (-0.05, 0.38),
    back: 0.62,
    height: 1.45,
    tumble: 0.1,
    doors: 4,
    open: false,
};

/// Points from local (u, v) to world (y, z) and extrudes along +X from
/// `x0`, so a counter-clockwise side profile faces outward.
fn side_frame(x0: f32) -> Mat4 {
    Mat4::from_cols(Vec4::new(0.0, 1.0, 0.0, 0.0), Vec4::new(0.0, 0.0, 1.0, 0.0), Vec4::new(1.0, 0.0, 0.0, 0.0), Vec4::new(x0, 0.0, 0.0, 1.0))
}

/// An extruded side profile from `x0` to `x1`.
pub(super) fn extrude(k: &mut Kit, profile: &[Vec2], x0: f32, x1: f32) {
    let m = k.xf;
    k.xf = m * side_frame(x0);
    k.prism(profile, 0.0, x1 - x0, true, true);
    k.xf = m;
}

/// The lower body's side profile, with or without the wheel arches.
fn profile(b: &Body, hy: f32, arches: bool) -> Vec<Vec2> {
    let mut p = vec![Vec2::new(-hy + 0.12, b.sill)];
    if arches {
        let ar = b.wr + 0.05;
        let phi = ((b.wr - b.sill) / ar).clamp(-1.0, 1.0).asin();
        for a in [b.axles.0, b.axles.1] {
            let y = a * hy;
            for i in 0..=10 {
                let t = -phi + (PI + 2.0 * phi) * i as f32 / 10.0;
                p.push(Vec2::new(y - ar * t.cos(), b.wr + ar * t.sin()));
            }
        }
    }
    let back = (b.back * hy).min(hy - 0.12);
    p.extend([
        Vec2::new(hy - 0.12, b.sill),
        Vec2::new(hy, b.sill + 0.12),
        Vec2::new(hy, b.tail - 0.08),
        Vec2::new(hy - 0.08, b.tail),
        Vec2::new(back, b.belt),
        Vec2::new(b.ws * hy, b.belt),
        Vec2::new(-hy + 0.25, b.nose),
        Vec2::new(-hy, b.nose - 0.08),
        Vec2::new(-hy, b.sill + 0.12),
    ]);
    p
}

/// A wheel: tyre, rim and hub (spokes at Mid detail); `s` is the side.
pub(super) fn wheel(k: &mut Kit, c: Vec3, r: f32, tw: f32, s: f32) {
    k.at(Mat4::from_translation(c) * Mat4::from_rotation_y(FRAC_PI_2), |k| {
        k.mat(ids::ASPHALT).frustum(Vec3::Z * -tw * 0.5, r, r, tw, 18, true);
        let z = if s > 0.0 { tw * 0.5 } else { -tw * 0.5 - 0.012 };
        k.mat(ids::IRON).frustum(Vec3::Z * z, r * 0.64, r * 0.64, 0.012, 16, true);
        k.frustum(Vec3::Z * (z + s * 0.012), r * 0.15, r * 0.12, 0.02, 8, true);
        if k.mid {
            k.mat(ids::ASPHALT);
            for i in 0..5 {
                let d = Vec2::from_angle(i as f32 * TAU / 5.0 + 0.3);
                k.beam((d * r * 0.18).extend(z + s * 0.014), (d * r * 0.6).extend(z + s * 0.014), r * 0.09, 0.008);
            }
        }
    });
}

/// The lower body, wheels, cabin, lights and trim. `paint` is the body
/// colour and `glow` the headlight material.
pub(super) fn car(k: &mut Kit, b: &Body, hx: f32, hy: f32, paint: u16, glow: u16) {
    let skin = 0.3f32.min(hx * 0.4);
    k.mat(paint);
    let arched = profile(b, hy, true);
    extrude(k, &arched, -hx, -hx + skin);
    extrude(k, &arched, hx - skin, hx);
    extrude(k, &profile(b, hy, false), -hx + skin, hx - skin);
    let tw = (b.wr * 0.68).min(skin - 0.04);
    for a in [b.axles.0, b.axles.1] {
        for s in [-1.0f32, 1.0] {
            wheel(k, Vec3::new(s * (hx - tw * 0.5 - 0.02), a * hy, b.wr), b.wr, tw, s);
        }
    }
    let (ws, back) = (b.ws * hy, (b.back * hy).min(hy - 0.12));
    let (rf, rr) = (b.roof.0 * hy, b.roof.1 * hy);
    let cx = hx - b.tumble;
    if b.open {
        // Raked windshield in a frame, and the seats show.
        let top = Vec2::new(ws + 0.28, b.belt + 0.42);
        k.mat(ids::GLASS).quad(Vec3::new(-cx, ws, b.belt), Vec3::new(cx, ws, b.belt), Vec3::new(cx, top.x, top.y), Vec3::new(-cx, top.x, top.y));
        k.quad(Vec3::new(-cx, top.x, top.y), Vec3::new(cx, top.x, top.y), Vec3::new(cx, ws, b.belt), Vec3::new(-cx, ws, b.belt));
        k.mat(paint);
        for x in [-cx, cx] {
            k.beam(Vec3::new(x, ws, b.belt), Vec3::new(x, top.x, top.y), 0.04, 0.04);
        }
        k.beam(Vec3::new(-cx, top.x, top.y), Vec3::new(cx, top.x, top.y), 0.04, 0.04);
        seats(k, b, hx, hy, true);
    } else {
        let cab = [Vec2::new(ws, b.belt), Vec2::new(back, b.belt), Vec2::new(rr, b.height), Vec2::new(rf, b.height)];
        k.mat(ids::GLASS);
        extrude(k, &cab, -cx, cx);
        k.mat(paint).block(Vec3::new(-cx - 0.01, rf - 0.02, b.height - 0.01), Vec3::new(cx + 0.01, rr + 0.02, b.height + 0.025));
        let bp = ws + (back - ws) * if b.doors >= 4 { 0.45 } else { 0.62 };
        let at = |y: f32| {
            let t = ((y - ws) / (rf - ws)).clamp(0.0, 1.0);
            let t2 = ((y - back) / (rr - back)).clamp(0.0, 1.0);
            if y < rf { b.belt + (b.height - b.belt) * t } else if y > rr { b.belt + (b.height - b.belt) * t2 } else { b.height }
        };
        for x in [-cx, cx] {
            k.beam(Vec3::new(x, ws, b.belt), Vec3::new(x, rf, b.height), 0.06, 0.05);
            k.beam(Vec3::new(x, back, b.belt), Vec3::new(x, rr, b.height), 0.12, 0.05);
            k.beam(Vec3::new(x, bp, b.belt), Vec3::new(x, bp, at(bp)), 0.08, 0.05);
        }
        if k.mid {
            seats(k, b, hx, hy, false);
        }
    }
    trim(k, b, hx, hy, paint, glow);
}

/// Front and rear seats and a steering wheel (left-hand drive).
fn seats(k: &mut Kit, b: &Body, hx: f32, hy: f32, open: bool) {
    let ws = b.ws * hy;
    let rows = if b.doors >= 4 && !open { 2 } else { 1 };
    for row in 0..rows {
        let y = ws + 0.85 + row as f32 * 0.85;
        let xs: &[f32] = if row == 0 { &[-0.42, 0.42] } else { &[0.0] };
        for &x in xs {
            let w = if row == 0 { 0.25 } else { hx - b.tumble - 0.15 };
            let x = x * (hx / 0.9);
            k.mat(ids::ASPHALT).block(Vec3::new(x - w, y - 0.25, b.belt - 0.15), Vec3::new(x + w, y + 0.1, b.belt - 0.02));
            let (zc, hh) = if open { (b.belt + 0.08, 0.22) } else { (b.belt + 0.2, 0.32) };
            k.cuboid(Vec3::new(x, y + 0.14, zc), Vec3::new(w, 0.06, hh), Quat::from_rotation_x(-0.2));
            if row == 0 {
                k.block(Vec3::new(x - 0.1, y + 0.2, zc + hh), Vec3::new(x + 0.1, y + 0.3, zc + hh + 0.12));
            }
        }
    }
    let sw = Vec3::new(-0.42 * (hx / 0.9), ws + 0.42, b.belt + 0.12);
    k.at(Mat4::from_translation(sw) * Mat4::from_rotation_x(FRAC_PI_2 - 0.4), |k| {
        k.mat(ids::ASPHALT).frustum(Vec3::ZERO, 0.19, 0.19, 0.03, 14, false);
        k.beam(Vec3::new(-0.18, 0.0, 0.015), Vec3::new(0.18, 0.0, 0.015), 0.03, 0.02);
    });
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + b.tumble + 0.02, ws + 0.05, b.belt - 0.05), Vec3::new(hx - b.tumble - 0.02, ws + 0.3, b.belt + 0.04));
}

/// Lights, grille, bumpers, plates, mirrors, door seams and handles.
fn trim(k: &mut Kit, b: &Body, hx: f32, hy: f32, paint: u16, glow: u16) {
    let (zl, zh) = (b.nose - 0.24, b.nose - 0.12);
    for s in [-1.0f32, 1.0] {
        let x = s * (hx - 0.28);
        k.mat(glow).block(Vec3::new(x - 0.18, -hy - 0.01, zl), Vec3::new(x + 0.18, -hy + 0.02, zh));
        k.mat(ids::ROOF).block(Vec3::new(x - 0.16, hy - 0.02, b.tail - 0.24), Vec3::new(x + 0.16, hy + 0.01, b.tail - 0.12));
        // Mirror on its arm by the windshield base.
        let my = b.ws * hy + 0.12;
        k.mat(paint).block(Vec3::new(s * hx, my, b.belt + 0.02), Vec3::new(s * (hx + 0.12), my + 0.08, b.belt + 0.14));
    }
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.5, -hy - 0.012, zl), Vec3::new(hx - 0.5, -hy, zh));
    for (y0, y1) in [(-hy - 0.04, -hy + 0.1), (hy - 0.1, hy + 0.04)] {
        k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.02, y0, b.sill + 0.04), Vec3::new(hx - 0.02, y1, b.sill + 0.2));
    }
    k.mat(ids::PLASTER);
    k.block(Vec3::new(-0.26, -hy - 0.05, b.sill + 0.08), Vec3::new(0.26, -hy - 0.04, b.sill + 0.19));
    k.block(Vec3::new(-0.26, hy + 0.04, b.sill + 0.26), Vec3::new(0.26, hy + 0.05, b.sill + 0.37));
    if b.doors == 0 {
        return;
    }
    let ws = b.ws * hy;
    let back = (b.back * hy).min(hy - 0.12);
    let seams: Vec<f32> = if b.doors >= 4 {
        vec![ws + 0.05, ws + (back - ws) * 0.45, back - 0.05]
    } else {
        vec![ws + 0.05, ws + (back - ws) * 0.62 + 0.04]
    };
    for s in [-1.0f32, 1.0] {
        let x = s * (hx + 0.003);
        for &y in &seams {
            k.mat(ids::ASPHALT).block(Vec3::new(x - 0.004, y - 0.006, b.sill + 0.06), Vec3::new(x + 0.004, y + 0.006, b.belt - 0.02));
        }
        for w in seams.windows(2) {
            k.mat(ids::IRON).block(Vec3::new(x - 0.012, w[1] - 0.2, b.belt - 0.12), Vec3::new(x + 0.012, w[1] - 0.06, b.belt - 0.09));
        }
    }
}
