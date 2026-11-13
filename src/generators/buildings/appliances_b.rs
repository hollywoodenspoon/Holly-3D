//! Appliances, part two: the small ones and electronics. Microwave,
//! kettle and toaster, coffee machine, television, PC setup, laptop and
//! speakers. Counter-top pieces stand at z = 0 so Drop on Surfaces puts
//! them on a worktop or desk; the front faces -Y.

use super::appliances::{dial, shell};
use super::furniture::cushion;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec3};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Microwave => microwave(b, hx, hy),
        K::KettleToaster => kettle_toaster(b, hx),
        K::CoffeeMachine => coffee(b, hx, hy),
        K::Television => television(b, hx, hy),
        K::PcSetup => pc(b, hx, hy),
        K::Laptop => laptop(b, hx, hy),
        K::Speakers => speakers(b, hx),
        _ => {}
    }
}

/// A flat screen of half-width `w` and height `h` centred at `c`,
/// facing -Y, with a bezel and a lit panel.
fn screen(k: &mut Kit, glow: u16, c: Vec3, w: f32, h: f32) {
    k.mat(ids::ASPHALT).block(c - Vec3::new(w, 0.02, h * 0.5), c + Vec3::new(w, 0.015, h * 0.5));
    k.mat(glow).block(c - Vec3::new(w - 0.012, 0.025, h * 0.5 - 0.012), c + Vec3::new(w - 0.012, -0.02, h * 0.5 - 0.012));
}

fn microwave(b: &mut Ctx, hx: f32, hy: f32) {
    // A counter-top microwave: dark glass door with a grille, a side
    // control panel with a display and buttons, a handle and feet.
    let body = shell(b);
    let glow = b.glow(true);
    let k = &mut b.kit;
    let h = 0.3;
    k.mat(ids::ASPHALT);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        k.frustum(Vec3::new(x * (hx - 0.04), y * (hy - 0.04), 0.0), 0.015, 0.015, 0.01, 6, true);
    }
    k.mat(body);
    cushion(k, Vec3::new(-hx, -hy, 0.01), Vec3::new(hx, hy, h));
    let px = hx - 0.13;
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.02, -hy - 0.005, 0.03), Vec3::new(px - 0.01, -hy, h - 0.02));
    k.mat(ids::GLASS).block(Vec3::new(-hx + 0.05, -hy - 0.008, 0.06), Vec3::new(px - 0.07, -hy - 0.005, h - 0.05));
    k.mat(ids::IRON).block(Vec3::new(px - 0.05, -hy - 0.03, 0.07), Vec3::new(px - 0.03, -hy, h - 0.06));
    k.mat(glow).block(Vec3::new(px + 0.02, -hy - 0.005, h - 0.07), Vec3::new(hx - 0.02, -hy, h - 0.04));
    k.mat(ids::STUCCO);
    for r in 0..4 {
        for c in 0..3 {
            let x = px + 0.025 + c as f32 * 0.03;
            let z = h - 0.11 - r as f32 * 0.028;
            k.block(Vec3::new(x - 0.01, -hy - 0.006, z - 0.008), Vec3::new(x + 0.01, -hy, z + 0.008));
        }
    }
    dial(k, Vec3::new((px + hx) * 0.5, -hy, 0.06), 0.022);
}

