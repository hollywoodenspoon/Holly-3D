//! Procedural hair fitted to the morphed head (Material ID 3). Styles are
//! built from a scalp cap (a sphere patch hugging the skull that leaves
//! the face open) plus style pieces: long sheets, a ponytail chain,
//! anime spikes, a top bun or a full beard.

use super::landmarks::Landmarks;
use super::params::{ids, CharacterParams, Hair};
use super::shapes::{chain, dir, ellipsoid, patch, spike};
use crate::generators::kit::{rng, Kit};
use bevy::math::Vec3;
use rand::Rng;
use std::f32::consts::PI;

/// Lowest elevation the cap reaches at azimuth `az` (face open at front).
fn hairline(az: f32) -> f32 {
    let front = (1.0 - (az.abs() / 1.2).min(1.0)).powi(2);
    -0.55 + front * 0.95
}

pub fn build(kit: &mut Kit, p: &CharacterParams, lm: &Landmarks) {
    if p.hair == Hair::Bald {
        return;
    }
    kit.mat(ids::HAIR);
    let (c, r) = (lm.head + Vec3::new(0.0, 0.08, 0.05) * lm.head_r, lm.head_r);
    let thick = 1.10 + if p.hair == Hair::Crop || p.hair == Hair::Beard { 0.0 } else { 0.05 };
    let shell = |d: Vec3| r * (thick + 0.04 * d.z.max(0.0));
    patch(kit, c, (24, 12), shell, |az, el| el > hairline(az));
    let len = 0.15 + 0.55 * p.hair_length.clamp(0.0, 1.0);
    match p.hair {
        Hair::Long => {
            // A curved back sheet falling from the cap to the shoulders and below.
            let top = c + Vec3::new(0.0, 0.25, -0.2) * r;
            patch(
                kit,
                top,
                (20, 10),
                |d| r * (1.05 + 0.15 * (-d.z).max(0.0)) + len * (-d.z).max(0.0) * 1.6,
                |az, el| el < 0.0 && az.abs() > 1.0 && el > -1.2,
            );
        }
        Hair::Ponytail => {
            let root = c + Vec3::new(0.0, r * 1.0, r * 0.35);
            let pts: Vec<Vec3> = (0..7).map(|i| {
                let t = i as f32 / 6.0;
                root + Vec3::new(0.0, r * 0.35 * (t * PI).sin(), -len * t * 1.3)
            }).collect();
            let radii: Vec<f32> = (0..7).map(|i| r * (0.28 - 0.035 * i as f32).max(0.05)).collect();
            chain(kit, &pts, &radii, 8);
        }
        Hair::Spiky => {
            let mut rg = rng(p.seed, 0x5A1C);
            for k in 0..16 {
                let az: f32 = if k < 4 { rg.gen_range(-0.7..0.7) } else { rg.gen_range(-PI..PI) };
                let el = (hairline(az) + 0.2).max(rg.gen_range(0.0..1.2));
                let d = dir(az, el);
                let out = (d + Vec3::new(0.0, 0.3, -0.25)).normalize();
                spike(kit, c + d * r * 1.05, out, r * 0.28, r * (0.6 + len), 5);
            }
        }
        Hair::Bun => ellipsoid(kit, c + Vec3::new(0.0, r * 0.15, r * 1.15), Vec3::splat(r * 0.42), 12),
        Hair::Beard => {
            let jaw = lm.head + Vec3::new(0.0, -0.1, -0.35) * r;
            patch(kit, jaw, (24, 12), |d| r * (0.95 + 0.2 * (-d.z).max(0.0)), |az, el| az.abs() < 1.5 && (-1.1..-0.15).contains(&el));
        }
        Hair::Crop | Hair::Bald => {}
    }
}
