//! Half-Timbered Medieval House / Tavern on a Box: stone ground floor,
//! jettied timber-frame upper storeys with plaster infill (floor count from
//! the box height), a steep gabled roof with dormer windows (count from the
//! roof length) and an extruded stone chimney.

use super::{ArchLayout, ArchitectMod};
use crate::generators::kit::{Kit, ids, sides};
use crate::generators::settlement::window;
use crate::generators::settlement::timber_frame;
use bevy::math::{Mat4, Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

pub fn build(kit: &mut Kit, a: &ArchitectMod, s: Vec3, lay: &ArchLayout) {
    let floors = lay.floors.max(1);
    let roof_share = if a.roof { 0.35 } else { 0.0 };
    let fh = s.z * (1.0 - roof_share) / floors as f32;
    let jetty = (fh * 0.12).clamp(0.15, 0.6);
    // Ridge runs along the long side; the base shrinks so jetties stay inside.
    let mut h = Vec2::new(s.x, s.y) * 0.5 - Vec2::new(0.0, jetty * (floors - 1) as f32);
    h = h.max(Vec2::splat(1.0));
    let along_x = s.x >= s.y;
    let f = if along_x { Mat4::IDENTITY } else { Mat4::from_rotation_z(FRAC_PI_2) };
    if !along_x {
        h = Vec2::new(s.y * 0.5, s.x * 0.5 - jetty * (floors - 1) as f32).max(Vec2::splat(1.0));
    }
    let spacing = a.window_spacing.max(0.8);
    kit.at(f, |k| {
        k.mat(ids::FACADE).block((-h).extend(0.0), h.extend(fh));
        sides(k, h.x, h.y, |k, side, w| {
            let n = ((w / spacing).floor() as usize).max(1);
            for i in 0..n {
                let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
                if side == 0 && a.door && i == n / 2 {
                    k.at(Mat4::from_translation(Vec3::new(x, 0.0, 0.0)), |k| k.mat(ids::TRIM).arched_panel(1.2, (fh * 0.8).min(2.4), false));
                } else if a.windows {
                    window(k, x, 0.0, fh * 0.35, 0.9, fh * 0.4, if (i + side) % 3 == 0 { ids::LIT } else { ids::GLASS });
                }
            }
        });
        let mut z = fh;
        for _ in 1..floors {
            h.y += jetty;
            k.mat(ids::TRIM).block(Vec3::new(-h.x, -h.y, z - 0.22), Vec3::new(h.x, h.y, z));
            k.mat(ids::PLASTER).block(Vec3::new(-h.x, -h.y, z), Vec3::new(h.x, h.y, z + fh));
            let hh = h;
            sides(k, hh.x, hh.y, |k, side, w| {
                timber_frame(k, w, z, fh);
                if a.windows {
                    let n = ((w / spacing).floor() as usize).max(1);
                    for i in 0..n {
                        let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
                        window(k, x, -0.13, z + fh * 0.3, 0.8, fh * 0.4, if (i + side) % 4 == 1 { ids::LIT } else { ids::GLASS });
                    }
                }
            });
            z += fh;
        }
        if a.roof {
            let rise = s.z * roof_share;
            k.at(Mat4::from_translation(Vec3::Z * z), |k| k.mat(ids::ROOF).gable(h.x, h.y, rise, 0.4, ids::PLASTER));
            dormers(k, h, z, rise, spacing);
            // Stone chimney through the roof near one gable end.
            let cx = h.x * 0.7;
            k.mat(ids::FACADE).block(Vec3::new(cx - 0.45, -0.45, 0.0), Vec3::new(cx + 0.45, 0.45, z + rise + 1.2));
            k.block(Vec3::new(cx - 0.55, -0.55, z + rise + 1.0), Vec3::new(cx + 0.55, 0.55, z + rise + 1.3));
        }
    });
}

/// Dormer windows on the front roof slope, one per `spacing * 1.5`.
fn dormers(k: &mut Kit, h: Vec2, z: f32, rise: f32, spacing: f32) {
    let n = ((2.0 * h.x * 0.8 / (spacing * 1.5)).floor() as usize).max(1);
    let (dw, dh) = (0.7, 0.9);
    let y = -h.y * 0.45;
    let zb = z + rise * (1.0 - 0.45);
    for i in 0..n {
        let x = -h.x * 0.8 + 2.0 * h.x * 0.8 * (i as f32 + 0.5) / n as f32;
        k.mat(ids::PLASTER).block(Vec3::new(x - dw, y - 0.6, zb - 0.3), Vec3::new(x + dw, y + 0.6, zb + dh));
        k.at(Mat4::from_translation(Vec3::new(x, y, zb + dh)) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::ROOF).gable(0.6, dw, 0.5, 0.15, ids::PLASTER));
        window(k, x, y - 0.63, zb, dw * 1.2, dh * 0.8, ids::GLASS);
    }
}
