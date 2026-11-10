//! Arched Stone Bridge / Aqueduct on a Box (along its long axis): piers
//! with pointed cutwaters, round arches with radial voussoir blocks, a
//! string course, a cobbled deck and parapets. Tall bases stack into
//! aqueduct tiers, the top tier carrying a row of smaller arches.

use super::parts::{self, Masonry};
use super::{ArchLayout, ArchitectMod};
use crate::generators::kit::roofs::arch_curve;
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

/// Tiers of arches for a structure `h` tall (one up to 12 m).
pub(super) fn tiers(h: f32) -> usize {
    ((h / 12.0).ceil() as usize).clamp(1, 3)
}

/// Arches in the bottom tier: spans about as wide as a tier is tall.
pub(super) fn span_count(long: f32, height: f32) -> usize {
    let th = height / tiers(height) as f32;
    let target = (th * 0.9).clamp(3.0, 24.0);
    (long / target).round().clamp(1.0, 60.0) as usize
}

pub fn build(kit: &mut Kit, a: &ArchitectMod, s: Vec3, lay: &ArchLayout, m: &Masonry) {
    let along_x = s.x >= s.y;
    let (len, w, h) = (s.x.max(s.y), s.x.min(s.y).max(1.0), s.z.max(1.0));
    let f = if along_x { Mat4::IDENTITY } else { Mat4::from_rotation_z(FRAC_PI_2) };
    let n_tiers = tiers(h);
    let parapet = 1.0f32.min(h * 0.2);
    let th = h / n_tiers as f32;
    kit.at(f, |k| {
        for t in 0..n_tiers {
            let spans = if t + 1 == n_tiers && n_tiers > 1 { lay.arches.max(1) as usize * 3 } else { lay.arches.max(1) as usize };
            let tw = w * (1.0 - 0.08 * t as f32);
            tier(k, a, m, len, tw, t as f32 * th, th, spans, t == 0);
        }
        // Deck: cobbled road between parapets.
        let pt = (w * 0.08).clamp(0.3, 0.6);
        let tw = w * (1.0 - 0.08 * (n_tiers - 1) as f32);
        k.mat(ids::ROAD).quad(
            Vec3::new(-len * 0.5, -tw * 0.5 + pt, h + 0.01),
            Vec3::new(len * 0.5, -tw * 0.5 + pt, h + 0.01),
            Vec3::new(len * 0.5, tw * 0.5 - pt, h + 0.01),
            Vec3::new(-len * 0.5, tw * 0.5 - pt, h + 0.01),
        );
        for sy in [-1.0f32, 1.0] {
            let (y0, y1) = (sy * tw * 0.5, sy * (tw * 0.5 - pt));
            k.mat(ids::FACADE).block(Vec3::new(-len * 0.5, y0.min(y1), h), Vec3::new(len * 0.5, y0.max(y1), h + parapet));
            // Coping stones along the parapet.
            k.block(Vec3::new(-len * 0.5, y0.min(y1) - 0.05, h + parapet), Vec3::new(len * 0.5, y0.max(y1) + 0.05, h + parapet + 0.15));
            if a.crenellations {
                k.at(Mat4::from_translation(Vec3::Y * (y0 + y1) * 0.5), |k| parts::crenellate(k, len, h + parapet + 0.15, pt, m));
            }
        }
    });
}

/// One tier of `spans` arches from `z0`, `th` tall and `w` wide.
#[allow(clippy::too_many_arguments)]
fn tier(k: &mut Kit, a: &ArchitectMod, m: &Masonry, len: f32, w: f32, z0: f32, th: f32, spans: usize, bottom: bool) {
    let step = len / spans as f32;
    let pw = (step * 0.2).clamp(0.5, 4.0);
    let sw = ((len - pw) / spans as f32 - pw).max(0.5);
    let band = (th * 0.12).clamp(0.3, 1.2);
    let rise = (sw * 0.5).min(th - band - 0.5).max(0.2);
    let spring = z0 + th - band - rise;
    let pier_x = |i: usize| -len * 0.5 + pw * 0.5 + i as f32 * (len - pw) / spans as f32;
    for i in 0..=spans {
        let x = pier_x(i);
        k.mat(ids::FACADE).block(Vec3::new(x - pw * 0.5, -w * 0.5, z0), Vec3::new(x + pw * 0.5, w * 0.5, spring));
        if m.bricks && spring - z0 > m.brick.y {
            for side in [0.0, std::f32::consts::PI] {
                let fr = Mat4::from_rotation_z(side) * Mat4::from_translation(Vec3::new(if side == 0.0 { x } else { -x }, -w * 0.5, z0));
                k.at(fr, |k| {
                    parts::brick_face(k, pw, 0.0, spring - z0, m);
                });
            }
        }
        if bottom && a.buttresses && i > 0 && i < spans {
            let ch = (spring - z0) * 0.75;
            k.mat(ids::FACADE);
            k.prism(&[Vec2::new(x - pw * 0.5, -w * 0.5), Vec2::new(x, -w * 0.5 - pw * 0.8), Vec2::new(x + pw * 0.5, -w * 0.5)], z0, ch, true, false);
            k.prism(&[Vec2::new(x + pw * 0.5, w * 0.5), Vec2::new(x, w * 0.5 + pw * 0.8), Vec2::new(x - pw * 0.5, w * 0.5)], z0, ch, true, false);
        }
    }
    let top = z0 + th - band;
    for i in 0..spans {
        let cx = (pier_x(i) + pier_x(i + 1)) * 0.5;
        k.at(Mat4::from_translation(Vec3::new(cx, 0.0, spring)), |k| {
            k.mat(ids::FACADE).arch_wall(sw, rise, top - spring, w, false);
            if m.bricks {
                voussoirs(k, sw, rise, w, m);
            }
        });
    }
    // String course: closes the top of the arches and marks the tier.
    k.mat(ids::FACADE).block(Vec3::new(-len * 0.5 - 0.1, -w * 0.5 - 0.1, top), Vec3::new(len * 0.5 + 0.1, w * 0.5 + 0.1, z0 + th));
}

/// Radial wedge blocks around the arch ring on both faces.
fn voussoirs(k: &mut Kit, sw: f32, rise: f32, w: f32, m: &Masonry) {
    let arc_len = std::f32::consts::PI * 0.5 * (sw * 0.5 + rise);
    let n = (arc_len / m.brick.y.max(0.1) / 1.5).round().clamp(5.0, 61.0) as usize;
    let curve = arch_curve(sw, rise, false, n);
    let depth = (sw * 0.12).clamp(0.35, 1.2);
    k.mat(ids::FACADE);
    for seg in curve.windows(2) {
        let (p, q) = (seg[0], seg[1]);
        let mid = (p + q) * 0.5;
        // Outward normal of the ellipse at the segment.
        let out = Vec2::new(mid.x / (sw * 0.5).powi(2), mid.y / rise.powi(2)).normalize_or(Vec2::Y);
        let h = p.distance(q) * 0.85;
        for y in [-w * 0.5 - m.depth * 0.5, w * 0.5 + m.depth * 0.5] {
            let a3 = Vec3::new(mid.x, y, mid.y);
            k.beam(a3, a3 + Vec3::new(out.x, 0.0, out.y) * depth, m.depth, h);
        }
    }
}
