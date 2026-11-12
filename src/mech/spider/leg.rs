//! One spider leg, built in the leg frame: +X points out along the leg's
//! diagonal, +Z up, Y is the hinge-pin axis. Each part's mesh is relative
//! to its own pivot (the joint that carries it):
//! hip drum (drum centre) -> thigh (hip pin) -> shin (knee bolt) -> foot
//! (ankle), plus the Option A knee support: a carriage on a rail along the
//! shin and a piston from the thigh to the carriage.

use super::kit::{Kit, beam, block, cyl, cyl_mid, loft, mat, spike};
use crate::mech::PistonPart;
use crate::mesh::EditablePoly;
use bevy::math::{Mat4, Vec3};

/// Hip pin, relative to the drum centre (top of the hull plate).
pub const HIP: Vec3 = Vec3::new(0.85, 0.0, 0.25);
/// Knee bolt, relative to the hip pin.
pub const KNEE: Vec3 = Vec3::new(1.45, 0.0, 0.75);
/// Ankle, relative to the knee bolt.
pub const ANKLE: Vec3 = Vec3::new(1.0, 0.0, -2.9);
/// Rest travel point of the carriage along the shin (from the knee).
pub const CARRIAGE_AT: f32 = 1.2;
/// Foot claw length (1.5x the old 0.6 m claws).
pub const CLAW: f32 = 0.9;

/// Down the shin, from knee to ankle.
pub fn shin_dir() -> Vec3 {
    ANKLE.normalize()
}

/// Shin face towards the hull (the side the rail and piston sit on).
pub fn shin_in() -> Vec3 {
    let d = shin_dir();
    Vec3::new(d.z, 0.0, -d.x)
}

/// Carriage pivot, relative to the knee.
pub fn carriage_pivot() -> Vec3 {
    shin_dir() * CARRIAGE_AT + shin_in() * 0.3
}

/// Piston housing eyelet on the thigh, relative to the hip pin.
pub fn housing_eye() -> Vec3 {
    let t = KNEE.normalize();
    t * 0.75 + Vec3::new(t.z, 0.0, -t.x) * 0.3
}

/// Piston rod eyelet on the carriage, relative to the carriage pivot.
pub fn rod_eye() -> Vec3 {
    shin_in() * 0.14
}

pub fn hip_drum() -> EditablePoly {
    let mut k = Kit::default();
    k.add(cyl(0.56, 0.5, 20, Vec3::ZERO, Vec3::Z), mat::JOINT);
    for z in [0.08, 0.22, 0.36] {
        k.add(cyl(0.6, 0.05, 20, Vec3::Z * z, Vec3::Z), mat::JOINT);
    }
    k.add(cyl(0.38, 0.07, 16, Vec3::Z * 0.5, Vec3::Z), mat::JOINT);
    // Bolted clevis: back plate on the drum and two fork plates round the pin.
    k.add(block(Vec3::new(0.14, 0.66, 0.46), Vec3::new(0.56, 0.0, HIP.z)), mat::LEG);
    for s in [-1.0, 1.0] {
        k.add(block(Vec3::new(0.56, 0.08, 0.48), Vec3::new(0.78, s * 0.28, HIP.z)), mat::LEG);
    }
    k.add(cyl_mid(0.09, 0.72, 12, HIP, Vec3::Y), mat::JOINT);
    k.finish()
}

pub fn thigh() -> EditablePoly {
    let mut k = Kit::default();
    let t = KNEE.normalize();
    let up = Vec3::new(-t.z, 0.0, t.x);
    k.add(cyl_mid(0.22, 0.44, 16, Vec3::ZERO, Vec3::Y), mat::JOINT);
    k.add(beam(t * 0.1, KNEE, Vec3::Y, 0.42, 0.5, 0.36, 0.4), mat::LEG);
    // Top armour panel and the piston tab underneath.
    k.add(beam(t * 0.25 + up * 0.25, KNEE - t * 0.2 + up * 0.2, Vec3::Y, 0.46, 0.07, 0.4, 0.07), mat::ARMOUR);
    let eye = housing_eye();
    k.add(beam(eye + up * 0.12, eye, Vec3::Y, 0.06, 0.22, 0.06, 0.22).tap_shift(0.12), mat::LEG);
    k.add(beam(eye + up * 0.12, eye, Vec3::Y, 0.06, 0.22, 0.06, 0.22).tap_shift(-0.12), mat::LEG);
    k.finish()
}

