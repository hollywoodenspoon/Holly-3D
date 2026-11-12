//! Layered rubble mounds: a dust heap (ID 27) with ash patches, then the
//! era's debris on its surface (concrete chunks, bricks, rebar and
//! girders for Modern; stone blocks, roof tiles and charred timbers for
//! Medieval; big weathered stones, column drums and moss for Ancient),
//! glowing embers (ID 25) in the crevices and blood smears (ID 28).

use super::damage::decal;
use super::{Foot, RuinEra as Era, RuinParams};
use crate::generators::kit::{Kit, ids, rng};
use crate::mesh::{EditablePoly, VertexId};
use crate::modifiers::battlefield::debris;
use bevy::math::{EulerRot, Quat, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

/// Heap height at `p` (no noise), for placing debris on the surface.
fn surf(f: &Foot, h: f32, p: Vec2) -> f32 {
    let q = (p - f.c) / f.half.max(Vec2::splat(0.1));
    f.z + h * (1.0 - q.length_squared()).max(0.0).powf(0.8)
}

/// A random point inside the footprint ellipse.
fn spot(f: &Foot, r: &mut ChaCha8Rng) -> Vec2 {
    let (a, d) = (r.gen_range(0.0..TAU), r.gen_range(0.0f32..1.0).sqrt() * 0.95);
    f.c + Vec2::from_angle(a) * f.half * d
}

fn rot(r: &mut ChaCha8Rng) -> Quat {
    Quat::from_euler(EulerRot::ZXY, r.gen_range(0.0..TAU), r.gen_range(-0.7..0.7), r.gen_range(-0.7..0.7))
}

/// The dust heap: rings of quads around an apex, rough edge on the ground.
fn heap(kit: &mut Kit, f: &Foot, h: f32, r: &mut ChaCha8Rng) {
    let (rings, segs) = (6usize, 20usize);
    let phase: Vec<f32> = (0..3).map(|_| r.gen_range(0.0..TAU)).collect();
    let rough = |t: f32, k: f32| {
        1.0 + 0.18 * ((t * 3.0 + phase[0]).sin() + (t * 5.0 + phase[1]).sin() * 0.6 + (k * 7.0 + t * 2.0 + phase[2]).sin() * 0.4)
    };
    kit.mat(ids::DUST);
    let apex = kit.vtx((f.c).extend(f.z + h * rough(0.0, 0.0)));
    let mut rows: Vec<Vec<VertexId>> = vec![];
    for i in 1..=rings {
        let k = i as f32 / rings as f32;
        let row = (0..segs)
            .map(|j| {
                let t = TAU * j as f32 / segs as f32;
                let rad = k * if i == rings { rough(t, 1.0).clamp(0.85, 1.2) } else { 1.0 };
                let p = f.c + Vec2::from_angle(t) * f.half * rad;
                let z = if i == rings {
                    f.z - 0.05
                } else {
                    f.z + h * ((1.0 - k * k).powf(0.8) * rough(t, k) + r.gen_range(-0.09..0.09)).max(0.02)
                };
                kit.vtx(p.extend(z))
            })
            .collect();
        rows.push(row);
    }
    for j in 0..segs {
        kit.face(&[apex, rows[0][j], rows[0][(j + 1) % segs]]);
    }
    for i in 0..rings - 1 {
        for j in 0..segs {
            let jn = (j + 1) % segs;
            kit.face(&[rows[i][j], rows[i + 1][j], rows[i + 1][jn], rows[i][jn]]);
        }
    }
}

/// Embers and blood on the heap (or on the ground for Light damage).
fn extras(kit: &mut Kit, f: &Foot, h: f32, n: usize, p: &RuinParams, r: &mut ChaCha8Rng) {
    if p.embers {
        for _ in 0..(n / 3).max(4) {
            let q = spot(f, r);
            let c = q.extend(surf(f, h, q) + 0.04);
            let k = r.gen_range(5..8);
            let rad = r.gen_range(0.08..0.3);
            let pts: Vec<Vec3> = (0..k)
                .map(|i| c + Vec2::from_angle(TAU * i as f32 / k as f32).extend(r.gen_range(-0.02..0.02)) * rad * r.gen_range(0.6..1.2))
                .collect();
            kit.mat(ids::EMBERS).poly(&pts);
        }
    }
    if p.gore && p.gore_amount > 0.0 {
        let count = ((p.gore_amount / 100.0) * 14.0).ceil() as usize;
        for _ in 0..count {
            let q = spot(&Foot { half: f.half * 1.3, ..*f }, r);
            let c = q.extend(surf(f, h, q) + 0.035);
            let k = r.gen_range(7..11);
            let rad = r.gen_range(0.3..1.1);
            let pts: Vec<Vec3> = (0..k)
                .map(|i| {
                    c + Vec2::from_angle(TAU * i as f32 / k as f32 + r.gen_range(-0.2..0.2)).extend(0.0) * rad * r.gen_range(0.45..1.25)
                })
                .collect();
            kit.mat(ids::BLOOD).poly(&pts);
            if r.gen_bool(0.5) {
                let d = Vec2::from_angle(r.gen_range(0.0..TAU));
                let s = c + d.extend(0.0) * rad * 1.6;
                decal(kit, s.with_z(surf(f, h, s.truncate()) + 0.035), Vec3::Z, rad * 1.4, rad * 0.25, d.y.atan2(d.x), ids::BLOOD);
            }
        }
    }
}

/// A full mound of height `h` over the footprint.
pub(super) fn mound(kit: &mut Kit, f: &Foot, h: f32, era: Era, p: &RuinParams, r: &mut ChaCha8Rng) {
    heap(kit, f, h, r);
    let area = std::f32::consts::PI * f.half.x * f.half.y;
    let n = ((area * p.rubble.clamp(0.0, 1.0) * 0.5) as usize).clamp(6, 320);
    for _ in 0..n {
        let q = spot(f, r);
        let z = surf(f, h, q);
        let roll = r.gen_range(0.0..1.0f32);
        match era {
            Era::Modern | Era::Auto => {
                if roll < 0.45 {
                    let half = Vec3::new(r.gen_range(0.2..0.9), r.gen_range(0.2..0.7), r.gen_range(0.1..0.35));
                    kit.mat(ids::FACADE).cuboid(q.extend(z + half.z * 0.3), half, rot(r));
                } else if roll < 0.8 {
                    kit.mat(ids::BRICK).cuboid(q.extend(z + 0.04), Vec3::new(0.11, 0.055, 0.035) * 1.6, rot(r));
                } else if roll < 0.94 {
                    debris::rubble(kit, r, q.extend(z), 0.3, 1, ids::FACADE, true);
                } else {
                    let d = Vec2::from_angle(r.gen_range(0.0..TAU)) * r.gen_range(1.5..4.0);
                    kit.mat(ids::IRON).beam(q.extend(z + 0.1), (q + d).extend(surf(f, h, q + d) + r.gen_range(0.1..1.2)), 0.2, 0.3);
                }
            }
            Era::Medieval => {
                if roll < 0.5 {
                    let half = Vec3::new(r.gen_range(0.2..0.5), r.gen_range(0.15..0.35), r.gen_range(0.12..0.25));
                    kit.mat(ids::FACADE).cuboid(q.extend(z + half.z * 0.3), half, rot(r));
                } else if roll < 0.75 {
                    kit.mat(ids::ROOF).cuboid(q.extend(z + 0.03), Vec3::new(0.16, 0.12, 0.015), rot(r));
                } else {
                    let d = Vec2::from_angle(r.gen_range(0.0..TAU)) * r.gen_range(1.2..3.5);
                    kit.mat(ids::CHARRED).beam(q.extend(z + 0.1), (q + d).extend(surf(f, h, q + d) + r.gen_range(0.1..0.9)), 0.22, 0.24);
                }
            }
            Era::Ancient => {
                if roll < 0.85 {
                    let half = Vec3::new(r.gen_range(0.3..0.8), r.gen_range(0.25..0.6), r.gen_range(0.2..0.45));
                    kit.mat(ids::FACADE).cuboid(q.extend(z + half.z * 0.2), half, rot(r));
                } else if roll < 0.93 {
                    debris::drums(kit, q.extend(z), Vec2::from_angle(r.gen_range(0.0..TAU)), 0.4, 2.0);
                } else {
                    decal(kit, q.extend(z), Vec3::Z, r.gen_range(0.8..2.0), r.gen_range(0.6..1.4), r.gen_range(0.0..3.0), ids::FOLIAGE);
                }
            }
        }
    }
    // Ash: darker soot patches spread over the dust.
    if era != Era::Ancient {
        for _ in 0..(n / 10).max(2) {
            let q = spot(f, r);
            decal(kit, q.extend(surf(f, h, q)), Vec3::Z, r.gen_range(0.8..2.5), r.gen_range(0.6..1.8), r.gen_range(0.0..3.0), ids::SCAR);
        }
    }
    extras(kit, f, h, n, p, r);
}

/// Light damage: blood and embers on the ground around the base only.
pub(super) fn decor(kit: &mut Kit, f: &Foot, _era: Era, p: &RuinParams, r: &mut ChaCha8Rng) {
    let ring = Foot { half: f.half + Vec2::splat(1.5), ..*f };
    let q = RuinParams { embers: false, ..p.clone() };
    extras(kit, &ring, 0.0, 0, &q, r);
}

/// The standalone Rubble Pile: a mound `spread` (half size) across and
/// `p.pile_height` high, centred on the origin, standing on z = 0.
pub fn rubble_pile(spread: Vec2, p: &RuinParams, medieval: bool) -> EditablePoly {
    let mut kit = Kit::new();
    let mut r = rng(p.seed, 0x9113);
    let f = Foot { c: Vec2::ZERO, half: spread.max(Vec2::splat(0.3)), z: 0.0 };
    mound(&mut kit, &f, p.pile_height.clamp(0.1, 50.0), p.era.resolve(medieval), p, &mut r);
    kit.finish()
}
