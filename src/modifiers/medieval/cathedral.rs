//! Gothic Cathedral Vault & Colonnade on a Box (nave along its long axis):
//! side walls with staggered masonry and pointed lancet windows with stone
//! tracery, an interior colonnade carrying pointed transverse ribs and
//! diagonal rib vaults, flying buttresses with pinnacles, a west portal
//! with a rose window, and a steep gabled roof. The bay count follows the
//! nave length, so a longer base gains bays instead of stretching them.

use super::parts::{self, Masonry};
use super::{ArchLayout, ArchitectMod};
use crate::generators::kit::roofs::arch_curve;
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use std::f32::consts::{FRAC_PI_2, PI};

pub fn build(kit: &mut Kit, a: &ArchitectMod, s: Vec3, lay: &ArchLayout, m: &Masonry) {
    let along_x = s.x >= s.y;
    let (len, w, h) = (s.x.max(s.y), s.x.min(s.y).max(3.0), s.z.max(3.0));
    let f = if along_x { Mat4::IDENTITY } else { Mat4::from_rotation_z(FRAC_PI_2) };
    let t = a.wall_thickness.clamp(0.4, w * 0.15);
    let hw = if a.roof { h * 0.6 } else { h };
    let bays = lay.buttresses.max(2) as usize;
    let bay = len / bays as f32;
    kit.at(f, |k| {
        // Side walls, then the west and east end walls between them.
        let core = if m.bricks { ids::PLASTER } else { ids::FACADE };
        for sy in [-1.0f32, 1.0] {
            let (y0, y1) = (sy * w * 0.5, sy * (w * 0.5 - t));
            k.mat(core).block(Vec3::new(-len * 0.5, y0.min(y1), 0.0), Vec3::new(len * 0.5, y0.max(y1), hw));
            if m.bricks {
                let side = if sy < 0.0 { 0.0 } else { PI };
                k.at(Mat4::from_rotation_z(side) * Mat4::from_translation(Vec3::new(0.0, -w * 0.5, 0.0)), |k| {
                    parts::brick_face(k, len, 0.0, hw, m);
                });
            }
            if a.crenellations {
                k.at(Mat4::from_translation(Vec3::Y * sy * (w * 0.5 - t * 0.5)), |k| parts::crenellate(k, len, hw, t, m));
            }
        }
        for sx in [-1.0f32, 1.0] {
            let (x0, x1) = (sx * len * 0.5, sx * (len * 0.5 - t));
            k.mat(core).block(Vec3::new(x0.min(x1), -w * 0.5 + t, 0.0), Vec3::new(x0.max(x1), w * 0.5 - t, hw));
        }
        k.mat(ids::LOT).quad(
            Vec3::new(-len * 0.5 + t, -w * 0.5 + t, 0.02),
            Vec3::new(len * 0.5 - t, -w * 0.5 + t, 0.02),
            Vec3::new(len * 0.5 - t, w * 0.5 - t, 0.02),
            Vec3::new(-len * 0.5 + t, w * 0.5 - t, 0.02),
        );
        if a.windows {
            for i in 0..bays {
                let x = -len * 0.5 + bay * (i as f32 + 0.5);
                for side in [0.0, PI] {
                    let fr = Mat4::from_rotation_z(side) * Mat4::from_translation(Vec3::new(if side == 0.0 { x } else { -x }, -w * 0.5 - m.depth - 0.03, 0.0));
                    k.at(fr, |k| lancet(k, (bay * 0.38).clamp(0.6, 3.5), hw * 0.55, hw * 0.22));
                }
            }
        }
        vault(k, len, w, h, hw, t, bays);
        if a.buttresses {
            for i in 0..=bays {
                let x = -len * 0.5 + bay * i as f32;
                for sy in [-1.0f32, 1.0] {
                    flyer(k, x, sy, w, hw, (w * 0.25).clamp(2.0, 8.0));
                }
            }
        }
        if a.door || a.windows {
            k.at(Mat4::from_rotation_z(-FRAC_PI_2) * Mat4::from_translation(Vec3::new(0.0, -len * 0.5 - 0.04, 0.0)), |k| portal(k, a, w, hw));
        }
        if a.roof {
            k.at(Mat4::from_translation(Vec3::Z * hw), |k| k.mat(ids::ROOF).gable(len * 0.5, w * 0.5, h - hw, 0.4, ids::FACADE));
        }
    });
}

