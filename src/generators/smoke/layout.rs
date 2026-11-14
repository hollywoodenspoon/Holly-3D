//! Where a plume's pieces go: lobes up the column, flame tongues at the
//! source, the mushroom-cap ring vortex and the turbulence cards. The
//! preview mesh and the volume renderer both read this one layout.

use super::SmokeParams;
use super::column::Column;
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{PI, TAU};

/// One piece of a plume at normalised plume height `s`.
#[derive(Clone, Copy, Debug)]
pub enum Piece {
    /// A soft lobe: centre, radius, vertical squash and orientation.
    Lobe { c: Vec3, r: f32, squash: f32, s: f32, rot: Quat },
    /// A turbulence card (mesh only).
    Card { c: Vec3, r: f32, s: f32, rot: Quat },
}

/// A plume's layout: its pieces, the share of the height that burns and
/// the noise offset of its shading.
pub struct Layout {
    pub pieces: Vec<Piece>,
    pub fire_zone: f32,
    pub noise_off: Vec3,
}

pub fn layout(p: &SmokeParams) -> Layout {
    let col = Column::new(p);
    let mut r = crate::generators::kit::rng(p.seed, 0x5307E);
    let fire_zone = 0.16 * (p.fire / 100.0).clamp(0.0, 1.0);
    let noise_off = Vec3::new(r.gen_range(0.0..100.0), r.gen_range(0.0..100.0), r.gen_range(0.0..100.0));
    let mut out = Vec::new();
    let n = p.lobes.clamp(3, 64) as usize;
    let mut puffs = Vec::with_capacity(n);
    for i in 0..n {
        // Puffs rise with the phase and slow toward the top (z ~ t^(3/4)).
        let t = ((i as f32 + 0.35 + r.gen_range(-0.25..0.25)) / n as f32 + col.phase).rem_euclid(1.0);
        puffs.push(Column::height_at(t).min(0.97));
    }
    puffs.sort_by(f32::total_cmp);
    for &s in &puffs {
        let jitter = Vec3::new(r.gen_range(-1.0..1.0), r.gen_range(-1.0..1.0), r.gen_range(-0.5..0.5)) * col.radius(s) * 0.25 * col.turbulence;
        let rad = col.radius(s) * r.gen_range(0.75..1.1) * if s < 0.06 { 0.8 } else { 1.0 };
        let rot = Quat::from_rotation_arc(Vec3::Z, col.tangent(s)) * Quat::from_rotation_z(r.gen_range(0.0..TAU));
        out.push(Piece::Lobe { c: col.centre(s) + jitter, r: rad, squash: r.gen_range(0.75..0.95), s, rot });
        if p.cards && r.gen_bool((0.3 + 0.6 * col.turbulence) as f64) {
            let yaw = Quat::from_rotation_z(r.gen_range(0.0..TAU));
            out.push(Piece::Card { c: col.centre(s) + jitter * 1.5, r: rad * 1.7, s, rot: yaw });
            out.push(Piece::Card { c: col.centre(s) + jitter * 1.5, r: rad * 1.5, s, rot: yaw * Quat::from_rotation_z(PI / 2.0) });
        }
    }
    // Flame tongues licking up around the source (Fire Base Intensity).
    let flames = (p.fire.clamp(0.0, 100.0) / 100.0 * 9.0).ceil() as usize;
    let fz = (fire_zone * 0.5).max(1e-3);
    for _ in 0..flames {
        let off = Vec2::from_angle(r.gen_range(0.0..TAU)) * col.r0 * r.gen_range(0.0..0.8);
        let size = col.r0 * r.gen_range(0.35..0.65);
        let c = off.extend(size * r.gen_range(0.6..1.6));
        let squash = r.gen_range(1.6..2.4);
        let s = fz * r.gen_range(0.0..0.9);
        out.push(Piece::Lobe { c, r: size, squash, s, rot: Quat::from_rotation_z(r.gen_range(0.0..TAU)) });
    }
    if p.mushroom {
        // Ring vortex: a cap puff with a torus of lobes curling down around it.
        let (top, r1) = (col.centre(1.0), col.radius(1.0));
        out.push(Piece::Lobe { c: top, r: r1 * 1.05, squash: 0.6, s: 1.0, rot: Quat::IDENTITY });
        let k = 7;
        for ring in 0..2 {
            let (rad, drop, size) = if ring == 0 { (1.05, 0.25, 0.55) } else { (1.3, 0.7, 0.35) };
            for i in 0..k {
                let a = TAU * (i as f32 + 0.5 * ring as f32) / k as f32 + r.gen_range(-0.2..0.2);
                let c = top + Vec3::new(a.cos(), a.sin(), 0.0) * r1 * rad - Vec3::Z * r1 * drop;
                out.push(Piece::Lobe { c, r: r1 * size * r.gen_range(0.85..1.15), squash: 0.8, s: 0.98, rot: Quat::from_rotation_z(a) });
            }
        }
    }
    Layout { pieces: out, fire_zone, noise_off }
}
