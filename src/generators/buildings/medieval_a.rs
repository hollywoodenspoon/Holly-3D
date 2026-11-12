//! Medieval civic buildings: castle keep, guild hall, market hall,
//! half-timber tavern, stone chapel and gatehouse.

use super::arch::{Opening, StairKind, stairs, wall_with_openings};
use super::parts;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids, sides};
use crate::generators::settlement::timber_frame;
use bevy::math::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_2;

const FH: f32 = 3.1;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::CastleKeep => keep(b, hx, hy, floors),
        K::GuildHall => hall(b, hx, hy, floors, true),
        K::MarketHall => market_hall(b, hx, hy, floors),
        K::Tavern => hall(b, hx, hy, floors, false),
        K::StoneChapel => chapel(b, hx, hy),
        K::Gatehouse => gatehouse(b, hx, hy, floors),
        _ => {}
    }
}

/// Crenellated parapet around a `2hx` x `2hy` roof at `z`.
pub(super) fn battlements(kit: &mut Kit, hx: f32, hy: f32, z: f32) {
    kit.mat(ids::FACADE);
    sides(kit, hx, hy, |k, _, w| {
        k.block(Vec3::new(-w * 0.5, 0.0, z), Vec3::new(w * 0.5, 0.6, z + 0.9));
        let n = ((w / 1.2).floor() as usize).max(2);
        for i in (0..n).step_by(2) {
            let x = -w * 0.5 + w * i as f32 / n as f32;
            k.block(Vec3::new(x, 0.0, z + 0.9), Vec3::new(x + w / n as f32, 0.6, z + 1.7));
        }
    });
}

fn keep(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let fh = 4.0;
    let h = fh * floors as f32;
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
    // Corner turrets, arrow slits, raised door with stairs, battlements.
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        b.kit.frustum(Vec3::new(sx * hx, sy * hy, 0.0), 2.0, 1.8, h + 3.0, 10, true);
        b.kit.mat(ids::SLATE).frustum(Vec3::new(sx * hx, sy * hy, h + 3.0), 2.2, 0.0, 3.5, 10, false);
        b.kit.mat(ids::FACADE);
    }
    let (r, em) = (&mut b.r, b.p.emissive);
    sides(&mut b.kit, hx, hy, |k, _, w| {
        for f in 1..floors {
            for i in 0..3 {
                let x = -w * 0.3 + w * 0.3 * i as f32;
                let id = super::pane_id(r, em, 0.2);
                crate::generators::settlement::window(k, x, 0.0, f as f32 * fh + 1.0, 0.25, 1.4, id);
            }
        }
    });
    let st = b.p.stairs;
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| {
        k.mat(ids::TRIM).arched_panel(1.6, 3.0, false);
        k.at(Mat4::from_translation(Vec3::Z * fh), |k| k.mat(ids::TRIM).arched_panel(1.4, 2.4, true));
    });
    if b.p.parts.porches {
        // External stair to the first-floor door (spiral = stair turret).
        b.kit.at(Mat4::from_translation(Vec3::new(hx * 0.5, -hy - 1.5, 0.0)), |k| {
            stairs(k, st, 1.4, fh);
        });
    }
    battlements(&mut b.kit, hx, hy, h);
    if b.p.parts.signs {
        let g = b.glow(false);
        b.kit.mat(g).frustum(Vec3::new(-2.0, -hy - 0.4, 2.8), 0.15, 0.25, 0.4, 6, true);
    }
}