/// A pointed lancet window (glass) with a central mullion, a transom at the
/// springing and a small oculus in the head: the stone tracery.
fn lancet(k: &mut Kit, w: f32, h: f32, z: f32) {
    k.at(Mat4::from_translation(Vec3::Z * z), |k| {
        k.mat(ids::GLASS).arched_panel(w, h, true);
        let rise = w * 0.75;
        k.mat(ids::FACADE);
        let bar = (w * 0.07).clamp(0.06, 0.2);
        k.block(Vec3::new(-bar * 0.5, -0.08, 0.0), Vec3::new(bar * 0.5, 0.0, h - rise * 0.55));
        k.block(Vec3::new(-w * 0.5, -0.08, h - rise - bar * 0.5), Vec3::new(w * 0.5, 0.0, h - rise + bar * 0.5));
        let oc = Mat4::from_rotation_translation(Quat::from_rotation_x(FRAC_PI_2), Vec3::new(0.0, -0.02, h - rise * 0.45));
        k.at(oc, |k| k.frustum(Vec3::ZERO, w * 0.18, w * 0.18, 0.08, 12, true));
    });
}

/// Interior colonnade, pointed transverse ribs, diagonal ribs and the
/// vault web of each bay.
fn vault(k: &mut Kit, len: f32, w: f32, h: f32, hw: f32, t: f32, bays: usize) {
    let iw = w - 2.0 * t - 1.6;
    if iw < 1.0 {
        return;
    }
    let col = 0.45f32.min(iw * 0.1);
    let spring = (hw - iw * 0.5).max(hw * 0.4);
    let rise = (iw * 0.55).min((h - spring) * 0.85).max(0.3);
    let pointed = rise >= iw * 0.5;
    let bay = (len - 2.0 * t) / bays as f32;
    let x_at = |i: usize| -len * 0.5 + t + bay * i as f32;
    let tr = arch_curve(iw, rise, pointed, 12);
    let rib = (iw * 0.03).clamp(0.15, 0.4);
    for i in 0..=bays {
        let x = x_at(i);
        for sy in [-1.0f32, 1.0] {
            let c = Vec3::new(x, sy * iw * 0.5, 0.0);
            k.mat(ids::FACADE).block(c - Vec3::new(col * 1.4, col * 1.4, 0.0), c + Vec3::new(col * 1.4, col * 1.4, 0.4));
            k.frustum(c + Vec3::Z * 0.4, col, col * 0.9, spring - 0.8, 12, false);
            k.block(c + Vec3::new(-col * 1.3, -col * 1.3, spring - 0.4), c + Vec3::new(col * 1.3, col * 1.3, spring));
        }
        for p in tr.windows(2) {
            k.beam(Vec3::new(x, p[0].x, spring + p[0].y), Vec3::new(x, p[1].x, spring + p[1].y), rib, rib);
        }
    }
    for i in 0..bays {
        let (x0, x1) = (x_at(i), x_at(i + 1));
        // Diagonal ribs: corner to corner, crowning at the transverse height.
        for (a, b) in [(Vec2::new(x0, -iw * 0.5), Vec2::new(x1, iw * 0.5)), (Vec2::new(x0, iw * 0.5), Vec2::new(x1, -iw * 0.5))] {
            let d = arch_curve(a.distance(b), rise, false, 12);
            let at = |q: Vec2| {
                let s = (q.x / a.distance(b) + 0.5).clamp(0.0, 1.0);
                (a + (b - a) * s).extend(spring + q.y)
            };
            for p in d.windows(2) {
                k.mat(ids::FACADE).beam(at(p[0]), at(p[1]), rib * 0.8, rib * 0.8);
            }
        }
        // Web: a pointed barrel between the two transverse ribs, facing down.
        k.mat(ids::PLASTER);
        for p in tr.windows(2) {
            let (a, b) = (p[0], p[1]);
            let z = spring + rib * 0.5;
            k.quad(Vec3::new(x0, b.x, z + b.y), Vec3::new(x1, b.x, z + b.y), Vec3::new(x1, a.x, z + a.y), Vec3::new(x0, a.x, z + a.y));
        }
    }
}

