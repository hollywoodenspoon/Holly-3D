//! Box and Plane primitives built from welded integer lattices.

use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Vec2, Vec3};
use std::collections::HashMap;

/// Box of size (sx, sy, sz) centered on X/Y with its base on Z = 0.
pub fn build_box(sx: f32, sy: f32, sz: f32, nx: u32, ny: u32, nz: u32) -> EditablePoly {
    let mut m = EditablePoly::new();
    let mut lattice: HashMap<(u32, u32, u32), VertexId> = HashMap::new();
    let n = [nx, ny, nz];
    let size = [sx, sy, sz];
    let mut vert = |m: &mut EditablePoly, c: [u32; 3]| -> VertexId {
        *lattice.entry((c[0], c[1], c[2])).or_insert_with(|| {
            let p = Vec3::new(
                (c[0] as f32 / n[0] as f32 - 0.5) * size[0],
                (c[1] as f32 / n[1] as f32 - 0.5) * size[1],
                c[2] as f32 / n[2] as f32 * size[2],
            );
            m.add_vertex(p)
        })
    };
    // (normal axis, at max?, u axis, v axis) chosen so (u x v) points outward.
    let faces: [(usize, bool, usize, usize, u16); 6] = [
        (2, false, 1, 0, 1), // bottom  -Z
        (2, true, 0, 1, 2),  // top     +Z
        (1, false, 0, 2, 3), // front   -Y
        (0, true, 1, 2, 4),  // right   +X
        (1, true, 2, 0, 5),  // back    +Y
        (0, false, 2, 1, 6), // left    -X
    ];
    for (axis, at_max, ua, va, mat) in faces {
        let (nu, nv) = (n[ua], n[va]);
        for i in 0..nu {
            for j in 0..nv {
                let corner = |di: u32, dj: u32| {
                    let mut c = [0u32; 3];
                    c[axis] = if at_max { n[axis] } else { 0 };
                    c[ua] = i + di;
                    c[va] = j + dj;
                    c
                };
                let quad = [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)];
                let ids: Vec<VertexId> = quad.iter().map(|&c| vert(&mut m, c)).collect();
                let uvs: Vec<Vec2> = [(0, 0), (1, 0), (1, 1), (0, 1)]
                    .iter()
                    .map(|&(di, dj)| Vec2::new((i + di) as f32 / nu as f32, (j + dj) as f32 / nv as f32))
                    .collect();
                let p = m.add_polygon_uv(&ids, &uvs);
                m.polygons[p].smoothing_group = 1 << (mat as u32 - 1);
                m.polygons[p].material_id = mat;
            }
        }
    }
    m
}

/// Flat plane in XY, facing +Z.
pub fn build_plane(sx: f32, sy: f32, nx: u32, ny: u32) -> EditablePoly {
    let mut m = EditablePoly::new();
    let mut ids = vec![vec![VertexId::default(); (ny + 1) as usize]; (nx + 1) as usize];
    for i in 0..=nx {
        for j in 0..=ny {
            let p = Vec3::new((i as f32 / nx as f32 - 0.5) * sx, (j as f32 / ny as f32 - 0.5) * sy, 0.0);
            ids[i as usize][j as usize] = m.add_vertex(p);
        }
    }
    for i in 0..nx as usize {
        for j in 0..ny as usize {
            let vs = [ids[i][j], ids[i + 1][j], ids[i + 1][j + 1], ids[i][j + 1]];
            let uv = |a: usize, b: usize| Vec2::new(a as f32 / nx as f32, b as f32 / ny as f32);
            let p = m.add_polygon_uv(&vs, &[uv(i, j), uv(i + 1, j), uv(i + 1, j + 1), uv(i, j + 1)]);
            m.polygons[p].smoothing_group = 1;
        }
    }
    m
}