fn kettle_toaster(b: &mut Ctx, hx: f32) {
    // A jug kettle on its power base, and a two-slot toaster with a
    // lever and a browning dial, side by side.
    let body = shell(b);
    let glow = b.glow(true);
    let k = &mut b.kit;
    let kc = Vec3::new(-hx + 0.12, 0.0, 0.0);
    k.mat(ids::ASPHALT).frustum(kc, 0.1, 0.1, 0.02, 16, true);
    k.mat(body).frustum(kc + Vec3::Z * 0.02, 0.095, 0.075, 0.22, 16, true);
    k.mat(ids::ASPHALT).frustum(kc + Vec3::Z * 0.24, 0.07, 0.04, 0.025, 12, true);
    k.frustum(kc + Vec3::Z * 0.265, 0.015, 0.015, 0.02, 6, true);
    k.mat(glow).frustum(kc + Vec3::new(0.0, -0.09, 0.03), 0.012, 0.012, 0.01, 6, true);
    k.mat(ids::ASPHALT);
    k.beam(kc + Vec3::new(0.08, 0.0, 0.06), kc + Vec3::new(0.14, 0.0, 0.08), 0.03, 0.025);
    k.beam(kc + Vec3::new(0.14, 0.0, 0.08), kc + Vec3::new(0.12, 0.0, 0.22), 0.03, 0.025);
    k.beam(kc + Vec3::new(0.12, 0.0, 0.22), kc + Vec3::new(0.06, 0.0, 0.23), 0.03, 0.025);
    k.mat(body).beam(kc + Vec3::new(-0.08, 0.0, 0.17), kc + Vec3::new(-0.13, 0.0, 0.21), 0.035, 0.03);
    // Toaster.
    let t = Vec3::new(hx - 0.16, 0.0, 0.0);
    k.mat(body);
    cushion(k, t + Vec3::new(-0.14, -0.09, 0.0), t + Vec3::new(0.14, 0.09, 0.19));
    k.mat(ids::ASPHALT);
    for x in [-0.05, 0.05] {
        k.block(t + Vec3::new(x - 0.075, -0.018, 0.185), t + Vec3::new(x + 0.075, 0.018, 0.192));
    }
    k.block(t + Vec3::new(0.11, -0.115, 0.12), t + Vec3::new(0.13, -0.09, 0.14));
    k.mat(ids::TRIM).block(t + Vec3::new(-0.06, -0.012, 0.19), t + Vec3::new(-0.03, 0.012, 0.23));
    dial(k, t + Vec3::new(0.12, -0.09, 0.05), 0.015);
}

fn coffee(b: &mut Ctx, hx: f32, hy: f32) {
    // An espresso machine: a boxy body with a cup warmer on top, a
    // group head with a portafilter, a steam wand, a drip tray and cups.
    let body = shell(b);
    let glow = b.glow(true);
    let k = &mut b.kit;
    let h = 0.38;
    k.mat(body);
    cushion(k, Vec3::new(-hx, -hy + 0.1, 0.0), Vec3::new(hx, hy, h));
    k.block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, -hy + 0.1, 0.05));
    k.mat(ids::IRON).block(Vec3::new(-hx + 0.03, -hy + 0.01, 0.05), Vec3::new(hx - 0.03, -hy + 0.1, 0.06));
    k.block(Vec3::new(-hx + 0.02, -hy + 0.12, h), Vec3::new(hx - 0.02, hy - 0.02, h + 0.015));
    let g = Vec3::new(-hx * 0.2, -hy + 0.08, h - 0.12);
    k.frustum(g, 0.04, 0.04, 0.04, 12, true);
    k.mat(ids::ASPHALT).beam(g + Vec3::new(0.0, -0.02, 0.0), g + Vec3::new(0.0, -0.18, -0.02), 0.025, 0.02);
    k.mat(ids::PLASTER).frustum(Vec3::new(-hx * 0.2, -hy + 0.06, 0.06), 0.035, 0.04, 0.06, 10, true);
    k.mat(ids::IRON).tube(Vec3::new(hx * 0.6, -hy + 0.1, h - 0.1), Vec3::new(hx * 0.65, -hy + 0.04, 0.1), 0.006, 6);
    k.mat(glow).block(Vec3::new(hx * 0.1, -hy + 0.095, h - 0.06), Vec3::new(hx * 0.5, -hy + 0.1, h - 0.03));
    dial(k, Vec3::new(-hx * 0.75, -hy + 0.1, h - 0.06), 0.025);
    k.mat(ids::PLASTER);
    for i in 0..3 {
        k.frustum(Vec3::new(-hx * 0.5 + i as f32 * 0.07, 0.0, h + 0.015), 0.03, 0.035, 0.05, 10, true);
    }
}

