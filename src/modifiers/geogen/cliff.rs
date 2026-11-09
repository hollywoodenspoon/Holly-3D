//! Layered Cliff / Slate: stratified horizontal terraces (each stratum is a
//! vertical-walled ring stepped inward from the one below, joined by flat
//! ledges) cut by vertical fracture cracks shared by every layer.

use super::{Field, GeoGenParams, range, rng};
use crate::mesh::EditablePoly;
use crate::splines::sweep::{cap, loft};
use bevy::math::Vec3;
use std::f32::consts::TAU;

pub const CLIFF_SIDES: usize = 32;

pub fn cliff(p: &GeoGenParams) -> EditablePoly {
    let mut r = rng(p.seed, 30);
    let field = Field::new(p.seed);
    let layers = p.count.clamp(1, 50) as usize;
    let height = range(&mut r, p.min_height, p.max_height.max(p.min_height));
    let lh = height / layers as f32;
    let off = Vec3::splat(range(&mut r, 0.0, 40.0));
    let cracks: Vec<(f32, f32)> = (0..p.facets.min(30)).map(|_| (range(&mut r, 0.0, TAU), range(&mut r, 0.12, 0.3))).collect();
    // Footprint: a lumpy, slightly elongated blob.
    let elong = range(&mut r, 1.0, 1.5);
    let footprint: Vec<Vec3> = (0..CLIFF_SIDES)
        .map(|i| {
            let a = i as f32 / CLIFF_SIDES as f32 * TAU;
            let dir = Vec3::new(a.cos() * elong, a.sin(), 0.0);
            let lump = 1.0 + 0.25 * field.fbm(Vec3::new(a.cos(), a.sin(), 0.0) * p.noise_scale + off);
            let crack: f32 = cracks.iter().map(|(c, depth)| depth * (-(angle_diff(a, *c) / 0.06).powi(2)).exp()).sum();
            dir * p.base_radius * lump * (1.0 - crack.min(0.6))
        })
        .collect();
    let mut rows = vec![];
    let mut scale = 1.0;
    for l in 0..layers {
        let (z0, z1) = (l as f32 * lh, (l + 1) as f32 * lh);
        let jitter = |z: f32, i: usize| {
            let a = i as f32 / CLIFF_SIDES as f32 * TAU;
            1.0 + p.noise_strength * 0.3 * field.fbm(Vec3::new(a.cos() * 2.0, a.sin() * 2.0, z * 3.0) * p.noise_scale + off)
        };
        for z in [z0, z1] {
            rows.push(footprint.iter().enumerate().map(|(i, &f)| f * scale * jitter(z0, i) + Vec3::Z * z).collect::<Vec<_>>());
        }
        scale *= 1.0 - range(&mut r, 0.04, 0.12);
    }
    let (mut m, ids) = loft(&rows, true, false, false);
    let bottom: Vec<_> = ids[0].iter().rev().copied().collect();
    cap(&mut m, &bottom);
    cap(&mut m, ids.last().unwrap());
    m
}

fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}
