//! The 4-leg base hull (one part, pivot at the centre of the plate's
//! underside): bevelled base plate, central box with front vent slats,
//! sloped armour front plate, the lower tiers of the four hip drums and
//! the turret pedestal. Front = -Y, up = +Z.

use super::kit::{Kit, block, cyl, loft, mat, profile};
use super::{DRUM_AT, PEDESTAL_TOP, PLATE_H};
use crate::mesh::EditablePoly;
use bevy::math::Vec3;

/// Rectangle with cut corners, counter-clockwise seen from +Z.
fn cut_rect(w: f32, l: f32, cut: f32, cy: f32) -> Vec<(f32, f32)> {
    let (x, y) = (w * 0.5, l * 0.5);
    vec![(x - cut, -y), (x, -y + cut), (x, y - cut), (x - cut, y), (-x + cut, y), (-x, y - cut), (-x, -y + cut), (-x + cut, -y)]
        .into_iter()
        .map(|(a, b)| (a, b + cy))
        .collect()
}

pub fn hull() -> EditablePoly {
    let mut k = Kit::default();
    // Base plate, bevelled top edge.
    k.add(loft(&profile(&cut_rect(4.0, 5.0, 0.6, 0.0), 0.0), &profile(&cut_rect(3.8, 4.8, 0.55, 0.0), PLATE_H)), mat::HULL);
    // Central box, narrowing towards its top.
    let box_top = PEDESTAL_TOP - 0.2;
    k.add(loft(&profile(&cut_rect(2.6, 3.6, 0.25, 0.1), PLATE_H), &profile(&cut_rect(2.3, 3.2, 0.2, 0.15), box_top)), mat::HULL);
    // Sloped armour front plate.
    let front = [(1.5, -2.55), (1.5, -2.2), (-1.5, -2.2), (-1.5, -2.55)];
    let top = [(1.3, -1.95), (1.3, -1.7), (-1.3, -1.7), (-1.3, -1.95)];
    let ring = |pts: &[(f32, f32)], z: f32| -> Vec<Vec3> { pts.iter().map(|&(x, y)| Vec3::new(x, y, z)).collect() };
    k.add(loft(&ring(&front, 0.15), &ring(&top, 0.85)), mat::ARMOUR);
    // Front vent slats on the central box, above the armour plate.
    for i in 0..4 {
        let z = 0.95 + i as f32 * 0.1;
        let y = -1.7 + i as f32 * 0.035;
        k.add(block(Vec3::new(1.7, 0.08, 0.05), Vec3::new(0.0, y, z)), mat::RADIATOR);
    }
    // Lower drum tiers with two ribs each.
    for c in DRUM_AT {
        let at = Vec3::new(c.x, c.y, 0.0);
        k.add(cyl(0.62, PLATE_H, 20, at, Vec3::Z), mat::JOINT);
        for z in [0.1, 0.28] {
            k.add(cyl(0.66, 0.06, 20, at + Vec3::Z * z, Vec3::Z), mat::JOINT);
        }
    }
    // Turret pedestal and its bearing ring.
    k.add(cyl(1.1, 0.16, 28, Vec3::Z * box_top, Vec3::Z), mat::HULL);
    k.add(cyl(1.15, 0.04, 28, Vec3::Z * (PEDESTAL_TOP - 0.04), Vec3::Z), mat::JOINT);
    // Side hatches on the central box.
    for s in [-1.0, 1.0] {
        k.add(block(Vec3::new(0.06, 1.0, 0.4), Vec3::new(s * 1.22, 0.4, 0.95)), mat::HULL);
    }
    k.finish()
}
