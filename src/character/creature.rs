//! Procedural creature presets (Create -> Characters -> Creatures), built
//! from swept tube chains, ellipsoids and spikes, facing -Y with feet on
//! Z = 0. Material IDs follow the Character Pack (skin 1, eyes 2, hair /
//! fur 3, metal / horn 5).

use super::params::{ids, CharacterParams, Creature};
use super::shapes::{chain, ellipsoid, patch, spike};
use crate::generators::kit::{rng, Kit};
use crate::mesh::EditablePoly;
use bevy::math::{Quat, Vec3};
use rand::Rng;
use std::f32::consts::PI;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn eyes(k: &mut Kit, at: Vec3, spread: f32, r: f32) {
    k.mat(ids::EYES);
    for sx in [-1.0, 1.0] {
        ellipsoid(k, at + Vec3::X * spread * sx, Vec3::splat(r), 8);
    }
    k.mat(ids::SKIN);
}

/// Four legs from hip/shoulder sockets to the ground (knee bent back).
fn legs(k: &mut Kit, sockets: &[Vec3], r: f32) {
    for &s in sockets {
        let knee = v(s.x * 1.1, s.y + 0.08, s.z * 0.5);
        chain(k, &[s, knee, v(s.x * 1.1, s.y - 0.04, 0.03), v(s.x * 1.1, s.y - 0.12, 0.0)], &[r, r * 0.75, r * 0.6, r * 0.5], 8);
    }
}

