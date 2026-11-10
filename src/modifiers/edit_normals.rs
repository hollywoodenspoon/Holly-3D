//! Edit Normals: per-face-corner explicit normals layered on top of the
//! smoothing-group normals (Break / Unify / Reset / Make Explicit / Rotate).

use crate::mesh::{EditablePoly, PolyId, VertexId};
use bevy::math::{Quat, Vec3};
use std::collections::{HashMap, HashSet};

/// A face corner: polygon + corner index.
pub type CornerKey = (PolyId, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum NormalSelBy {
    #[default]
    Normal,
    Vertex,
    Edge,
    Face,
}

impl NormalSelBy {
    pub const ALL: [NormalSelBy; 4] = [NormalSelBy::Normal, NormalSelBy::Vertex, NormalSelBy::Edge, NormalSelBy::Face];

    pub fn label(self) -> &'static str {
        match self {
            NormalSelBy::Normal => "Normal",
            NormalSelBy::Vertex => "Vertex",
            NormalSelBy::Edge => "Edge",
            NormalSelBy::Face => "Face",
        }
    }
}

/// Viewport colour class of a normal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NormalKind {
    /// Faceted: follows its face only.
    Unspecified,
    /// Averaged over smoothing-group neighbours.
    Unified,
    Explicit,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditNormalsMod {
    pub explicit: HashMap<CornerKey, Vec3>,
    pub selected: HashSet<CornerKey>,
    pub display_length: f32,
    pub select_by: NormalSelBy,
}

impl Default for EditNormalsMod {
    fn default() -> Self {
        Self { explicit: HashMap::new(), selected: HashSet::new(), display_length: 0.1, select_by: NormalSelBy::Normal }
    }
}

/// Every corner of a mesh with its vertex.
pub fn corners(m: &EditablePoly) -> impl Iterator<Item = (CornerKey, VertexId)> + '_ {
    m.polygons.iter().flat_map(|(p, poly)| poly.corners.iter().enumerate().map(move |(i, c)| ((p, i), c.vertex_id)))
}

pub fn keys_at_vertex(m: &EditablePoly, v: VertexId) -> Vec<CornerKey> {
    corners(m).filter(|c| c.1 == v).map(|c| c.0).collect()
}

pub fn keys_of_face(m: &EditablePoly, p: PolyId) -> Vec<CornerKey> {
    m.polygons.get(p).map_or(vec![], |poly| (0..poly.corners.len()).map(|i| (p, i)).collect())
}

/// Corners at both ends of edge a-b on the faces that use the edge.
pub fn keys_at_edge(m: &EditablePoly, a: VertexId, b: VertexId) -> Vec<CornerKey> {
    let mut out = vec![];
    for (p, poly) in &m.polygons {
        if poly.half_edges().any(|(x, y)| (x == a && y == b) || (x == b && y == a)) {
            out.extend(poly.corners.iter().enumerate().filter(|(_, c)| c.vertex_id == a || c.vertex_id == b).map(|(i, _)| (p, i)));
        }
    }
    out
}

impl EditNormalsMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        for (&(p, i), n) in &self.explicit {
            if let Some(c) = m.polygons.get_mut(p).and_then(|poly| poly.corners.get_mut(i)) {
                c.explicit_normal = Some(n.normalize_or(Vec3::Z));
            }
        }
    }

    /// Current normal of every corner (after this modifier) and its colour class.
    pub fn current(&self, input: &EditablePoly) -> HashMap<CornerKey, (Vec3, NormalKind)> {
        let mut m = input.clone();
        self.apply(&mut m);
        let faces_at: HashMap<VertexId, usize> = corners(&m).fold(HashMap::new(), |mut acc, c| {
            *acc.entry(c.1).or_default() += 1;
            acc
        });
        let normals = m.corner_normals();
        let mut out = HashMap::new();
        for (p, poly) in &m.polygons {
            for (i, c) in poly.corners.iter().enumerate() {
                let kind = if self.explicit.contains_key(&(p, i)) {
                    NormalKind::Explicit
                } else if poly.smoothing_group != 0 && faces_at.get(&c.vertex_id).copied().unwrap_or(0) > 1 {
                    NormalKind::Unified
                } else {
                    NormalKind::Unspecified
                };
                out.insert((p, i), (normals[&p][i], kind));
            }
        }
        out
    }

    fn selected_in(&self, input: &EditablePoly) -> Vec<CornerKey> {
        let mut v: Vec<CornerKey> =
            self.selected.iter().copied().filter(|&(p, i)| input.polygons.get(p).is_some_and(|poly| i < poly.corners.len())).collect();
        v.sort();
        v
    }

    /// Break: every selected corner gets its own face's normal (hard shading).
    pub fn break_selected(&mut self, input: &EditablePoly) {
        for (p, i) in self.selected_in(input) {
            self.explicit.insert((p, i), input.poly_normal(p));
        }
    }

    /// Unify: selected corners that share a vertex get one averaged normal.
    pub fn unify_selected(&mut self, input: &EditablePoly) {
        let cur = self.current(input);
        let mut by_vertex: HashMap<VertexId, Vec<CornerKey>> = HashMap::new();
        for (p, i) in self.selected_in(input) {
            by_vertex.entry(input.polygons[p].corners[i].vertex_id).or_default().push((p, i));
        }
        for keys in by_vertex.values() {
            let avg = keys.iter().map(|k| cur[k].0).sum::<Vec3>().normalize_or(Vec3::Z);
            for &k in keys {
                self.explicit.insert(k, avg);
            }
        }
    }

    /// Reset: back to smoothing-group control (all corners when nothing is selected).
    pub fn reset_selected(&mut self) {
        if self.selected.is_empty() {
            self.explicit.clear();
        } else {
            self.explicit.retain(|k, _| !self.selected.contains(k));
        }
    }

    /// Make Explicit: freezes the current normals of the selection.
    pub fn make_explicit(&mut self, input: &EditablePoly) {
        let cur = self.current(input);
        for k in self.selected_in(input) {
            self.explicit.insert(k, cur[&k].0);
        }
    }

    /// Rotates the selected normals (E gizmo), making them explicit.
    pub fn rotate_selected(&mut self, input: &EditablePoly, q: Quat) {
        let cur = self.current(input);
        for k in self.selected_in(input) {
            self.explicit.insert(k, (q * cur[&k].0).normalize_or(Vec3::Z));
        }
    }
}
