//! Symmetry: mirrors the mesh across a local plane (X, Y or Z), optionally
//! slicing along the mirror plane first and welding the seam.

use crate::mesh::ops::{slice, weld};
use crate::mesh::{EditablePoly, PolyId, SubObject};
use bevy::math::{Mat3, Mat4, Vec3};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SymmetryMod {
    pub axis: usize,
    /// Keep the negative side instead of the positive one.
    pub flip: bool,
    pub slice: bool,
    pub weld: bool,
    pub threshold: f32,
    /// Mirror plane offset along the axis.
    pub offset: f32,
    /// Mirror sub-object frame (object space), moved / rotated with W/E.
    #[serde(default)]
    pub mirror: Mat4,
}

impl Default for SymmetryMod {
    fn default() -> Self {
        Self { axis: 0, flip: false, slice: true, weld: true, threshold: 0.01, offset: 0.0, mirror: Mat4::IDENTITY }
    }
}

impl SymmetryMod {
    /// Kept side's normal of the mirror plane (object space).
    pub fn normal(&self) -> Vec3 {
        let mut n = Vec3::ZERO;
        n[self.axis.min(2)] = if self.flip { -1.0 } else { 1.0 };
        self.mirror.transform_vector3(n).normalize_or(n)
    }

    /// A point on the mirror plane (object space).
    pub fn point(&self) -> Vec3 {
        let mut p = Vec3::ZERO;
        p[self.axis.min(2)] = self.offset;
        self.mirror.transform_point3(p)
    }

    pub fn apply(&self, m: &mut EditablePoly) {
        let (n, point) = (self.normal(), self.point());
        let side = |m: &EditablePoly, p: PolyId| (m.poly_centroid(p) - point).dot(n);
        if self.slice {
            let sel = m.selection_sets();
            m.clear_selection();
            slice::slice(m, point, n, false, false);
            m.apply_selection_sets(&sel);
            let drop: Vec<PolyId> = m.polygons.keys().filter(|&p| side(m, p) < 0.0).collect();
            for p in drop {
                m.polygons.remove(p);
            }
            m.remove_isolated_vertices();
            m.finalize();
            // snap seam vertices exactly onto the plane
            let eps = self.threshold.max(1e-5);
            for v in m.vertices.values_mut() {
                let d = (v.position - point).dot(n);
                if d.abs() <= eps {
                    v.position -= n * d;
                }
            }
        }
        // reflection across the plane: p - 2 ((p - point) . n) n
        let reflect = Mat4::from_mat3(Mat3::IDENTITY - 2.0 * Mat3::from_cols(n * n.x, n * n.y, n * n.z));
        let mirror = Mat4::from_translation(point) * reflect * Mat4::from_translation(-point);
        let copy = m.clone();
        m.append(&copy, mirror);
        if self.weld {
            for v in m.vertices.values_mut() {
                v.selected = (v.position - point).dot(n).abs() <= self.threshold;
            }
            weld::weld(m, SubObject::Vertex, self.threshold);
            m.vertices.values_mut().for_each(|v| v.selected = false);
        }
        m.finalize();
    }
}
