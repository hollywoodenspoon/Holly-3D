//! Chamfer modifier: non-destructive edge bevel over a filtered edge set.

use crate::mesh::adjacency::Adjacency;
use crate::mesh::ops::chamfer::chamfer_edges_ex;
use crate::mesh::{EdgeId, EditablePoly};

#[derive(Clone, Copy, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum ChamferFilter {
    #[default]
    All,
    Selected,
    SmoothingBoundaries,
    /// Dihedral angle between the two faces, degrees.
    Angle {
        min: f32,
        max: f32,
    },
}

impl ChamferFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All Edges",
            Self::Selected => "Selected Edges",
            Self::SmoothingBoundaries => "Smoothing Group Boundaries",
            Self::Angle { .. } => "By Angle Threshold",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChamferMod {
    pub amount: f32,
    pub segments: u32,
    pub tension: f32,
    pub filter: ChamferFilter,
}

impl Default for ChamferMod {
    fn default() -> Self {
        Self { amount: 0.05, segments: 1, tension: 0.5, filter: ChamferFilter::All }
    }
}

impl ChamferMod {
    /// Edges the filter picks (manifold edges only).
    pub fn edges(&self, m: &EditablePoly) -> Vec<EdgeId> {
        let adj = Adjacency::build(m);
        m.edges
            .iter()
            .filter(|(_, e)| {
                let fs = adj.faces_of_edge(e.v0, e.v1);
                if fs.len() != 2 {
                    return false;
                }
                let (p, q) = (fs[0], fs[1]);
                match self.filter {
                    ChamferFilter::All => true,
                    ChamferFilter::Selected => e.selected,
                    ChamferFilter::SmoothingBoundaries => m.polygons[p].smoothing_group & m.polygons[q].smoothing_group == 0,
                    ChamferFilter::Angle { min, max } => {
                        let a = m.poly_normal(p).dot(m.poly_normal(q)).clamp(-1.0, 1.0).acos().to_degrees();
                        a >= min && a <= max
                    }
                }
            })
            .map(|(id, _)| id)
            .collect()
    }

    pub fn apply(&self, m: &mut EditablePoly) {
        let picked = self.edges(m);
        if picked.is_empty() || self.amount <= 0.0 {
            return;
        }
        let polys: Vec<_> = m.polygons.iter().filter(|(_, p)| p.selected).map(|(k, _)| k).collect();
        m.edges.values_mut().for_each(|e| e.selected = false);
        for e in picked {
            m.edges[e].selected = true;
        }
        chamfer_edges_ex(m, self.amount, self.segments.clamp(1, 16), self.tension.clamp(0.0, 1.0), false);
        // Keep the incoming polygon selection flowing up the stack.
        m.polygons.values_mut().for_each(|p| p.selected = false);
        for p in polys {
            if let Some(p) = m.polygons.get_mut(p) {
                p.selected = true;
            }
        }
        m.edges.values_mut().for_each(|e| e.selected = false);
    }
}
