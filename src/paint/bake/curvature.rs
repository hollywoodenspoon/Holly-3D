//! Curvature from the mesh's creases: every edge where two faces meet at
//! an angle is convex (an outside corner, where paint chips) or concave
//! (an inside corner, where dirt collects). A texel's curvature is the sum
//! of the creases within reach, fading with distance.

use crate::mesh::adjacency::Adjacency;
use crate::mesh::{EditablePoly, PolyId};
use bevy::math::{Mat4, Vec3};
use std::collections::HashMap;

struct Crease {
    a: Vec3,
    b: Vec3,
    /// Signed strength: +1 = a 90 degree (or sharper) outside corner.
    k: f32,
    /// Normals of the two faces that meet here.
    n: [Vec3; 2],
}

pub struct Edges {
    creases: Vec<Crease>,
    /// Crease indices by grid cell (cells are one `radius` wide).
    grid: HashMap<(i32, i32, i32), Vec<u32>>,
    radius: f32,
}

fn dist_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

impl Edges {
    pub fn build(mesh: &EditablePoly, world: Mat4, radius: f32) -> Edges {
        let adj = Adjacency::build(mesh);
        let face = |p: PolyId| {
            let pts: Vec<Vec3> = mesh.polygons[p].vertex_ids().map(|v| world.transform_point3(mesh.pos(v))).collect();
            let c = pts.iter().sum::<Vec3>() / pts.len() as f32;
            // Newell's method: robust for n-gons.
            let n = (0..pts.len()).map(|i| pts[i].cross(pts[(i + 1) % pts.len()])).sum::<Vec3>().normalize_or_zero();
            (c, n)
        };
        let mut e = Edges { creases: vec![], grid: HashMap::new(), radius: radius.max(1e-6) };
        for ((v0, v1), faces) in &adj.edge_faces {
            let [f0, f1] = faces[..] else { continue };
            let ((c0, n0), (c1, n1)) = (face(f0), face(f1));
            let angle = n0.dot(n1).clamp(-1.0, 1.0).acos();
            if angle < 0.12 {
                continue;
            }
            // The neighbour's centre lies below this face's plane on a convex edge.
            let sign = if (c1 - c0).dot(n0) < 0.0 { 1.0 } else { -1.0 };
            let (a, b) = (world.transform_point3(mesh.pos(*v0)), world.transform_point3(mesh.pos(*v1)));
            let i = e.creases.len() as u32;
            e.creases.push(Crease { a, b, k: sign * (angle / std::f32::consts::FRAC_PI_2).min(1.0), n: [n0, n1] });
            let (lo, hi) = (e.cell(a.min(b) - Vec3::splat(e.radius)), e.cell(a.max(b) + Vec3::splat(e.radius)));
            // Very long edges on a fine grid would flood it; they are still found through their end cells.
            if (hi.0 - lo.0 + 1) as i64 * (hi.1 - lo.1 + 1) as i64 * (hi.2 - lo.2 + 1) as i64 > 4096 {
                for c in [e.cell(a), e.cell(b)] {
                    e.grid.entry(c).or_default().push(i);
                }
                continue;
            }
            for x in lo.0..=hi.0 {
                for y in lo.1..=hi.1 {
                    for z in lo.2..=hi.2 {
                        e.grid.entry((x, y, z)).or_default().push(i);
                    }
                }
            }
        }
        e
    }

    fn cell(&self, p: Vec3) -> (i32, i32, i32) {
        let q = (p / self.radius).floor();
        (q.x as i32, q.y as i32, q.z as i32)
    }

    /// Curvature 0..1 at point `p` on a face with normal `n` (0.5 = flat).
    pub fn at(&self, p: Vec3, n: Vec3) -> f32 {
        let Some(list) = self.grid.get(&self.cell(p)) else { return 0.5 };
        let (mut convex, mut concave) = (0.0f32, 0.0f32);
        for &i in list {
            let c = &self.creases[i as usize];
            // Only creases of the surface this point lies on (not the far
            // side of a thin wall).
            if c.n[0].dot(n) < 0.3 && c.n[1].dot(n) < 0.3 {
                continue;
            }
            let t = 1.0 - (dist_to_segment(p, c.a, c.b) / self.radius).min(1.0);
            let w = c.k.abs() * t * t * (3.0 - 2.0 * t);
            if c.k > 0.0 {
                convex = convex.max(w);
            } else {
                concave = concave.max(w);
            }
        }
        (0.5 + 0.5 * (convex - concave)).clamp(0.0, 1.0)
    }
}
