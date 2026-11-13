//! Sphere, Cylinder/Cone and Torus primitives.

use super::{ring, stitch_rings};
use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

/// UV sphere centered at the origin (sphere pivot is its center).
/// `hemisphere` in 0..1 chops that fraction off the bottom and caps it.
pub fn build_sphere(radius: f32, segments: u32, hemisphere: f32) -> EditablePoly {
    let mut m = EditablePoly::new();
    let sides = segments as usize;
    let h = hemisphere.clamp(0.0, 0.95);
    let phi_max = PI * (1.0 - h);
    let rings_n = ((segments / 2) as f32 * (1.0 - h)).ceil().max(2.0) as usize;
    let mut rings = Vec::new();
    let mut vs = Vec::new();
    // bottom (phi = phi_max) to top (phi = 0)
    for k in (0..=rings_n).rev() {
        let phi = phi_max * k as f32 / rings_n as f32;
        let r = if k == 0 || (k == rings_n && h == 0.0) { 0.0 } else { radius * phi.sin() };
        rings.push(ring(&mut m, r, radius * phi.cos(), sides));
        vs.push(1.0 - phi / PI);
    }
    stitch_rings(&mut m, &rings, &vs, sides, 1, 1);
    if h > 0.0 {
        let bottom: Vec<VertexId> = rings[0].iter().rev().copied().collect();
        let uvs = planar_uvs(&m, &bottom, radius);
        let p = m.add_polygon_uv(&bottom, &uvs);
        m.polygons[p].smoothing_group = 2;
    }
    m
}

fn planar_uvs(m: &EditablePoly, vs: &[VertexId], r: f32) -> Vec<Vec2> {
    vs.iter()
        .map(|&v| {
            let p = m.pos(v);
            Vec2::new(p.x / (2.0 * r.max(1e-6)) + 0.5, p.y / (2.0 * r.max(1e-6)) + 0.5)
        })
        .collect()
}

/// Cap made of an inner N-gon plus concentric quad rings sharing `outer`.
#[allow(clippy::too_many_arguments)]
fn cap(m: &mut EditablePoly, outer: &[VertexId], radius: f32, z: f32, cap_segs: u32, top: bool, sg: u32, mat: u16) {
    if outer.len() < 3 {
        return;
    }
    let sides = outer.len();
    let mut rings: Vec<Vec<VertexId>> = (1..cap_segs).map(|c| ring(m, radius * c as f32 / cap_segs as f32, z, sides)).collect();
    rings.push(outer.to_vec());
    let add = |m: &mut EditablePoly, vs: Vec<VertexId>| {
        let vs: Vec<VertexId> = if top { vs } else { vs.into_iter().rev().collect() };
        let uvs = planar_uvs(m, &vs, radius);
        let p = m.add_polygon_uv(&vs, &uvs);
        m.polygons[p].smoothing_group = sg;
        m.polygons[p].material_id = mat;
    };
    add(m, rings[0].clone());
    for r in 0..rings.len() - 1 {
        let (inner, outer) = (&rings[r], &rings[r + 1]);
        for j in 0..sides {
            let j1 = (j + 1) % sides;
            add(m, vec![outer[j], outer[j1], inner[j1], inner[j]]);
        }
    }
}

/// Cylinder when radius1 == radius2, cone otherwise. Base on Z = 0.
pub fn build_cone(r1: f32, r2: f32, height: f32, hsegs: u32, cap_segs: u32, sides: u32) -> EditablePoly {
    let mut m = EditablePoly::new();
    let sides = sides as usize;
    let mut rings = Vec::new();
    let mut vs = Vec::new();
    for k in 0..=hsegs {
        let t = k as f32 / hsegs as f32;
        rings.push(ring(&mut m, r1 + (r2 - r1) * t, height * t, sides));
        vs.push(t);
    }
    stitch_rings(&mut m, &rings, &vs, sides, 1, 3);
    let bottom = rings[0].clone();
    let top = rings[rings.len() - 1].clone();
    cap(&mut m, &bottom, r1, 0.0, cap_segs, false, 2, 1);
    cap(&mut m, &top, r2, height, cap_segs, true, 4, 2);
    m
}

/// Torus centered at the origin lying in the XY plane.
pub fn build_torus(r1: f32, r2: f32, segments: u32, sides: u32) -> EditablePoly {
    let mut m = EditablePoly::new();
    let (ns, nt) = (segments as usize, sides as usize);
    let mut ids = vec![vec![VertexId::default(); nt]; ns];
    for (i, row) in ids.iter_mut().enumerate() {
        let a = i as f32 / ns as f32 * TAU;
        let dir = Vec3::new(a.cos(), a.sin(), 0.0);
        for (j, id) in row.iter_mut().enumerate() {
            let b = j as f32 / nt as f32 * TAU;
            let p = dir * (r1 + r2 * b.cos()) + Vec3::Z * (r2 * b.sin());
            *id = m.add_vertex(p);
        }
    }
    for i in 0..ns {
        for j in 0..nt {
            let (i1, j1) = ((i + 1) % ns, (j + 1) % nt);
            let vs = [ids[i][j], ids[i1][j], ids[i1][j1], ids[i][j1]];
            let uv = |a: usize, b: usize| Vec2::new(a as f32 / ns as f32, b as f32 / nt as f32);
            let p = m.add_polygon_uv(&vs, &[uv(i, j), uv(i + 1, j), uv(i + 1, j + 1), uv(i, j + 1)]);
            m.polygons[p].smoothing_group = 1;
        }
    }
    m
}