/// Flying buttress on side `sy` at x: a stepped pier `d` out from the wall,
/// a pinnacle, and a sloping arch-beam leaning against the clerestory.
fn flyer(k: &mut Kit, x: f32, sy: f32, w: f32, hw: f32, d: f32) {
    let pw = (d * 0.25).clamp(0.6, 1.6);
    let py = sy * (w * 0.5 + d);
    let ph = hw * 0.75;
    k.mat(ids::FACADE);
    k.block(Vec3::new(x - pw * 0.5, py - pw * 0.5, 0.0), Vec3::new(x + pw * 0.5, py + pw * 0.5, ph));
    k.block(Vec3::new(x - pw * 0.6, py - pw * 0.6, 0.0), Vec3::new(x + pw * 0.6, py + pw * 0.6, ph * 0.3));
    k.frustum(Vec3::new(x, py, ph), pw * 0.45, 0.0, pw * 2.5, 4, true);
    // Wall buttress and the flying arch.
    let wy = sy * w * 0.5;
    k.block(Vec3::new(x - pw * 0.35, wy.min(wy + sy * pw * 0.5), 0.0), Vec3::new(x + pw * 0.35, wy.max(wy + sy * pw * 0.5), hw * 0.9));
    let from = Vec3::new(x, py - sy * pw * 0.5, ph * 0.92);
    let to = Vec3::new(x, wy + sy * pw * 0.5, hw * 0.93);
    k.beam(from, to, pw * 0.5, 0.5);
    let mid = (from + to) * 0.5 - Vec3::Z * 0.6;
    k.beam(from - Vec3::Z * 0.8, mid, pw * 0.4, 0.4);
    k.beam(mid, to - Vec3::Z * 0.8, pw * 0.4, 0.4);
}

/// West front (face frame): a pointed portal with stepped archivolts and
/// the rose window over it.
fn portal(k: &mut Kit, a: &ArchitectMod, w: f32, hw: f32) {
    let dw = (w * 0.3).clamp(1.6, 6.0);
    let dh = (hw * 0.45).clamp(2.5, 9.0);
    if a.door {
        for (i, grow) in [0.9f32, 0.45, 0.0].into_iter().enumerate() {
            k.at(Mat4::from_translation(Vec3::new(0.0, -0.1 * (3 - i) as f32, 0.0)), |k| {
                k.mat(if i == 2 { ids::TRIM } else { ids::FACADE }).arched_panel(dw + grow * 2.0, dh + grow, true);
            });
        }
    }
    if a.windows {
        let r = (w * 0.18).min((hw - dh - 1.5) * 0.45).max(0.4);
        let c = Vec3::new(0.0, -0.05, (dh + 1.0 + r).min(hw - r - 0.3).max(r + 0.2));
        let f = Mat4::from_rotation_translation(Quat::from_rotation_x(FRAC_PI_2), c);
        k.at(f, |k| {
            k.mat(ids::GLASS).frustum(Vec3::ZERO, r, r, 0.05, 24, true);
            k.mat(ids::FACADE).frustum(Vec3::Z * 0.05, r * 0.25, r * 0.25, 0.1, 12, true);
            for i in 0..8 {
                let ang = PI * i as f32 / 8.0;
                let dir = Vec3::new(ang.cos(), ang.sin(), 0.0) * r;
                k.beam(-dir + Vec3::Z * 0.1, dir + Vec3::Z * 0.1, 0.08, 0.08);
            }
        });
    }
}