/// Guild hall (stone ground floor, timber-framed upper floors, cross
/// gables) or tavern (lower, with a hanging sign and porch).
fn hall(b: &mut Ctx, hx: f32, hy: f32, floors: u32, guild: bool) {
    let w = 2.0 * hx;
    let r = &mut b.r;
    let em = b.p.emissive;
    let mut ops: Vec<Opening> = super::arch::window_row(w, 2.6, 1.0, 0.9, 1.4, |_| super::pane_id(r, em, 0.5));
    let mid = ops.len() / 2;
    ops[mid] = Opening { at: ops[mid].at, width: 1.4, sill: 0.0, height: 2.4, fill: ids::TRIM };
    b.kit.at(Mat4::from_translation(Vec3::new(-hx, -hy, 0.0)), |k| {
        k.mat(ids::FACADE);
        wall_with_openings(k, w, 0.4, FH, &ops);
    });
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy + 0.4, 0.0), Vec3::new(hx, hy, FH));
    let mut z = FH;
    for _ in 1..floors.max(1) {
        b.kit.mat(ids::PLASTER).block(Vec3::new(-hx - 0.3, -hy - 0.3, z), Vec3::new(hx + 0.3, hy + 0.3, z + FH));
        let (r, em, sh) = (&mut b.r, b.p.emissive, b.p.parts.shutters);
        sides(&mut b.kit, hx + 0.3, hy + 0.3, |k, _, w| {
            timber_frame(k, w, z, FH);
            let n = ((w / 2.4) as usize).max(1);
            for i in 0..n {
                let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
                crate::generators::settlement::window(k, x, -0.13, z + 1.0, 0.8, 1.1, super::pane_id(r, em, 0.4));
                if sh {
                    parts::shutters(k, x, z + 1.0, 0.8, 1.1);
                }
            }
        });
        z += FH;
    }
    let (ex, ey) = (hx + 0.3, hy + 0.3);
    b.kit.at(Mat4::from_translation(Vec3::Z * z), |k| {
        k.mat(ids::ROOF);
        if guild {
            super::arch::cross_gable(k, ex, ey, ex * 0.4, -ey * 1.6, 50.0, ids::PLASTER);
        } else {
            k.gable(ex, ey, ey * 1.1, 0.4, ids::PLASTER);
        }
    });
    let p = b.p.parts;
    if p.chimneys {
        parts::chimney(&mut b.kit, Vec3::new(ex * 0.6, ey * 0.3, z), ey * 1.1 + 1.0);
    }
    if p.dormers && !guild {
        let pane = b.pane(0.4);
        b.kit.at(Mat4::from_translation(Vec3::new(-ex * 0.4, -ey * 0.45, z + ey * 0.5)), |k| parts::dormer(k, Vec3::ZERO, 1.4, pane));
    }
    if p.signs && !guild {
        let g = b.glow(false);
        b.kit.at(Mat4::from_translation(Vec3::new(hx * 0.6, -hy - 0.03, 0.0)), |k| {
            k.mat(ids::IRON).beam(Vec3::new(0.0, 0.0, FH - 0.4), Vec3::new(0.0, -1.2, FH - 0.4), 0.06, 0.06);
            k.mat(ids::TRIM).block(Vec3::new(-0.05, -1.2, FH - 1.4), Vec3::new(0.05, -0.5, FH - 0.5));
            k.mat(g).frustum(Vec3::new(-0.9, -0.2, FH - 0.9), 0.12, 0.2, 0.35, 6, true);
        });
    }
    if p.porches && !guild {
        let st = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::porch(k, -hx * 0.45, 2.2, 1.6, 0.4, st));
    }
}

fn market_hall(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Open arcade of stone arches with a timber-framed hall on top.
    let n = ((2.0 * hy / 3.5).round() as usize).max(2);
    let span = 2.0 * hy / n as f32;
    b.kit.mat(ids::COBBLE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.15));
    for x in [-hx + 0.3, hx - 0.3] {
        for i in 0..n {
            let y = -hy + span * (i as f32 + 0.5);
            b.kit.at(Mat4::from_translation(Vec3::new(x, y, 1.6)) * Mat4::from_rotation_z(FRAC_PI_2), |k| {
                k.mat(ids::FACADE).arch_wall(span - 0.6, span * 0.4, FH - 1.6, 0.6, false)
            });
        }
        b.kit.mat(ids::FACADE);
        for i in 0..=n {
            let y = -hy + span * i as f32;
            b.kit.block(Vec3::new(x - 0.3, y - 0.3, 0.0), Vec3::new(x + 0.3, y + 0.3, FH));
        }
    }
    let z = FH;
    let h = FH * floors.saturating_sub(1).max(1) as f32;
    b.kit.mat(ids::PLASTER).block(Vec3::new(-hx, -hy, z), Vec3::new(hx, hy, z + h));
    sides(&mut b.kit, hx, hy, |k, _, w| timber_frame(k, w, z, h));
    b.kit.at(Mat4::from_translation(Vec3::Z * (z + h)), |k| k.mat(ids::ROOF).hipped(hy.max(hx), hx.min(hy), hx * 0.9, 0.5));
    // A small bell turret on the ridge.
    b.kit.mat(ids::TRIM).block(Vec3::new(-0.6, -0.6, z + h + hx * 0.6), Vec3::new(0.6, 0.6, z + h + hx * 0.9 + 1.4));
    b.kit.mat(ids::SLATE).frustum(Vec3::Z * (z + h + hx * 0.9 + 1.4), 0.9, 0.0, 1.6, 8, false);
}