pub fn build(p: &CharacterParams) -> EditablePoly {
    let mut k = Kit::new();
    k.mat(ids::SKIN);
    let mut rg = rng(p.seed, 0xC4EA);
    match p.creature {
        Creature::Humanoid => {}
        Creature::Wolf => {
            chain(&mut k, &[v(0.0, 0.55, 0.72), v(0.0, 0.1, 0.78), v(0.0, -0.35, 0.82)], &[0.17, 0.2, 0.22], 12);
            chain(&mut k, &[v(0.0, -0.35, 0.82), v(0.0, -0.55, 0.98), v(0.0, -0.72, 0.98), v(0.0, -0.92, 0.92)], &[0.16, 0.13, 0.1, 0.04], 10);
            legs(&mut k, &[v(-0.12, 0.45, 0.62), v(0.12, 0.45, 0.62), v(-0.13, -0.28, 0.66), v(0.13, -0.28, 0.66)], 0.07);
            k.mat(ids::HAIR);
            chain(&mut k, &[v(0.0, 0.55, 0.75), v(0.0, 0.8, 0.7), v(0.0, 1.0, 0.5)], &[0.07, 0.09, 0.02], 8);
            for sx in [-1.0, 1.0] {
                spike(&mut k, v(0.06 * sx, -0.62, 1.05), v(0.3 * sx, 0.2, 1.0), 0.04, 0.12, 4);
            }
            eyes(&mut k, v(0.0, -0.8, 1.0), 0.05, 0.02);
        }
        Creature::Dragon => {
            chain(&mut k, &[v(0.0, 0.7, 1.0), v(0.0, 0.1, 1.15), v(0.0, -0.5, 1.2)], &[0.3, 0.4, 0.35], 14);
            let neck: Vec<Vec3> = (0..6).map(|i| v(0.0, -0.5 - 0.15 * i as f32, 1.2 + 0.18 * i as f32)).collect();
            chain(&mut k, &neck, &[0.25, 0.2, 0.17, 0.15, 0.14, 0.13], 12);
            chain(&mut k, &[v(0.0, -1.25, 2.1), v(0.0, -1.55, 2.12), v(0.0, -1.75, 2.05)], &[0.16, 0.11, 0.05], 10);
            let tail: Vec<Vec3> = (0..9).map(|i| { let t = i as f32 / 8.0; v((t * PI * 1.5).sin() * 0.4, 0.7 + 1.8 * t, 1.0 - 0.8 * t) }).collect();
            let tr: Vec<f32> = (0..9).map(|i| 0.28 * (1.0 - i as f32 / 9.0) + 0.02).collect();
            chain(&mut k, &tail, &tr, 10);
            legs(&mut k, &[v(-0.3, 0.55, 0.9), v(0.3, 0.55, 0.9), v(-0.32, -0.35, 1.0), v(0.32, -0.35, 1.0)], 0.12);
            k.mat(ids::HAIR);
            for sx in [-1.0f32, 1.0] {
                let root = v(0.25 * sx, -0.2, 1.45);
                let (tip, mid) = (v(2.2 * sx, 0.1, 2.3), v(1.3 * sx, -0.4, 2.6));
                k.poly(&if sx > 0.0 { [root, mid, tip, v(1.6 * sx, 0.6, 1.4), v(0.3 * sx, 0.6, 1.35)] } else { [root, v(0.3 * sx, 0.6, 1.35), v(1.6 * sx, 0.6, 1.4), tip, mid] });
                chain(&mut k, &[root, mid, tip], &[0.05, 0.035, 0.01], 6);
            }
            k.mat(ids::METAL);
            for sx in [-1.0, 1.0] {
                spike(&mut k, v(0.08 * sx, -1.3, 2.22), v(0.3 * sx, 0.8, 0.5), 0.04, 0.3, 5);
            }
            eyes(&mut k, v(0.0, -1.5, 2.18), 0.09, 0.03);
        }
        Creature::Spider => {
            ellipsoid(&mut k, v(0.0, 0.45, 0.55), v(0.4, 0.5, 0.35), 16);
            ellipsoid(&mut k, v(0.0, -0.2, 0.45), v(0.25, 0.28, 0.2), 14);
            for i in 0..8 {
                let sx = if i % 2 == 0 { -1.0 } else { 1.0 };
                let row = (i / 2) as f32;
                let root = v(0.18 * sx, -0.35 + 0.12 * row, 0.45);
                let out = v(sx, (row - 1.5) * 0.45, 0.0).normalize();
                chain(&mut k, &[root, root + out * 0.45 + Vec3::Z * 0.4, root + out * 1.0 + Vec3::Z * 0.15, (root + out * 1.25).with_z(0.0)], &[0.05, 0.04, 0.03, 0.01], 6);
            }
            eyes(&mut k, v(0.0, -0.45, 0.55), 0.06, 0.035);
        }
        Creature::Slime => {
            let phase: f32 = rg.gen_range(0.0..PI);
            patch(&mut k, v(0.0, 0.0, 0.32), (24, 12), |d| 0.45 * (1.0 + 0.08 * (d.x * 7.0 + phase).sin() * (d.y * 5.0).cos()) * if d.z < 0.0 { 0.75 } else { 1.0 }, |_, _| true);
            eyes(&mut k, v(0.0, -0.36, 0.5), 0.12, 0.07);
        }
        Creature::Golem => {
            k.sg = 0;
            let rot = |a: f32| Quat::from_rotation_z(a);
            k.cuboid(v(0.0, 0.0, 1.55), v(0.55, 0.35, 0.5), rot(0.0));
            k.cuboid(v(0.0, -0.05, 2.25), v(0.22, 0.2, 0.2), rot(0.1));
            for sx in [-1.0f32, 1.0] {
                k.cuboid(v(0.75 * sx, 0.0, 1.4), v(0.2, 0.22, 0.55), rot(0.15 * sx));
                k.cuboid(v(0.8 * sx, -0.05, 0.75), v(0.26, 0.26, 0.22), rot(-0.2 * sx));
                k.cuboid(v(0.3 * sx, 0.0, 0.5), v(0.22, 0.24, 0.5), rot(0.05 * sx));
            }
            k.mat(ids::EYES);
            for sx in [-1.0, 1.0] {
                k.cuboid(v(0.08 * sx, -0.25, 2.3), v(0.04, 0.02, 0.025), rot(0.0));
            }
        }
        Creature::Serpent => {
            let pts: Vec<Vec3> = (0..24).map(|i| { let t = i as f32 / 23.0; v((t * PI * 3.0).sin() * 0.6, 3.0 * t, 0.25 + (1.0 - t).powi(3) * 1.2) }).collect();
            let r: Vec<f32> = (0..24).map(|i| 0.05 + 0.18 * (1.0 - i as f32 / 24.0).sqrt()).collect();
            chain(&mut k, &pts, &r, 10);
            chain(&mut k, &[pts[0], pts[0] + v(0.0, -0.35, 0.05)], &[0.2, 0.08], 10);
            k.mat(ids::METAL);
            for i in (2..20).step_by(3) {
                spike(&mut k, pts[i] + Vec3::Z * r[i] * 0.8, Vec3::Z, 0.05, 0.18 * r[i] / 0.2, 4);
            }
            eyes(&mut k, pts[0] + v(0.0, -0.15, 0.15), 0.1, 0.035);
        }
    }
    let mut m = k.finish();
    let s = p.creature_size.max(0.05);
    let lift = m.bounds().map_or(0.0, |b| -b.min.z);
    m.transform(bevy::math::Mat4::from_scale(Vec3::splat(s)) * bevy::math::Mat4::from_translation(Vec3::Z * lift));
    m
}
