//! Quartz Crystal clusters (hexagonal prisms with pyramidal terminations,
//! tilted within the spread angle, optional side shards and a rock base)
//! and Stalagmites / Stalactites (tapered, fluted drip columns with ring
//! bulges).

use super::rock::rock_pile;
use super::{Field, GeoGenParams, GeoMode, range, rng, tilt};
use crate::mesh::EditablePoly;
use crate::splines::sweep::{cap, loft};
use bevy::math::{Mat4, Quat, Vec3};
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

const CRYSTAL_SIDES: usize = 6;

fn ring(r: f32, z: f32, sides: usize, phase: f32) -> Vec<Vec3> {
    (0..sides).map(|i| i as f32 / sides as f32 * TAU + phase).map(|a| Vec3::new(a.cos() * r, a.sin() * r, z)).collect()
}

/// Lofts rings bottom -> apex and caps the bottom.
fn column(rows: Vec<Vec<Vec3>>) -> EditablePoly {
    let (mut m, ids) = loft(&rows, true, false, true);
    let bottom: Vec<_> = ids[0].iter().rev().copied().collect();
    cap(&mut m, &bottom);
    m.finalize();
    m
}

/// One hexagonal quartz spire: prism body, shoulder, pyramidal tip.
/// The base sinks a little below z = 0 so it grows out of the cluster.
pub fn spire(radius: f32, height: f32, taper: f32, r: &mut ChaCha8Rng) -> EditablePoly {
    let phase = range(r, 0.0, TAU);
    let tip = radius * range(r, 1.2, 2.0);
    let shoulder = (height - tip).max(height * 0.4);
    let top_r = radius * (1.0 - taper.clamp(0.0, 0.9) * 0.6);
    let apex = Vec3::new(range(r, -0.15, 0.15) * radius, range(r, -0.15, 0.15) * radius, height);
    column(vec![ring(radius, -radius * 0.4, CRYSTAL_SIDES, phase), ring(top_r, shoulder, CRYSTAL_SIDES, phase), vec![apex; CRYSTAL_SIDES]])
}

pub fn crystal_cluster(p: &GeoGenParams) -> EditablePoly {
    let mut out = EditablePoly::new();
    let mut r = rng(p.seed, 10);
    let n = p.count.clamp(1, 50);
    let max_h = p.max_height.max(p.min_height);
    for i in 0..n {
        let h = if i == 0 { max_h } else { range(&mut r, p.min_height, max_h) };
        let rad = p.base_radius * 0.32 * (h / max_h.max(1e-3)).sqrt() * range(&mut r, 0.8, 1.15);
        let q = if i == 0 { tilt(&mut r, p.spread * 0.25) } else { tilt(&mut r, p.spread) };
        let a = range(&mut r, 0.0, TAU);
        let d = if i == 0 { 0.0 } else { p.base_radius * range(&mut r, 0.1, 0.7) };
        let at = Vec3::new(a.cos() * d, a.sin() * d, 0.0);
        let s = spire(rad, h, p.tip_taper, &mut r);
        let m = Mat4::from_rotation_translation(q, at);
        out.append(&s, m);
        if p.side_shards && range(&mut r, 0.0, 1.0) < 0.6 {
            let up = range(&mut r, 0.2, 0.5) * h;
            let side = Quat::from_rotation_z(range(&mut r, 0.0, TAU)) * Quat::from_rotation_x(range(&mut r, 0.6, 1.1));
            let shard = spire(rad * 0.45, h * range(&mut r, 0.25, 0.4), p.tip_taper, &mut r);
            out.append(&shard, m * Mat4::from_rotation_translation(side, Vec3::new(0.0, 0.0, up)));
        }
    }
    if p.cluster_base {
        let base = GeoGenParams {
            mode: GeoMode::Rock,
            count: 1,
            min_height: p.base_radius * 0.45,
            max_height: p.base_radius * 0.45,
            base_radius: p.base_radius * 0.9,
            ..p.clone()
        };
        out.append(&rock_pile(&base), Mat4::from_translation(Vec3::new(0.0, 0.0, -p.base_radius * 0.12)));
    }
    out
}

/// One drip column: fluted cross-section, stepped ring bulges, tapering
/// to a point.
fn drip(p: &GeoGenParams, radius: f32, height: f32, r: &mut ChaCha8Rng, field: &Field) -> EditablePoly {
    let sides = 18;
    let rows_n = 16;
    let flutes = p.facets.clamp(0, 30) as f32;
    let flute_phase = range(r, 0.0, TAU);
    let bulges: Vec<(f32, f32)> = (0..3).map(|_| (range(r, 0.15, 0.8), range(r, 0.05, 0.18))).collect();
    let power = 0.6 + p.tip_taper.clamp(0.0, 1.0) * 1.4;
    let off = Vec3::splat(range(r, 0.0, 50.0));
    let mut rows = vec![];
    for k in 0..=rows_n {
        let t = k as f32 / rows_n as f32;
        if k == rows_n {
            rows.push(vec![Vec3::new(0.0, 0.0, height); sides]);
            break;
        }
        let bulge: f32 = bulges.iter().map(|(c, a)| a * (-((t - c) / 0.05).powi(2)).exp()).sum();
        let base_r = radius * (1.0 - t).powf(power) * (1.0 + bulge);
        rows.push(
            (0..sides)
                .map(|i| {
                    let a = i as f32 / sides as f32 * TAU;
                    let flute = if flutes > 0.0 { 1.0 + 0.1 * (a * flutes + flute_phase).cos() } else { 1.0 };
                    let n = field.fbm(Vec3::new(a.cos(), a.sin(), t * 2.0) * p.noise_scale + off);
                    let rr = base_r * flute * (1.0 + n * p.noise_strength * 0.5);
                    Vec3::new(a.cos() * rr, a.sin() * rr, t * height)
                })
                .collect(),
        );
    }
    column(rows)
}

pub fn stalagmites(p: &GeoGenParams) -> EditablePoly {
    let mut out = EditablePoly::new();
    let mut r = rng(p.seed, 20);
    let field = Field::new(p.seed);
    let max_h = p.max_height.max(p.min_height);
    for i in 0..p.count.clamp(1, 50) {
        let h = if i == 0 { max_h } else { range(&mut r, p.min_height, max_h) };
        let rad = p.base_radius * if i == 0 { 1.0 } else { range(&mut r, 0.5, 0.9) };
        let a = range(&mut r, 0.0, TAU);
        let d = if i == 0 { 0.0 } else { p.base_radius * range(&mut r, 1.5, 3.0) };
        let q = tilt(&mut r, p.spread);
        let col = drip(p, rad, h, &mut r, &field);
        out.append(&col, Mat4::from_rotation_translation(q, Vec3::new(a.cos() * d, a.sin() * d, 0.0)));
    }
    if p.hanging {
        // Stalactite: hangs from z = 0 (the ceiling) downwards.
        out.transform(Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0)));
    }
    out
}