fn chapel(b: &mut Ctx, hx: f32, hy: f32) {
    let h = 6.0;
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
    // Apse at the back, buttresses, pointed windows and a bellcote.
    b.kit.frustum(Vec3::new(0.0, hy, 0.0), hx * 0.8, hx * 0.8, h * 0.85, 10, true);
    let (r, em) = (&mut b.r, b.p.emissive);
    sides(&mut b.kit, hx, hy, |k, side, w| {
        if side % 2 == 1 {
            let n = ((w / 3.0) as usize).max(1);
            for i in 0..=n {
                let x = -w * 0.5 + w * i as f32 / n as f32;
                k.mat(ids::FACADE).block(Vec3::new(x - 0.3, -0.7, 0.0), Vec3::new(x + 0.3, 0.0, h * 0.7));
                if i < n {
                    let px = x + w / n as f32 * 0.5;
                    let id = super::pane_id(r, em, 0.5);
                    k.at(Mat4::from_translation(Vec3::new(px, -0.02, 1.8)), |k| k.mat(id).arched_panel(0.7, 2.6, true));
                }
            }
        } else if side == 0 {
            k.at(Mat4::from_translation(Vec3::new(0.0, -0.02, 0.0)), |k| k.mat(ids::TRIM).arched_panel(1.4, 3.0, true));
            let id = super::pane_id(r, em, 0.6);
            k.mat(id).frustum(Vec3::new(0.0, -0.02, h - 1.2), 0.8, 0.8, 0.02, 12, false);
        }
    });
    b.kit.at(Mat4::from_translation(Vec3::Z * h), |k| k.mat(ids::SLATE).gable(hx, hy, hx * 1.3, 0.4, ids::FACADE));
    b.kit.mat(ids::FACADE).block(Vec3::new(-0.8, -hy, h), Vec3::new(0.8, -hy + 0.8, h + hx * 1.3 + 2.5));
    b.kit.mat(ids::SLATE).frustum(Vec3::new(0.0, -hy + 0.4, h + hx * 1.3 + 2.5), 0.9, 0.0, 2.0, 4, false);
}

fn gatehouse(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let h = FH * floors as f32 + 2.0;
    let tw = (hx * 0.35).max(2.5);
    // Two flanking towers, the gate passage arch with a portcullis, and a
    // wall walk with battlements.
    for s in [-1.0, 1.0] {
        let c = Vec3::new(s * (hx - tw), 0.0, 0.0);
        b.kit.mat(ids::FACADE).frustum(c, tw, tw * 0.95, h + 2.0, 12, true);
        b.kit.at(Mat4::from_translation(c), |k| battlements(k, tw * 0.7, tw * 0.7, h + 2.0));
    }
    // Gate passage: piers either side, a round arch springing at 2.5 m,
    // and a portcullis in the front opening.
    let pw = ((hx - tw) * 0.55).max(1.2);
    let spring = 2.5;
    b.kit.mat(ids::FACADE);
    for s in [-1.0, 1.0] {
        let (a, c) = (s * pw, s * (hx - tw));
        b.kit.block(Vec3::new(a.min(c), -hy, 0.0), Vec3::new(a.max(c), hy, h));
    }
    b.kit.at(Mat4::from_translation(Vec3::Z * spring), |k| k.mat(ids::FACADE).arch_wall(2.0 * pw, pw, h - spring, 2.0 * hy, false));
    b.kit.mat(ids::IRON);
    let n = 6;
    for i in 0..=n {
        let x = -pw * 0.9 + 1.8 * pw * i as f32 / n as f32;
        b.kit.block(Vec3::new(x - 0.04, -hy - 0.1, 0.4), Vec3::new(x + 0.04, -hy - 0.02, spring + pw * 0.8));
    }
    battlements(&mut b.kit, hx - tw, hy, h);
    if b.p.parts.porches {
        let st = if b.p.stairs == StairKind::Spiral { StairKind::Spiral } else { StairKind::LShaped };
        b.kit.at(Mat4::from_translation(Vec3::new(hx - tw, hy + 1.5, 0.0)), |k| {
            stairs(k, st, 1.2, FH.min(h));
        });
    }
}
