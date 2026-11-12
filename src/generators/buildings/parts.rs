//! Building part toggles and the part builders shared by every building
//! type (and the GhostTown generator): balconies, fire escapes, awnings,
//! chimneys, dormers, shutters, rooftop clutter (AC units, tanks,
//! antennas), signs, and stairs & porches. Facade parts are built in a
//! `kit::sides` face frame: face at y = 0 facing -Y, centred on x = 0.

use super::arch::{StairKind, stairs};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Parts {
    pub balconies: bool,
    pub fire_escapes: bool,
    pub awnings: bool,
    pub chimneys: bool,
    pub dormers: bool,
    pub shutters: bool,
    /// Rooftop clutter: AC units, water tanks, antennas.
    pub rooftop: bool,
    pub signs: bool,
    /// Stairs & porches.
    pub porches: bool,
}

impl Default for Parts {
    fn default() -> Self {
        Self { balconies: true, fire_escapes: true, awnings: true, chimneys: true, dormers: true, shutters: true, rooftop: true, signs: true, porches: true }
    }
}

impl Parts {
    pub const NONE: Parts =
        Parts { balconies: false, fire_escapes: false, awnings: false, chimneys: false, dormers: false, shutters: false, rooftop: false, signs: false, porches: false };

    /// (label, flag) pairs for the checkbox grid.
    pub fn flags(&mut self) -> [(&'static str, &mut bool); 9] {
        [
            ("Balconies", &mut self.balconies),
            ("Fire Escapes", &mut self.fire_escapes),
            ("Awnings", &mut self.awnings),
            ("Chimneys", &mut self.chimneys),
            ("Dormers", &mut self.dormers),
            ("Shutters", &mut self.shutters),
            ("Rooftop Clutter", &mut self.rooftop),
            ("Signs", &mut self.signs),
            ("Stairs & Porches", &mut self.porches),
        ]
    }
}

/// A balcony slab with a railing in front of a facade at height `z`.
pub fn balcony(kit: &mut Kit, x: f32, z: f32, w: f32, depth: f32) {
    kit.mat(ids::FACADE).block(Vec3::new(x - w * 0.5, -depth, z - 0.15), Vec3::new(x + w * 0.5, 0.0, z));
    kit.mat(ids::IRON);
    kit.block(Vec3::new(x - w * 0.5, -depth, z + 0.95), Vec3::new(x + w * 0.5, -depth + 0.05, z + 1.0));
    let n = ((w / 0.6).round() as usize).max(2);
    for i in 0..=n {
        let px = x - w * 0.5 + w * i as f32 / n as f32;
        kit.block(Vec3::new(px - 0.02, -depth, z), Vec3::new(px + 0.02, -depth + 0.04, z + 0.95));
    }
}

/// An iron fire escape: a landing per floor with zig-zag ladders.
pub fn fire_escape(kit: &mut Kit, x: f32, w: f32, floors: u32, fh: f32) {
    kit.mat(ids::IRON);
    for f in 1..floors {
        let z = f as f32 * fh;
        kit.block(Vec3::new(x - w * 0.5, -1.2, z - 0.08), Vec3::new(x + w * 0.5, 0.0, z));
        kit.block(Vec3::new(x - w * 0.5, -1.2, z + 0.9), Vec3::new(x + w * 0.5, -1.15, z + 0.95));
        let (a, b) = if f % 2 == 0 { (x - w * 0.4, x + w * 0.4) } else { (x + w * 0.4, x - w * 0.4) };
        if f + 1 < floors {
            kit.beam(Vec3::new(a, -0.6, z), Vec3::new(b, -0.6, z + fh), 0.5, 0.06);
        }
    }
}

/// A sloped fabric awning `w` wide over a shopfront at height `z`.
pub fn awning(kit: &mut Kit, x: f32, z: f32, w: f32, depth: f32, stripe: bool) {
    let id = if stripe { ids::STUCCO } else { ids::ROOF };
    kit.mat(id).quad(
        Vec3::new(x - w * 0.5, -depth, z - 0.6),
        Vec3::new(x + w * 0.5, -depth, z - 0.6),
        Vec3::new(x + w * 0.5, 0.0, z),
        Vec3::new(x - w * 0.5, 0.0, z),
    );
    kit.quad(Vec3::new(x + w * 0.5, -depth, z - 0.6), Vec3::new(x - w * 0.5, -depth, z - 0.6), Vec3::new(x - w * 0.5, -depth, z - 0.9), Vec3::new(x + w * 0.5, -depth, z - 0.9));
}

/// A brick chimney stack standing at `at` (its base) `h` tall.
pub fn chimney(kit: &mut Kit, at: Vec3, h: f32) {
    kit.mat(ids::BRICK).block(at - Vec3::new(0.4, 0.4, 0.0), at + Vec3::new(0.4, 0.4, h));
    kit.mat(ids::TRIM).block(at + Vec3::new(-0.48, -0.48, h), at + Vec3::new(0.48, 0.48, h + 0.15));
}

/// A gabled dormer window poking out of a roof slope at `at` (local frame
/// of the roof, front facing -Y).
pub fn dormer(kit: &mut Kit, at: Vec3, w: f32, pane: u16) {
    kit.at(Mat4::from_translation(at), |k| {
        k.mat(ids::PLASTER).block(Vec3::new(-w * 0.5, -0.9, 0.0), Vec3::new(w * 0.5, 0.6, 1.2));
        k.mat(pane).quad(Vec3::new(-w * 0.3, -0.92, 0.2), Vec3::new(w * 0.3, -0.92, 0.2), Vec3::new(w * 0.3, -0.92, 1.0), Vec3::new(-w * 0.3, -0.92, 1.0));
        k.at(Mat4::from_translation(Vec3::new(0.0, -0.15, 1.2)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2), |k| {
            k.mat(ids::ROOF).gable(0.8, w * 0.5, 0.6, 0.12, ids::PLASTER)
        });
    });
}

/// A pair of shutters either side of a window centred at `x`.
pub fn shutters(kit: &mut Kit, x: f32, z: f32, w: f32, h: f32) {
    kit.mat(ids::TRIM);
    for s in [-1.0, 1.0] {
        let cx = x + s * (w * 0.5 + w * 0.25);
        kit.block(Vec3::new(cx - w * 0.24, -0.06, z), Vec3::new(cx + w * 0.24, -0.02, z + h));
    }
}

/// Rooftop clutter over a `2hx` x `2hy` flat roof at height `z`.
pub fn rooftop(kit: &mut Kit, hx: f32, hy: f32, z: f32, r: &mut ChaCha8Rng) {
    let n = ((hx * hy / 40.0).sqrt().round() as usize).clamp(1, 8);
    kit.mat(ids::IRON);
    for _ in 0..n {
        let c = Vec2::new(r.gen_range(-hx * 0.7..hx * 0.7), r.gen_range(-hy * 0.7..hy * 0.7));
        kit.block((c - Vec2::new(0.8, 0.6)).extend(z), (c + Vec2::new(0.8, 0.6)).extend(z + 1.1));
    }
    if hx.min(hy) > 4.0 && r.gen_bool(0.6) {
        let c = Vec3::new(hx * 0.5, -hy * 0.4, z);
        kit.mat(ids::TRIM);
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            kit.beam(c + Vec3::new(dx, dy, 0.0), c + Vec3::new(dx * 0.9, dy * 0.9, 2.5), 0.12, 0.12);
        }
        kit.frustum(c + Vec3::Z * 2.5, 1.5, 1.5, 2.6, 10, true);
        kit.frustum(c + Vec3::Z * 5.1, 1.55, 0.0, 0.9, 10, false);
    }
    kit.mat(ids::IRON).tube(Vec3::new(-hx * 0.6, hy * 0.5, z), Vec3::new(-hx * 0.6, hy * 0.5, z + r.gen_range(3.0..7.0)), 0.05, 5);
}

