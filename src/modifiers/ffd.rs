//! FFD 2x2x2 / 3x3x3 / 4x4x4: Bernstein-polynomial lattice deformer fitted to
//! the modifier's input bounding box ("Set Volume" at apply time).

use crate::mesh::{Aabb, EditablePoly};
use crate::viewport::camera::Ray;
use crate::viewport::picking::ray_triangle;
use bevy::math::{Mat4, Vec3};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FfdMod {
    /// Control points per axis (2, 3 or 4).
    pub dim: usize,
    /// Lattice volume in the modifier's input space.
    pub bounds: Aabb,
    /// Lattice sub-object transform (input space): moves the cage relative to the mesh.
    pub lattice: Mat4,
    /// Control points in normalized lattice space; index `i + n*(j + n*k)`.
    pub points: Vec<Vec3>,
    pub selected: Vec<bool>,
    /// Deform only vertices inside the lattice volume.
    pub only_in_volume: bool,
}

fn binomial(n: usize, k: usize) -> f32 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f32 / (i + 1) as f32)
}

/// Bernstein basis values B(n-1, i, t) for i in 0..n.
fn bernstein(n: usize, t: f32) -> [f32; 4] {
    let mut out = [0.0; 4];
    let d = n - 1;
    for (i, o) in out.iter_mut().enumerate().take(n) {
        *o = binomial(d, i) * t.powi(i as i32) * (1.0 - t).powi((d - i) as i32);
    }
    out
}

impl FfdMod {
    pub fn new(dim: usize) -> Self {
        let dim = dim.clamp(2, 4);
        let bounds = Aabb { min: Vec3::splat(-0.5), max: Vec3::splat(0.5) };
        let mut f = Self { dim, bounds, lattice: Mat4::IDENTITY, points: vec![], selected: vec![], only_in_volume: true };
        f.reset();
        f
    }

    pub fn name(&self) -> &'static str {
        match self.dim {
            2 => "FFD 2x2x2",
            3 => "FFD 3x3x3",
            _ => "FFD 4x4x4",
        }
    }

    pub fn index(&self, i: usize, j: usize, k: usize) -> usize {
        i + self.dim * (j + self.dim * k)
    }

    pub fn coords(&self, idx: usize) -> (usize, usize, usize) {
        let n = self.dim;
        (idx % n, (idx / n) % n, idx / (n * n))
    }

    /// Reset: control points back to the regular grid, lattice back in place.
    pub fn reset(&mut self) {
        let n = self.dim;
        let s = (n - 1) as f32;
        self.points = (0..n * n * n)
            .map(|idx| {
                let (i, j, k) = (idx % n, (idx / n) % n, idx / (n * n));
                Vec3::new(i as f32 / s, j as f32 / s, k as f32 / s)
            })
            .collect();
        self.selected = vec![false; self.points.len()];
        self.lattice = Mat4::IDENTITY;
    }

    /// Fits the lattice to `input` (padding flat axes so planes still deform).
    pub fn fit(&mut self, input: &EditablePoly) {
        let Some(b) = input.bounds() else { return };
        let size = b.size();
        let pad = (size.max_element() * 0.05).max(0.01);
        let mut b = b;
        for a in 0..3 {
            if size[a] < pad {
                b.min[a] -= pad * 0.5;
                b.max[a] += pad * 0.5;
            }
        }
        self.bounds = b;
    }

    fn size(&self) -> Vec3 {
        self.bounds.size().max(Vec3::splat(1e-6))
    }

    /// Control point `idx` in input (mesh) space.
    pub fn point_pos(&self, idx: usize) -> Vec3 {
        self.lattice.transform_point3(self.bounds.min + self.points[idx] * self.size())
    }

    /// Moves control point `idx` to a position given in input space.
    pub fn set_point_pos(&mut self, idx: usize, p: Vec3) {
        let q = self.lattice.inverse().transform_point3(p);
        self.points[idx] = (q - self.bounds.min) / self.size();
    }

    /// Deforms a single input-space point.
    pub fn deform(&self, p: Vec3) -> Vec3 {
        let q = self.lattice.inverse().transform_point3(p);
        let s = (q - self.bounds.min) / self.size();
        let eps = 1e-4;
        if self.only_in_volume && (s.min_element() < -eps || s.max_element() > 1.0 + eps) {
            return p;
        }
        let n = self.dim;
        let (bu, bv, bw) = (bernstein(n, s.x), bernstein(n, s.y), bernstein(n, s.z));
        let mut out = Vec3::ZERO;
        for (k, bw) in bw.iter().enumerate().take(n) {
            for (j, bv) in bv.iter().enumerate().take(n) {
                let w = bv * bw;
                for (i, bu) in bu.iter().enumerate().take(n) {
                    out += self.points[self.index(i, j, k)] * (bu * w);
                }
            }
        }
        self.lattice.transform_point3(self.bounds.min + out * self.size())
    }

    pub fn apply(&self, mesh: &mut EditablePoly) {
        for v in mesh.vertices.values_mut() {
            v.position = self.deform(v.position);
        }
    }

    /// Conform to Shape: moves each control point toward the lattice center
    /// until it meets the input surface.
    pub fn conform(&mut self, input: &EditablePoly) {
        let tris: Vec<[Vec3; 3]> = input.polygons.keys().flat_map(|p| input.poly_triangles(p)).collect();
        let center = self.lattice.transform_point3(self.bounds.center());
        for idx in 0..self.points.len() {
            let p = self.point_pos(idx);
            let to_c = center - p;
            let dist = to_c.length();
            if dist < 1e-6 {
                continue;
            }
            let ray = Ray { origin: p, dir: to_c / dist };
            let hit = tris.iter().filter_map(|t| ray_triangle(&ray, t[0], t[1], t[2])).filter(|&t| t <= dist).min_by(f32::total_cmp);
            if let Some(t) = hit {
                self.set_point_pos(idx, ray.at(t));
            }
        }
    }

    /// All X / All Y / All Z: extends the selection along a lattice axis.
    pub fn select_along(&mut self, axis: usize) {
        let picked: Vec<(usize, usize, usize)> = (0..self.points.len()).filter(|&i| self.selected[i]).map(|i| self.coords(i)).collect();
        for idx in 0..self.points.len() {
            let c = self.coords(idx);
            let hit = picked.iter().any(|p| match axis {
                0 => p.1 == c.1 && p.2 == c.2,
                1 => p.0 == c.0 && p.2 == c.2,
                _ => p.0 == c.0 && p.1 == c.1,
            });
            if hit {
                self.selected[idx] = true;
            }
        }
    }

    /// Cage edges between neighbouring control points (index pairs).
    pub fn cage_edges(&self) -> Vec<(usize, usize)> {
        let n = self.dim;
        let mut out = vec![];
        for k in 0..n {
            for j in 0..n {
                for i in 0..n {
                    let a = self.index(i, j, k);
                    if i + 1 < n {
                        out.push((a, self.index(i + 1, j, k)));
                    }
                    if j + 1 < n {
                        out.push((a, self.index(i, j + 1, k)));
                    }
                    if k + 1 < n {
                        out.push((a, self.index(i, j, k + 1)));
                    }
                }
            }
        }
        out
    }
}
