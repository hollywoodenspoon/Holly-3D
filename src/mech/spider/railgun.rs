//! Railgun turret: a wedge-fronted angular turret on a bearing ring with a
//! camera box, hatch boxes and a rear radiator pack (3 fins per side),
//! and the railgun itself: breech, mantlet, long square barrel with a
//! collar halfway and a forked two-prong muzzle. Front = -Y.

use super::kit::{Kit, block, cyl, cyl_mid, loft, mat, profile};
use crate::mesh::EditablePoly;
use bevy::math::Vec3;

/// Gun trunnion (lift pivot), relative to the turret ring centre.
pub const TRUNNION: Vec3 = Vec3::new(0.0, -1.1, 0.62);
/// Muzzle tip distance from the trunnion.
pub const GUN_REACH: f32 = 6.6;

pub fn turret() -> EditablePoly {
    let mut k = Kit::default();
    k.add(cyl(1.05, 0.15, 28, Vec3::ZERO, Vec3::Z), mat::JOINT);
    let low = [(0.0, -2.3), (1.05, -1.5), (1.35, 0.3), (1.15, 1.7), (-1.15, 1.7), (-1.35, 0.3), (-1.05, -1.5)];
    let high = [(0.0, -1.85), (0.85, -1.25), (1.1, 0.3), (0.95, 1.5), (-0.95, 1.5), (-1.1, 0.3), (-0.85, -1.25)];
    k.add(loft(&profile(&low, 0.15), &profile(&high, 1.0)), mat::ARMOUR);
    // Hatch boxes on top.
    k.add(block(Vec3::new(0.6, 0.6, 0.14), Vec3::new(0.45, 0.35, 1.07)), mat::HULL);
    k.add(block(Vec3::new(0.5, 0.5, 0.12), Vec3::new(-0.5, 0.6, 1.06)), mat::HULL);
    // Camera box with its lens facing forward.
    k.add(block(Vec3::new(0.42, 0.5, 0.34), Vec3::new(0.62, -0.95, 1.17)), mat::HULL);
    k.add(cyl(0.12, 0.1, 16, Vec3::new(0.62, -1.2, 1.17), -Vec3::Y), mat::GLASS);
    // Rear radiator pack: a central box and 3 vertical fins per side.
    k.add(block(Vec3::new(0.9, 0.7, 0.75), Vec3::new(0.0, 1.95, 0.6)), mat::RADIATOR);
    for s in [-1.0, 1.0] {
        for i in 0..3 {
            let x = s * (0.55 + 0.14 * i as f32);
            k.add(block(Vec3::new(0.05, 0.75, 0.85), Vec3::new(x, 1.95, 0.6)), mat::RADIATOR);
        }
    }
    k.finish()
}

/// The railgun, pivot at the trunnion; it points along -Y.
pub fn railgun() -> EditablePoly {
    let mut k = Kit::default();
    let y = |d: f32| Vec3::new(0.0, -d, 0.0);
    k.add(cyl_mid(0.12, 1.0, 14, Vec3::ZERO, Vec3::X), mat::JOINT);
    k.add(block(Vec3::new(0.62, 0.9, 0.56), y(-0.1)), mat::WEAPON);
    k.add(block(Vec3::new(0.72, 0.35, 0.6), y(0.55)), mat::ARMOUR);
    k.add(block(Vec3::new(0.34, 4.8, 0.34), y(0.7 + 2.4)), mat::WEAPON);
    k.add(block(Vec3::new(0.5, 0.4, 0.5), y(3.2)), mat::WEAPON);
    // Forked muzzle: two prongs with a gap between them.
    let len = GUN_REACH - 5.3;
    for s in [-1.0, 1.0] {
        k.add(block(Vec3::new(0.12, len, 0.36), Vec3::new(s * 0.12, -(5.3 + len * 0.5), 0.0)), mat::WEAPON);
    }
    k.finish()
}
