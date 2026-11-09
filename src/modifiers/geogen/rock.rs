//! Jagged Rock / Boulder: a subdivided quad sphere displaced by FBM +
//! Worley noise, then carved by seeded random planar slices (vertices past
//! a plane are clamped onto it, giving crisp flat fracture faces).

use super::{Field, GeoGenParams, range, rng, unit};
use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Mat4, Vec2, Vec3};
use std::collections::HashMap;

/// Cube subdivided `n` x `n` per face and projected to a unit sphere:
/// 6n² + 2 vertices, 12n² edges, 6n² quads.
pub fn quad_sphere(n: u32) -> EditablePoly {
    let n = n.max(1) as i32;
    let mut m = EditablePoly::new();
    let mut seen: HashMap<[i32; 3], VertexId> = HashMap::new();
    // (normal axis, u axis, v axis) chosen so every face winds outward.
    let faces = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (-Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (-Vec3::Z, Vec3::Y, Vec3::X),
    ];
    for (f, (nrm, u, v)) in faces.into_iter().enumerate() {
        let mut grid = vec![];
        for j in 0..=n {
            for i in 0..=n {
                let lattice = nrm * n as f32 + u * (2 * i - n) as f32 + v * (2 * j - n) as f32;
                let key = lattice.round().as_ivec3().to_array();
                let p = (lattice / n as f32).normalize();
                grid.push(*seen.entry(key).or_insert_with(|| m.add_vertex(p)));
            }
        }
        let w = (n + 1) as usize;
        for j in 0..n as usize {
            for i in 0..n as usize {
                let ids = [grid[j * w + i], grid[j * w + i + 1], grid[(j + 1) * w + i + 1], grid[(j + 1) * w + i]];
                let uv = |a: usize, b: usize| Vec2::new((f as f32 + a as f32 / n as f32) / 6.0, b as f32 / n as f32);
                let uvs = [uv(i, j), uv(i + 1, j), uv(i + 1, j + 1), uv(i, j + 1)];
                m.add_polygon_uv(&ids, &uvs);
            }
        }
    }
    m.finalize();
    m
}

/// Noise displacement along the radial direction plus planar fracture
/// cuts, applied in place (also the GeoGen modifier's Deform mode).
pub fn deform_rock(m: &mut EditablePoly, p: &GeoGenParams) {
    let Some(b) = m.bounds() else { return };
    let (c, half) = (b.center(), (b.size() * 0.5).max(Vec3::splat(1e-4)));
    let field = Field::new(p.seed);
    let off = Vec3::splat(p.seed as f32 * 0.37);
    for v in m.vertices.values_mut() {
        let local = (v.position - c) / half;
        let n = field.rocky(local * p.noise_scale + off);
        let dir = (v.position - c).normalize_or(Vec3::Z);
        v.position += dir * (n * p.noise_strength) * half.min_element();
    }
    let mut r = rng(p.seed, 3);
    for _ in 0..p.facets {
        let d = unit(&mut r);
        let reach = m.vertices.values().map(|v| (v.position - c).dot(d)).fold(f32::MIN, f32::max);
        let cut = reach * range(&mut r, 0.5, 0.82);
        for v in m.vertices.values_mut() {
            let s = (v.position - c).dot(d);
            if s > cut {
                v.position -= d * (s - cut);
            }
        }
    }
}

fn boulder(p: &GeoGenParams, seed: u32, radius: f32, height: f32) -> EditablePoly {
    let mut m = quad_sphere(8);
    let mut r = rng(seed, 1);
    let stretch = Vec3::new(radius * range(&mut r, 0.85, 1.15), radius * range(&mut r, 0.75, 1.05), height * 0.5);
    m.transform(Mat4::from_scale(stretch));
    deform_rock(&mut m, &GeoGenParams { seed, ..p.clone() });
    // Flatten the underside so the rock sits on the ground, pivot at its base.
    let min_z = m.bounds().map_or(0.0, |b| b.min.z);
    let floor = min_z + height * 0.12;
    for v in m.vertices.values_mut() {
        v.position.z = v.position.z.max(floor) - floor;
    }
    m
}

/// One boulder, or a scattered pile of `count` boulders.
pub fn rock_pile(p: &GeoGenParams) -> EditablePoly {
    let mut out = EditablePoly::new();
    let mut r = rng(p.seed, 2);
    let n = p.count.clamp(1, 50);
    for i in 0..n {
        let h = range(&mut r, p.min_height, p.max_height.max(p.min_height));
        let scale = if i == 0 { 1.0 } else { range(&mut r, 0.35, 0.7) };
        let rock = boulder(p, p.seed.wrapping_add(i * 101), p.base_radius * scale, h * scale);
        let at = if i == 0 {
            Vec3::ZERO
        } else {
            let a = range(&mut r, 0.0, std::f32::consts::TAU);
            let d = p.base_radius * range(&mut r, 1.0, 1.8);
            Vec3::new(a.cos() * d, a.sin() * d, 0.0)
        };
        out.append(
            &rock,
            Mat4::from_rotation_translation(bevy::math::Quat::from_rotation_z(range(&mut r, 0.0, std::f32::consts::TAU)), at),
        );
    }
    out
}
