//! Outfits & weapons. The base layer reuses MakeHuman's own clothing
//! helpers (`helper-tights`, `helper-skirt`), which follow every morph
//! like any `.mhclo` proxy; armor, belts, bracers, robes and weapons are
//! procedural pieces fitted to the landmarks. Material IDs: cloth 4,
//! metal 5, leather 6.

use super::landmarks::Landmarks;
use super::params::{ids, CharacterParams, Outfit, Weapon};
use super::shapes::{chain, ellipsoid, patch};
use crate::generators::kit::Kit;
use bevy::math::{Quat, Vec3};
use std::f32::consts::TAU;

pub fn tights_material(o: Outfit) -> u16 {
    match o {
        Outfit::Ranger => ids::LEATHER,
        _ => ids::CLOTH,
    }
}

pub fn wants_skirt(o: Outfit) -> bool {
    matches!(o, Outfit::Tunic | Outfit::Robe)
}

/// A closed ring of points around `c` (horizontal), radius `r`.
fn ring(c: Vec3, r: Vec3, n: usize) -> Vec<Vec3> {
    (0..=n).map(|i| {
        let a = TAU * i as f32 / n as f32;
        c + Vec3::new(a.cos() * r.x, a.sin() * r.y, 0.0)
    }).collect()
}

pub fn build(kit: &mut Kit, p: &CharacterParams, lm: &Landmarks) {
    let s = lm.height / 1.7;
    let waist = Vec3::new(0.17, 0.12, 0.0) * s * (0.8 + 0.5 * p.macros.weight);
    match p.outfit {
        Outfit::Knight => {
            kit.mat(ids::METAL);
            // Breastplate and fauld, pauldrons, greaves.
            ellipsoid(kit, lm.chest + Vec3::new(0.0, -0.03, 0.04) * s, Vec3::new(0.19, 0.14, 0.2) * s, 16);
            for k in 0..2 {
                ellipsoid(kit, lm.shoulder[k] + Vec3::Z * 0.02 * s, Vec3::splat(0.085 * s), 10);
                chain(kit, &[lm.knee[k], lm.ankle[k]], &[0.065 * s, 0.05 * s], 10);
                chain(kit, &[lm.elbow[k], lm.hand[k] + (lm.elbow[k] - lm.hand[k]) * 0.15], &[0.05 * s, 0.042 * s], 10);
            }
            kit.mat(ids::LEATHER);
            chain(kit, &ring(lm.pelvis + Vec3::Z * 0.08 * s, waist * 1.05, 16), &[0.022 * s], 6);
        }
        Outfit::Ranger => {
            kit.mat(ids::LEATHER);
            chain(kit, &ring(lm.pelvis + Vec3::Z * 0.08 * s, waist, 16), &[0.02 * s], 6);
            for k in 0..2 {
                chain(kit, &[lm.elbow[k] + (lm.hand[k] - lm.elbow[k]) * 0.35, lm.hand[k] + (lm.elbow[k] - lm.hand[k]) * 0.12], &[0.042 * s, 0.036 * s], 8);
            }
            // Hooded cape over the shoulders and back.
            kit.mat(ids::CLOTH);
            patch(kit, lm.neck + Vec3::new(0.0, 0.02, -0.05) * s, (20, 10), |d| 0.2 * s + (-d.z).max(0.0) * 0.45 * s, |az, el| az.abs() > 1.3 && (-1.2..0.6).contains(&el));
        }
        Outfit::Robe => {
            kit.mat(ids::CLOTH);
            let top = lm.pelvis + Vec3::Z * 0.1 * s;
            let r0 = waist.x * 1.15;
            kit.at(bevy::math::Mat4::from_rotation_translation(Quat::from_rotation_x(std::f32::consts::PI), top), |k| {
                k.frustum(Vec3::ZERO, r0, r0 * 1.9, top.z - 0.02 * s, 20, false);
            });
            kit.mat(ids::LEATHER);
            chain(kit, &ring(top, waist * 1.12, 16), &[0.02 * s], 6);
        }
        Outfit::Tunic => {
            kit.mat(ids::LEATHER);
            chain(kit, &ring(lm.pelvis + Vec3::Z * 0.06 * s, waist * 1.1, 16), &[0.018 * s], 6);
        }
        Outfit::Bodysuit | Outfit::None => {}
    }
    weapon(kit, p.weapon, lm, s);
}

fn weapon(kit: &mut Kit, w: Weapon, lm: &Landmarks, s: f32) {
    let h = lm.hand[1];
    let down = (lm.hand[1] - lm.elbow[1]).normalize_or(-Vec3::Z);
    match w {
        Weapon::None => {}
        Weapon::Sword | Weapon::Shield => {
            kit.mat(ids::LEATHER);
            chain(kit, &[h - down * 0.06 * s, h + down * 0.06 * s], &[0.018 * s], 6);
            kit.mat(ids::METAL);
            chain(kit, &[h + down * 0.06 * s, h + down * 0.07 * s], &[0.09 * s], 4);
            chain(kit, &[h + down * 0.07 * s, h + down * 0.85 * s, h + down * 0.95 * s], &[0.03 * s, 0.025 * s, 0.002], 4);
            if w == Weapon::Shield {
                let l = lm.hand[0] + (lm.elbow[0] - lm.hand[0]) * 0.4;
                let out = (lm.hand[0] - lm.pelvis).with_z(0.0).normalize_or(Vec3::X);
                chain(kit, &[l + out * 0.03 * s, l + out * 0.07 * s], &[0.3 * s, 0.28 * s], 16);
            }
        }
        Weapon::Axe => {
            kit.mat(ids::LEATHER);
            chain(kit, &[h - down * 0.15 * s, h + down * 0.75 * s], &[0.02 * s], 6);
            kit.mat(ids::METAL);
            let head = h + down * 0.65 * s;
            let side = down.cross(Vec3::Y).normalize_or(Vec3::X);
            chain(kit, &[head, head + side * 0.22 * s], &[0.03 * s, 0.13 * s], 4);
        }
        Weapon::Staff => {
            kit.mat(ids::LEATHER);
            chain(kit, &[h - Vec3::Z * 0.85 * s, h + Vec3::Z * 0.75 * s], &[0.02 * s, 0.024 * s], 6);
            kit.mat(ids::EYES);
            ellipsoid(kit, h + Vec3::Z * 0.83 * s, Vec3::splat(0.06 * s), 10);
        }
    }
}