pub fn shin() -> EditablePoly {
    let mut k = Kit::default();
    let (d, inn) = (shin_dir(), shin_in());
    k.add(cyl_mid(0.15, 0.62, 16, Vec3::ZERO, Vec3::Y), mat::JOINT);
    // Long armoured shin; its top sticks out past the knee bolt like a horn.
    k.add(beam(-d * 0.5, ANKLE - d * 0.15, Vec3::Y, 0.46, 0.55, 0.32, 0.38), mat::LEG);
    k.add(beam(-d * 0.45 - inn * 0.3, d * 1.9 - inn * 0.22, Vec3::Y, 0.52, 0.08, 0.4, 0.08), mat::ARMOUR);
    // Rail for the knee carriage on the inner face.
    k.add(beam(d * 0.55 + inn * 0.24, d * 2.1 + inn * 0.2, Vec3::Y, 0.12, 0.08, 0.12, 0.08), mat::BARREL);
    k.finish()
}

pub fn carriage() -> EditablePoly {
    let mut k = Kit::default();
    let d = shin_dir();
    k.add(beam(-d * 0.18, d * 0.18, Vec3::Y, 0.28, 0.14, 0.28, 0.14), mat::BARREL);
    for s in [-1.0, 1.0] {
        k.add(block(Vec3::new(0.08, 0.05, 0.08), Vec3::ZERO).tap_at(rod_eye() * 0.6 + Vec3::Y * s * 0.09), mat::BARREL);
    }
    k.add(cyl_mid(0.05, 0.24, 10, rod_eye(), Vec3::Y), mat::JOINT);
    k.finish()
}

pub fn foot() -> EditablePoly {
    let mut k = Kit::default();
    k.add(cyl_mid(0.17, 0.34, 16, Vec3::ZERO, Vec3::Y), mat::JOINT);
    k.add(cyl(0.12, 0.35, 12, Vec3::Z * -0.35, Vec3::Z), mat::JOINT);
    let oct = |r: f32, z: f32| -> Vec<Vec3> {
        (0..8).map(|i| (i as f32 + 0.5) * std::f32::consts::TAU / 8.0).map(|a| Vec3::new(r * a.cos(), r * a.sin(), z)).collect()
    };
    k.add(loft(&oct(0.46, -0.55), &oct(0.36, -0.3)), mat::FOOT);
    for i in 0..4 {
        let a = (i as f32 + 0.5) * std::f32::consts::FRAC_PI_2;
        let out = Vec3::new(a.cos(), a.sin(), 0.0);
        let side = Vec3::new(-a.sin(), a.cos(), 0.0);
        k.add(spike(out * 0.34 + Vec3::Z * -0.44, out + Vec3::Z * -0.12, side, 0.11, CLAW), mat::FOOT);
    }
    k.finish()
}

/// Piston housing (`rod` false) or rod with Material IDs and UVs; eyelet
/// at the origin (pin along Y), body along +Z.
pub fn piston(rod: bool) -> EditablePoly {
    let p = if rod {
        PistonPart { rod: true, radius: 0.045, length: 0.5, eyelet: 0.07, sides: 12 }
    } else {
        PistonPart { rod: false, radius: 0.09, length: 0.42, eyelet: 0.08, sides: 16 }
    };
    let mut k = Kit::default();
    k.add(p.build(), if rod { mat::ROD } else { mat::BARREL });
    k.finish()
}

/// Small transform helpers for pieces.
trait Tap {
    fn tap_shift(self, y: f32) -> Self;
    fn tap_at(self, at: Vec3) -> Self;
}

impl Tap for EditablePoly {
    fn tap_shift(self, y: f32) -> Self {
        self.tap_at(Vec3::Y * y)
    }
    fn tap_at(mut self, at: Vec3) -> Self {
        self.transform(Mat4::from_translation(at));
        self
    }
}
