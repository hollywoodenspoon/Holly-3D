//! Distortion-free (equal-angle) quad sphere: a cube whose face lattice is
//! warped by `tan` so every quad spans about the same solid angle, with
//! equirectangular (longitude / latitude) UVs that wrap cleanly at the seam
//! and poles, so any planet texture maps straight on.

use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Vec2, Vec3};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_4, PI, TAU};

/// Equirect UV of a unit direction (u = longitude, v = 0 at the north pole).
pub fn lat_long_uv(d: Vec3) -> Vec2 {
    Vec2::new((0.5 + d.y.atan2(d.x) / TAU).rem_euclid(1.0), d.z.clamp(-1.0, 1.0).acos() / PI)
}

/// Unit equal-angle quad sphere with `n` x `n` quads per cube face.
pub fn equal_angle_sphere(n: u32) -> EditablePoly {
    let n = n.max(1) as i32;
    let mut m = EditablePoly::new();
    let mut seen: HashMap<[i32; 3], VertexId> = HashMap::new();
    let faces = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (-Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (-Vec3::Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (-Vec3::Z, Vec3::Y, Vec3::X),
    ];
    let warp = |k: i32| (FRAC_PI_4 * k as f32 / n as f32).tan();
    for (nrm, u, v) in faces {
        let mut grid = vec![];
        for j in 0..=n {
            for i in 0..=n {
                let (a, b) = (2 * i - n, 2 * j - n);
                let key = (nrm * n as f32 + u * a as f32 + v * b as f32).round().as_ivec3().to_array();
                let p = (nrm + u * warp(a) + v * warp(b)).normalize();
                grid.push(*seen.entry(key).or_insert_with(|| m.add_vertex(p)));
            }
        }
        let w = (n + 1) as usize;
        for j in 0..n as usize {
            for i in 0..n as usize {
                let ids = [grid[j * w + i], grid[j * w + i + 1], grid[(j + 1) * w + i + 1], grid[(j + 1) * w + i]];
                let uvs = seam_uvs(&ids.map(|id| m.vertices[id].position));
                let p = m.add_polygon_uv(&ids, &uvs);
                m.polygons[p].smoothing_group = 1;
            }
        }
    }
    m.finalize();
    m
}

/// Lat-long UVs for one polygon, unwrapped across the U seam and with pole
/// corners taking the polygon's mean longitude.
fn seam_uvs(ps: &[Vec3; 4]) -> [Vec2; 4] {
    let mut uv = ps.map(lat_long_uv);
    let pole = |p: Vec3| p.truncate().length() < 1e-5;
    let (lo, hi) = uv.iter().zip(ps).filter(|(_, p)| !pole(**p)).fold((1.0f32, 0.0f32), |(lo, hi), (t, _)| (lo.min(t.x), hi.max(t.x)));
    if hi - lo > 0.5 {
        for (t, p) in uv.iter_mut().zip(ps) {
            if !pole(*p) && t.x < 0.5 {
                t.x += 1.0;
            }
        }
    }
    let rest: Vec<f32> = uv.iter().zip(ps).filter(|(_, p)| !pole(**p)).map(|(t, _)| t.x).collect();
    let mean = rest.iter().sum::<f32>() / rest.len().max(1) as f32;
    for (t, p) in uv.iter_mut().zip(ps) {
        if pole(*p) {
            t.x = mean;
        }
    }
    uv
}

/// Flat ring (annulus) in the XY plane from `inner` to `outer` radius:
/// `bands` radial rows x `sides` around. UV u = radial (0 inner .. 1
/// outer), v = around.
pub fn annulus(inner: f32, outer: f32, bands: u32, sides: u32) -> (EditablePoly, Vec<VertexId>) {
    let (bands, sides) = (bands.max(1) as usize, sides.max(3) as usize);
    let mut m = EditablePoly::new();
    let mut ids = Vec::with_capacity((bands + 1) * sides);
    for b in 0..=bands {
        let r = inner + (outer - inner) * b as f32 / bands as f32;
        for s in 0..sides {
            let a = TAU * s as f32 / sides as f32;
            ids.push(m.add_vertex(Vec3::new(r * a.cos(), r * a.sin(), 0.0)));
        }
    }
    for b in 0..bands {
        for s in 0..sides {
            let s1 = (s + 1) % sides;
            // Inner -> outer -> around: faces +Z.
            let q = [ids[b * sides + s], ids[(b + 1) * sides + s], ids[(b + 1) * sides + s1], ids[b * sides + s1]];
            let (u0, u1) = (b as f32 / bands as f32, (b + 1) as f32 / bands as f32);
            let (v0, v1) = (s as f32 / sides as f32, (s + 1) as f32 / sides as f32);
            let p = m.add_polygon_uv(&q, &[Vec2::new(u0, v0), Vec2::new(u1, v0), Vec2::new(u1, v1), Vec2::new(u0, v1)]);
            m.polygons[p].smoothing_group = 1;
        }
    }
    m.finalize();
    (m, ids)
}
