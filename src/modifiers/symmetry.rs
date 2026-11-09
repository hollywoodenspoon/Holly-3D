//! Symmetry: mirrors the mesh across a local plane (X, Y or Z), optionally
//! slicing along the mirror plane first and welding the seam.

use crate::mesh::ops::{slice, weld};
use crate::mesh::{EditablePoly, PolyId, SubObject};
use bevy::math::{Mat4, Vec3};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SymmetryMod {
    pub axis: usize,
    /// Keep the negative side instead of the positive one.
    pub flip: bool,
    pub slice: bool,
    pub weld: bool,
    pub threshold: f32,
    /// Mirror plane offset along the axis (the Mirror sub-object).
    pub offset: f32,
}

impl Default for SymmetryMod {
    fn default() -> Self {
        Self { axis: 0, flip: false, slice: true, weld: true, threshold: 0.01, offset: 0.0 }
    }
}

impl SymmetryMod {
    pub fn normal(&self) -> Vec3 {
        let mut n = Vec3::ZERO;
        n[self.axis.min(2)] = if self.flip { -1.0 } else { 1.0 };
        n
    }

    pub fn apply(&self, m: &mut EditablePoly) {
        let n = self.normal();
        let mut point = Vec3::ZERO;
        point[self.axis.min(2)] = self.offset;
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
        let a = self.axis.min(2);
        let mut s = Vec3::ONE;
        s[a] = -1.0;
        let mirror = Mat4::from_translation(point) * Mat4::from_scale(s) * Mat4::from_translation(-point);
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