/// A shop / neon sign board centred at `x`, `z` on a facade.
pub fn sign(kit: &mut Kit, x: f32, z: f32, w: f32, glow: u16) {
    kit.mat(ids::TRIM).block(Vec3::new(x - w * 0.5, -0.2, z), Vec3::new(x + w * 0.5, 0.0, z + 0.7));
    kit.mat(glow).quad(Vec3::new(x - w * 0.42, -0.21, z + 0.12), Vec3::new(x + w * 0.42, -0.21, z + 0.12), Vec3::new(x + w * 0.42, -0.21, z + 0.58), Vec3::new(x - w * 0.42, -0.21, z + 0.58));
}

/// An entrance porch: steps up to `rise`, a deck, two posts and a roof.
pub fn porch(kit: &mut Kit, x: f32, w: f32, depth: f32, rise: f32, kind: StairKind) {
    kit.mat(ids::TRIM).block(Vec3::new(x - w * 0.5, -depth, 0.0), Vec3::new(x + w * 0.5, 0.0, rise));
    if rise > 0.1 {
        let k = if kind == StairKind::Spiral { StairKind::Straight } else { kind };
        kit.at(Mat4::from_translation(Vec3::new(x, -depth, 0.0)) * Mat4::from_rotation_z(std::f32::consts::PI), |kk| {
            stairs(kk, k, w * 0.6, rise);
        });
    }
    kit.mat(ids::TRIM);
    for s in [-1.0, 1.0] {
        let px = x + s * (w * 0.5 - 0.1);
        kit.block(Vec3::new(px - 0.08, -depth, rise), Vec3::new(px + 0.08, -depth + 0.16, rise + 2.5));
    }
    kit.mat(ids::ROOF).block(Vec3::new(x - w * 0.5 - 0.1, -depth - 0.2, rise + 2.5), Vec3::new(x + w * 0.5 + 0.1, 0.0, rise + 2.65));
}
