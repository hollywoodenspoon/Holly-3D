//! Watchtower / Castle Keep on a Cylinder (round) or Box (square): flared
//! base plinth, staggered stone blocks and corner quoins, arched windows
//! per floor, arrow slits, an arched wooden door with steps, overhanging
//! machicolations, crenellations and an optional conical / pyramid spire.

use super::parts::{self, Masonry};
use super::{ArchLayout, ArchitectMod};
use crate::generators::kit::{Kit, ids};
use crate::generators::kit::sides;
use bevy::math::{Mat4, Vec2, Vec3};
use std::f32::consts::{FRAC_PI_2, TAU};

pub fn build(kit: &mut Kit, a: &ArchitectMod, s: Vec3, round: bool, lay: &ArchLayout, m: &Masonry) {
    let h = s.z;
    let plinth = (h * 0.08).clamp(0.4, 2.5);
    if round {
        let r = s.x.min(s.y) * 0.5;
        kit.mat(ids::FACADE).frustum(Vec3::ZERO, r * 1.15, r, plinth, 32, true);
        parts::round_tower(kit, Vec3::ZERO, r, h, &Masonry { crenels: false, ..m.clone() }, false);
        // Openings around the ring, one column per window slot.
        let n = lay.windows_per_floor.max(1);
        for i in 0..n {
            let ang = TAU * (i as f32 + 0.5) / n as f32;
            let f = Mat4::from_rotation_z(ang + FRAC_PI_2) * Mat4::from_translation(Vec3::new(0.0, -r - m.depth - 0.03, 0.0));
            kit.at(f, |k| openings(k, a, lay, h, i == 0, i % 2 == 1));
        }
        top(kit, a, |k, out| {
            let ring = TAU * r;
            let segs = (ring / 1.6).round().max(8.0) as usize;
            for i in 0..segs {
                let ang = TAU * i as f32 / segs as f32;
                let fr = Mat4::from_rotation_z(ang + FRAC_PI_2) * Mat4::from_translation(Vec3::new(0.0, -r + 0.3, 0.0));
                let len = ring / segs as f32 + 0.15;
                k.at(fr, |k| {
                    if out > 0.0 {
                        parts::machicolations(k, len, h, 0.6, out);
                    }
                    k.mat(ids::FACADE).block(Vec3::new(-len * 0.5, -0.3 - out, h + if out > 0.0 { 0.9 } else { 0.0 }), Vec3::new(len * 0.5, 0.3 - out, h + 1.0));
                });
            }
            if a.crenellations {
                parts::merlon_ring(k, Vec3::Z * (h + 1.0), r + out, m);
            }
        });
        if a.roof {
            kit.mat(ids::ROOF).frustum(Vec3::Z * (h + 0.2), r * 0.85, 0.0, r * 2.4, 24, true);
        }
    } else {
        let hf = Vec2::new(s.x, s.y) * 0.5;
        kit.mat(ids::FACADE).prism(&battered(hf, 0.6 + plinth * 0.25), 0.0, plinth, true, true);
        kit.mat(if m.bricks { ids::PLASTER } else { ids::FACADE }).block((-hf).extend(0.0), hf.extend(h));
        sides(kit, hf.x, hf.y, |k, side, w| {
            if m.bricks {
                k.at(Mat4::from_translation(Vec3::Y * 0.03), |k| {
                    parts::brick_face(k, w - 0.5, 0.0, h, m);
                    quoins(k, w, h, m);
                });
            }
            k.at(Mat4::from_translation(Vec3::Y * -(m.depth)), |k| openings(k, a, lay, h, side == 0, false));
        });
        top(kit, a, |k, out| {
            sides(k, hf.x, hf.y, |k, _, w| {
                k.at(Mat4::from_translation(Vec3::Y * 0.33), |k| {
                    if out > 0.0 {
                        parts::machicolations(k, w, h, 0.6, out);
                    }
                    k.mat(ids::FACADE).block(Vec3::new(-w * 0.5 - out, -0.3 - out, h + if out > 0.0 { 0.9 } else { 0.0 }), Vec3::new(w * 0.5 + out, 0.3 - out, h + 1.0));
                    if a.crenellations {
                        k.at(Mat4::from_translation(Vec3::Y * -out), |k| parts::crenellate(k, w + 2.0 * out, h + 1.0, 0.6, m));
                    }
                });
            });
        });
        if a.roof {
            kit.at(Mat4::from_translation(Vec3::Z * (h + 0.2)), |k| k.mat(ids::ROOF).hipped(hf.x * 0.85, hf.y * 0.85, hf.min_element() * 2.2, 0.0));
        }
    }
}

/// Machicolations (when enabled, `out` > 0) and the parapet top.
fn top(kit: &mut Kit, a: &ArchitectMod, f: impl FnOnce(&mut Kit, f32)) {
    f(kit, if a.machicolations { 0.6 } else { 0.0 });
}

/// Outline of a battered plinth `grow` metres wider than the footprint.
fn battered(hf: Vec2, grow: f32) -> [Vec2; 4] {
    let g = hf + grow;
    [Vec2::new(-g.x, -g.y), Vec2::new(g.x, -g.y), Vec2::new(g.x, g.y), Vec2::new(-g.x, g.y)]
}

/// Alternating long / short corner quoin blocks on both edges of a face.
fn quoins(kit: &mut Kit, w: f32, h: f32, m: &Masonry) {
    let n = parts::course_count(h, m.brick.y * 1.5);
    let ch = h / n as f32;
    kit.mat(ids::FACADE);
    for i in 0..n {
        let len = if i % 2 == 0 { m.brick.x * 1.2 } else { m.brick.x * 0.7 };
        let z = i as f32 * ch;
        for s in [-1.0f32, 1.0] {
            let (x0, x1) = (s * w * 0.5, s * (w * 0.5 - len));
            kit.block(Vec3::new(x0.min(x1), -m.depth * 1.6, z + 0.02), Vec3::new(x0.max(x1), 0.0, z + ch - 0.02));
        }
    }
}

/// One column of openings on a face frame: an arched window per floor
/// (or arrow slits), and on the front the arched door with steps.
fn openings(k: &mut Kit, a: &ArchitectMod, lay: &ArchLayout, h: f32, front: bool, slits_only: bool) {
    let fh = h / lay.floors.max(1) as f32;
    let per = if front { 1 } else { 0 };
    if a.windows {
        for f in per..lay.floors as usize {
            let z = f as f32 * fh + fh * 0.3;
            if z + 1.6 > h - 0.5 {
                break;
            }
            if slits_only || f == 0 {
                parts::arrow_slit(k, 0.0, z, (fh * 0.5).min(1.6));
            } else {
                k.at(Mat4::from_translation(Vec3::new(0.0, -0.02, z)), |k| k.mat(ids::GLASS).arched_panel(0.9, (fh * 0.5).clamp(1.0, 2.2), false));
            }
        }
    }
    if front && a.door {
        k.at(Mat4::from_translation(Vec3::new(0.0, -0.03, 0.6)), |k| k.mat(ids::TRIM).arched_panel(1.6, 2.8, false));
        for i in 0..3 {
            let d = 0.45 * (3 - i) as f32;
            k.mat(ids::FACADE).block(Vec3::new(-1.3, -d - 0.05, i as f32 * 0.2), Vec3::new(1.3, 0.0, i as f32 * 0.2 + 0.2));
        }
    }
}
