//! The light preview mesh of a `CloudShape`: one smooth lat-long sphere per
//! puff (fewer segments on the small detail puffs), with the underside
//! flattened toward the cloud base.

use super::{CloudShape, Puff};
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

/// Material ID of cloud polygons (Clouds pack).
pub const CLOUD_ID: u16 = 1;

/// (longitude, latitude) segments by detail level.
pub fn segments(level: u8) -> (usize, usize) {
    match level {
        0 => (12, 7),
        1 => (10, 6),
        _ => (8, 5),
    }
}

#[cfg(test)]
/// Vertices one puff adds.
pub fn verts_per_puff(level: u8) -> usize {
    let (lon, lat) = segments(level);
    lon * (lat - 1) + 2
}

fn sphere(m: &mut EditablePoly, q: &Puff, flat: f32) {
    let (lon, lat) = segments(q.level);
    let put = |m: &mut EditablePoly, d: Vec3| {
        let mut p = q.c + Vec3::new(d.x, d.y, d.z * q.sq) * q.r;
        if p.z < 0.0 {
            p.z *= 1.0 - flat;
        }
        m.add_vertex(p)
    };
    let bottom = put(m, -Vec3::Z);
    let rings: Vec<Vec<_>> = (1..lat)
        .map(|i| {
            let th = PI * i as f32 / lat as f32;
            (0..lon)
                .map(|j| {
                    let ph = TAU * j as f32 / lon as f32;
                    put(m, Vec3::new(th.sin() * ph.cos(), th.sin() * ph.sin(), -th.cos()))
                })
                .collect()
        })
        .collect();
    let top = put(m, Vec3::Z);
    let uv = |i: usize, j: usize| Vec2::new(j as f32 / lon as f32, i as f32 / lat as f32);
    let add = |m: &mut EditablePoly, vs: &[_], uvs: &[Vec2]| {
        let p = m.add_polygon_uv(vs, uvs);
        (m.polygons[p].material_id, m.polygons[p].smoothing_group) = (CLOUD_ID, 1);
    };
    let t = rings.len() - 1;
    for j in 0..lon {
        let jn = (j + 1) % lon;
        add(m, &[bottom, rings[0][jn], rings[0][j]], &[uv(0, j), uv(1, j + 1), uv(1, j)]);
        add(m, &[top, rings[t][j], rings[t][jn]], &[uv(lat, j), uv(lat - 1, j), uv(lat - 1, j + 1)]);
        for i in 0..t {
            add(m, &[rings[i][j], rings[i][jn], rings[i + 1][jn], rings[i + 1][j]], &[uv(i + 1, j), uv(i + 1, j + 1), uv(i + 2, j + 1), uv(i + 2, j)]);
        }
    }
}

/// All puffs of `s` as one mesh; `flatness` 0..=1 cuts the underside flat.
pub fn build(s: &CloudShape, flatness: f32) -> EditablePoly {
    let mut m = EditablePoly::new();
    let flat = flatness.clamp(0.0, 1.0);
    for q in &s.puffs {
        sphere(&mut m, q, flat);
    }
    m
}
