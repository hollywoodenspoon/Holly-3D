//! Geometry helpers for fitting the Spider rig to imported parts: world
//! meshes, islands with principal axes, and surface contact between parts.

use crate::character::face_cloner::math::jacobi;
use crate::mesh::adjacency::Adjacency;
use crate::mesh::{EditablePoly, PolyId};
use crate::scene::{ObjId, Scene};
use bevy::math::Vec3;
use std::collections::{HashMap, HashSet};

/// A connected group of polygons with its principal axes.
#[derive(Clone, Debug)]
pub struct Island {
    pub polys: Vec<PolyId>,
    pub verts: usize,
    pub centre: Vec3,
    /// Principal axes, smallest spread first.
    pub axes: [Vec3; 3],
    /// Standard deviation along each axis.
    pub spread: Vec3,
    pub min: Vec3,
    pub max: Vec3,
}

impl Island {
    /// A thin pin or bolt: long along its main axis, round across it.
    pub fn is_pin(&self) -> bool {
        self.spread.z > 2.5 * self.spread.y && self.spread.y < 1.6 * self.spread.x.max(1e-6)
    }
}

/// The object's evaluated mesh in world space.
pub fn world_mesh(s: &Scene, id: ObjId) -> EditablePoly {
    let o = &s.objects[id];
    let mut m = (*o.evaluated_mesh).clone();
    m.transform(o.world_matrix());
    m
}

/// Centre, principal axes (smallest first) and spreads of `pts`.
pub fn pca(pts: &[Vec3]) -> (Vec3, [Vec3; 3], Vec3) {
    let n = pts.len().max(1) as f32;
    let c = pts.iter().copied().sum::<Vec3>() / n;
    let mut a = vec![0.0f64; 9];
    for p in pts {
        let d = (*p - c).as_dvec3();
        let d = [d.x, d.y, d.z];
        for i in 0..3 {
            for j in 0..3 {
                a[i * 3 + j] += d[i] * d[j] / n as f64;
            }
        }
    }
    let (val, vec) = jacobi(a, 3);
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| val[i].total_cmp(&val[j]));
    let axis = |k: usize| Vec3::new(vec[k] as f32, vec[3 + k] as f32, vec[6 + k] as f32).normalize_or(Vec3::X);
    let axes = order.map(axis);
    let spread = Vec3::from_array(order.map(|k| val[k].max(0.0).sqrt() as f32));
    (c, axes, spread)
}

/// Edge-connected polygon islands of `m`, largest first.
pub fn islands(m: &EditablePoly) -> Vec<Island> {
    let adj = Adjacency::build(m);
    let mut out: Vec<Island> = adj.elements(m).into_iter().map(|polys| island(m, polys)).collect();
    out.sort_by_key(|i| std::cmp::Reverse(i.verts));
    out
}

pub fn island(m: &EditablePoly, polys: Vec<PolyId>) -> Island {
    let ids: HashSet<_> = polys.iter().flat_map(|&p| m.polygons[p].corners.iter().map(|c| c.vertex_id)).collect();
    let pts: Vec<Vec3> = ids.iter().map(|&v| m.vertices[v].position).collect();
    let (centre, axes, spread) = pca(&pts);
    let min = pts.iter().copied().fold(Vec3::splat(f32::MAX), Vec3::min);
    let max = pts.iter().copied().fold(Vec3::splat(f32::MIN), Vec3::max);
    Island { polys, verts: pts.len(), centre, axes, spread, min, max }
}

/// Points spread over every polygon at most `step` apart (vertices, edge
/// points and a fan of interior points).
pub fn surface_samples(m: &EditablePoly, step: f32) -> Vec<Vec3> {
    let mut out: Vec<Vec3> = m.vertices.values().map(|v| v.position).collect();
    for p in m.polygons.values() {
        let pts: Vec<Vec3> = p.corners.iter().map(|c| m.vertices[c.vertex_id].position).collect();
        let c = pts.iter().copied().sum::<Vec3>() / pts.len() as f32;
        for (i, &a) in pts.iter().enumerate() {
            let b = pts[(i + 1) % pts.len()];
            let n = ((a.distance(b).max(a.distance(c))) / step).ceil().min(64.0) as usize;
            for u in 0..=n {
                for v in 0..=(n - u) {
                    let (fu, fv) = (u as f32 / n.max(1) as f32, v as f32 / n.max(1) as f32);
                    out.push(c + (a - c) * fu + (b - c) * fv);
                }
            }
        }
    }
    out
}

/// A voxel hash of points for "how close is this point to that part".
pub struct Cloud {
    cell: f32,
    grid: HashMap<(i32, i32, i32), Vec<Vec3>>,
}

impl Cloud {
    pub fn new(pts: &[Vec3], cell: f32) -> Self {
        let mut grid: HashMap<_, Vec<Vec3>> = HashMap::new();
        for &p in pts {
            grid.entry(key(p, cell)).or_default().push(p);
        }
        Self { cell, grid }
    }

    /// Is any point within `cell` of `p`?
    pub fn near(&self, p: Vec3) -> bool {
        let (x, y, z) = key(p, self.cell);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(v) = self.grid.get(&(x + dx, y + dy, z + dz))
                        && v.iter().any(|q| q.distance_squared(p) <= self.cell * self.cell)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn key(p: Vec3, cell: f32) -> (i32, i32, i32) {
    let k = (p / cell).floor();
    (k.x as i32, k.y as i32, k.z as i32)
}

/// Points of `a` within reach of `b` (their contact patch).
pub fn contact(a: &[Vec3], b: &Cloud) -> Vec<Vec3> {
    a.iter().copied().filter(|&p| b.near(p)).collect()
}

/// Horizontal unit vector (Z dropped).
pub fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, 0.0).normalize_or(Vec3::X)
}
