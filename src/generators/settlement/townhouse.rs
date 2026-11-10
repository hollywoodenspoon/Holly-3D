//! Medieval burgher townhouses (stone ground floor, jettied timber-frame
//! upper storeys with plaster infill, steep gables to the street), village
//! cottages with thatched roofs, and timber barns.

use super::layout::sides;
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::FRAC_PI_2;

/// Timber framing on one face (in a `sides` frame): posts every ~1.3 m,
/// a rail at each floor line and a brace in alternate bays.
pub fn timber_frame(kit: &mut Kit, w: f32, z0: f32, h: f32) {
    kit.mat(ids::TRIM);
    let n = ((w / 1.3).round() as usize).max(2);
    for i in 0..=n {
        let x = -w * 0.5 + w * i as f32 / n as f32;
        kit.block(Vec3::new(x - 0.09, -0.12, z0), Vec3::new(x + 0.09, 0.0, z0 + h));
    }
    for z in [z0, z0 + h - 0.18] {
        kit.block(Vec3::new(-w * 0.5, -0.12, z), Vec3::new(w * 0.5, 0.0, z + 0.18));
    }
    for i in (0..n).step_by(2) {
        let (x0, x1) = (-w * 0.5 + w * i as f32 / n as f32, -w * 0.5 + w * (i + 1) as f32 / n as f32);
        kit.beam(Vec3::new(x0, -0.06, z0 + 0.2), Vec3::new(x1, -0.06, z0 + h - 0.2), 0.12, 0.14);
    }
}

/// A burgher townhouse in its lot frame; returns its height to the eaves.
pub fn townhouse(kit: &mut Kit, half: Vec2, floors: u32, pitch: f32, r: &mut ChaCha8Rng) -> f32 {
    let fh = super::Era::WalledCity.floor_height();
    let mut h = Vec2::new(half.x - 0.2, half.y - 0.45 * floors.saturating_sub(1) as f32).max(Vec2::splat(2.0));
    // Stone ground floor with a door and shop window.
    kit.mat(ids::FACADE).block((-h).extend(0.0), h.extend(fh));
    kit.at(Mat4::from_translation(Vec3::new(0.0, -h.y - 0.03, 0.0)), |k| {
        k.mat(ids::TRIM).arched_panel(1.2, 2.3, false);
        k.at(Mat4::from_translation(Vec3::X * h.x * 0.5), |k| k.mat(if r.gen_bool(0.3) { ids::LIT } else { ids::GLASS }).arched_panel(1.0, 1.6, false));
    });
    let mut z = fh;
    for _ in 1..floors.max(1) {
        // Jetty: each upper storey overhangs the one below front and back.
        h.y += 0.45;
        kit.mat(ids::TRIM).block(Vec3::new(-h.x, -h.y, z - 0.25), Vec3::new(h.x, h.y, z));
        kit.mat(ids::PLASTER).block(Vec3::new(-h.x, -h.y, z), Vec3::new(h.x, h.y, z + fh));
        let hh = h;
        sides(kit, hh.x, hh.y, |k, side, w| {
            if side % 2 == 1 {
                return; // party walls touch the neighbours
            }
            timber_frame(k, w, z, fh);
            let n = ((w / 2.2).floor() as usize).max(1);
            for i in 0..n {
                let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
                super::window(k, x, -0.13, z + 1.0, 0.8, 1.1, if r.gen_bool(0.25) { ids::LIT } else { ids::GLASS });
            }
        });
        z += fh;
    }
    // Steep gable facing the street (ridge along local Y), then a chimney.
    let rise = pitch.clamp(20.0, 70.0).to_radians().tan() * h.x;
    kit.at(Mat4::from_translation(Vec3::Z * z) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::ROOF).gable(h.y, h.x, rise, 0.35, ids::PLASTER));
    if r.gen_bool(0.6) {
        let cx = h.x * 0.55 * if r.gen_bool(0.5) { 1.0 } else { -1.0 };
        kit.mat(ids::FACADE).block(Vec3::new(cx - 0.4, h.y * 0.3 - 0.4, z), Vec3::new(cx + 0.4, h.y * 0.3 + 0.4, z + rise + 1.2));
    }
    z
}

/// Village cottage: low stone / wattle walls and a thick thatched hip roof.
pub fn cottage(kit: &mut Kit, half: Vec2, pitch: f32, r: &mut ChaCha8Rng) {
    let h = 2.6;
    let wall = if r.gen_bool(0.5) { ids::FACADE } else { ids::PLASTER };
    kit.mat(wall).block((-half).extend(0.0), half.extend(h));
    sides(kit, half.x, half.y, |k, side, w| {
        if side == 0 {
            super::window(k, 0.0, 0.0, 0.0, 1.0, 1.9, ids::TRIM);
        }
        if side % 2 == 0 {
            super::window(k, w * 0.28, 0.0, 1.0, 0.7, 0.7, if r.gen_bool(0.35) { ids::LIT } else { ids::GLASS });
        }
        timber_frame(k, w, 0.0, h);
    });
    let rise = pitch.clamp(25.0, 65.0).to_radians().tan() * half.y;
    kit.at(Mat4::from_translation(Vec3::Z * (h - 0.3)), |k| k.mat(ids::ROOF).hipped(half.x, half.y, rise, 0.6));
}

/// Timber barn with a big door and a gabled roof.
pub fn barn(kit: &mut Kit, half: Vec2, pitch: f32) {
    let h = 4.5;
    kit.mat(ids::TRIM).block((-half).extend(0.0), half.extend(h));
    super::window(kit, 0.0, -half.y - 0.03, 0.0, half.x * 0.7, 3.6, ids::IRON);
    let rise = pitch.clamp(20.0, 60.0).to_radians().tan() * half.y;
    kit.at(Mat4::from_translation(Vec3::Z * h), |k| k.mat(ids::ROOF).gable(half.x, half.y, rise, 0.4, ids::TRIM));
}