fn television(b: &mut Ctx, hx: f32, hy: f32) {
    // A flat-screen TV on two splayed feet or a centre pedestal (by
    // seed), with a soundbar in front.
    let pedestal = b.r.gen_bool(0.5);
    let glow = b.glow(false);
    let k = &mut b.kit;
    let w = hx - 0.02;
    let h = w * 2.0 * 9.0 / 16.0;
    let z0 = 0.08;
    screen(k, glow, Vec3::new(0.0, 0.0, z0 + h * 0.5), w, h);
    k.mat(ids::ASPHALT).block(Vec3::new(-w * 0.5, 0.015, z0 + h * 0.2), Vec3::new(w * 0.5, 0.05, z0 + h * 0.75));
    k.mat(ids::IRON);
    if pedestal {
        k.block(Vec3::new(-0.2, -hy * 0.6, 0.0), Vec3::new(0.2, hy * 0.6, 0.012));
        k.block(Vec3::new(-0.03, 0.0, 0.012), Vec3::new(0.03, 0.03, z0 + 0.05));
    } else {
        for s in [-1.0f32, 1.0] {
            let x = s * (w - 0.12);
            k.beam(Vec3::new(x, 0.0, z0 + 0.03), Vec3::new(x + s * 0.06, -hy * 0.7, 0.005), 0.02, 0.01);
            k.beam(Vec3::new(x, 0.0, z0 + 0.03), Vec3::new(x + s * 0.06, hy * 0.7, 0.005), 0.02, 0.01);
        }
    }
    k.mat(ids::ASPHALT);
    cushion(k, Vec3::new(-w * 0.6, -hy, 0.0), Vec3::new(w * 0.6, -hy + 0.09, 0.06));
    k.mat(ids::SLATE).block(Vec3::new(-w * 0.58, -hy - 0.003, 0.008), Vec3::new(w * 0.58, -hy, 0.052));
}

fn pc(b: &mut Ctx, hx: f32, hy: f32) {
    // A desktop PC setup for a desk: a tower with a glass side and lit
    // fans, one or two monitors, keyboard, mouse on a mat, headphones.
    let two = b.r.gen_bool(0.6);
    let (glow, neon) = (b.glow(false), b.glow(true));
    let k = &mut b.kit;
    let t = Vec3::new(hx - 0.13, 0.0, 0.0);
    k.mat(ids::ASPHALT).block(t + Vec3::new(-0.1, -0.22, 0.0), t + Vec3::new(0.1, 0.22, 0.46));
    k.mat(ids::GLASS).block(t + Vec3::new(-0.105, -0.2, 0.03), t + Vec3::new(-0.1, 0.2, 0.43));
    k.mat(neon);
    for z in [0.12, 0.3] {
        k.at(Mat4::from_translation(t + Vec3::new(-0.095, -0.12, z)) * Mat4::from_rotation_y(-FRAC_PI_2), |k| k.frustum(Vec3::ZERO, 0.06, 0.06, 0.01, 12, true));
    }
    k.frustum(t + Vec3::new(0.0, -0.225, 0.42), 0.008, 0.008, 0.005, 6, true);
    // Monitors on stands.
    let mx: &[f32] = if two { &[-0.28, 0.28] } else { &[0.0] };
    let base_x = -hx * 0.25;
    for &dx in mx {
        let m = Vec3::new(base_x + dx, hy - 0.15, 0.0);
        k.mat(ids::IRON).block(m + Vec3::new(-0.1, -0.07, 0.0), m + Vec3::new(0.1, 0.07, 0.01));
        k.block(m + Vec3::new(-0.02, 0.0, 0.01), m + Vec3::new(0.02, 0.03, 0.2));
        screen(k, glow, m + Vec3::new(0.0, -0.01, 0.32), 0.27, 0.32);
    }
    // Keyboard, mouse mat, mouse and headphones.
    k.mat(ids::ASPHALT).block(Vec3::new(base_x - 0.22, -hy + 0.1, 0.0), Vec3::new(base_x + 0.22, -hy + 0.25, 0.02));
    k.mat(ids::SLATE);
    for r in 0..4 {
        k.block(Vec3::new(base_x - 0.21, -hy + 0.115 + r as f32 * 0.033, 0.02), Vec3::new(base_x + 0.21, -hy + 0.14 + r as f32 * 0.033, 0.028));
    }
    let mm = Vec3::new(base_x + 0.38, -hy + 0.18, 0.0);
    k.mat(ids::SLATE).block(mm - Vec3::new(0.14, 0.11, 0.0), mm + Vec3::new(0.14, 0.11, 0.003));
    k.mat(ids::ASPHALT);
    cushion(k, mm + Vec3::new(-0.03, -0.05, 0.003), mm + Vec3::new(0.03, 0.05, 0.04));
    let hp = Vec3::new(base_x - 0.4, -hy + 0.2, 0.0);
    k.frustum(hp + Vec3::new(-0.08, 0.0, 0.0), 0.045, 0.045, 0.03, 10, true);
    k.frustum(hp + Vec3::new(0.08, 0.0, 0.0), 0.045, 0.045, 0.03, 10, true);
    k.beam(hp + Vec3::new(-0.08, 0.0, 0.03), hp + Vec3::new(0.08, 0.0, 0.03), 0.03, 0.012);
}

fn laptop(b: &mut Ctx, hx: f32, hy: f32) {
    // An open laptop: a slim base with keys and a trackpad, the lid
    // opened to about 110 degrees, plus a phone and a mug beside it.
    let case = [ids::IRON, ids::ASPHALT, ids::SLATE][b.r.gen_range(0..3)];
    let glow = b.glow(false);
    let k = &mut b.kit;
    let (w, d) = (0.17, 0.12);
    let c = Vec3::new(-hx + w + 0.02, -hy + d + 0.02, 0.0);
    k.mat(case);
    cushion(k, c - Vec3::new(w, d, 0.0), c + Vec3::new(w, d, 0.018));
    k.mat(ids::ASPHALT).block(c + Vec3::new(-w + 0.02, -0.02, 0.018), c + Vec3::new(w - 0.02, d - 0.02, 0.021));
    k.mat(ids::STUCCO).block(c + Vec3::new(-0.05, -d + 0.02, 0.018), c + Vec3::new(0.05, -0.04, 0.019));
    let hinge = c + Vec3::new(0.0, d, 0.018);
    k.at(Mat4::from_translation(hinge) * Mat4::from_rotation_x(-0.35), |k| {
        k.mat(case).block(Vec3::new(-w, -0.008, 0.0), Vec3::new(w, 0.0, 0.23));
        k.mat(glow).block(Vec3::new(-w + 0.012, -0.01, 0.012), Vec3::new(w - 0.012, -0.008, 0.215));
    });
    let p = Vec3::new(c.x + w + 0.1, c.y - 0.03, 0.0);
    k.mat(ids::ASPHALT);
    cushion(k, p - Vec3::new(0.035, 0.075, 0.0), p + Vec3::new(0.035, 0.075, 0.009));
    k.mat(ids::PLASTER).frustum(Vec3::new(p.x, hy - 0.08, 0.0), 0.04, 0.04, 0.1, 12, true);
}

fn speakers(b: &mut Ctx, hx: f32) {
    // A hi-fi pair: two floor-standing tower speakers with a tweeter,
    // two woofers and a port, and a low amplifier between them.
    let cab = [ids::TRIM, ids::ASPHALT, ids::PLASTER][b.r.gen_range(0..3)];
    let glow = b.glow(true);
    let k = &mut b.kit;
    for s in [-1.0f32, 1.0] {
        let c = Vec3::new(s * (hx - 0.12), 0.0, 0.0);
        k.mat(ids::IRON);
        for (dx, dy) in [(-0.08, -0.1), (0.08, -0.1), (-0.08, 0.1), (0.08, 0.1)] {
            k.frustum(c + Vec3::new(dx, dy, 0.0), 0.01, 0.003, 0.02, 4, true);
        }
        k.mat(cab);
        cushion(k, c + Vec3::new(-0.1, -0.14, 0.02), c + Vec3::new(0.1, 0.14, 1.0));
        let face = |k: &mut Kit, z: f32, r: f32| {
            k.at(Mat4::from_translation(c + Vec3::new(0.0, -0.14, z)) * Mat4::from_rotation_x(FRAC_PI_2), |k| {
                k.mat(ids::IRON).frustum(Vec3::ZERO, r, r, 0.006, 16, true);
                k.mat(ids::ASPHALT).frustum(Vec3::new(0.0, 0.0, 0.006), r * 0.85, r * 0.2, 0.012, 16, true);
            });
        };
        face(k, 0.88, 0.03);
        face(k, 0.7, 0.075);
        face(k, 0.5, 0.075);
        k.at(Mat4::from_translation(c + Vec3::new(0.0, -0.14, 0.2)) * Mat4::from_rotation_x(FRAC_PI_2), |k| k.mat(ids::ASPHALT).frustum(Vec3::ZERO, 0.03, 0.03, 0.01, 10, true));
    }
    let w = (hx - 0.3).min(0.22);
    k.mat(ids::IRON).block(Vec3::new(-w, -0.15, 0.0), Vec3::new(w, 0.15, 0.12));
    k.mat(glow).block(Vec3::new(-w * 0.4, -0.155, 0.07), Vec3::new(w * 0.2, -0.15, 0.09));
    dial(k, Vec3::new(w * 0.6, -0.15, 0.06), 0.025);
    dial(k, Vec3::new(-w * 0.7, -0.15, 0.06), 0.015);
}
