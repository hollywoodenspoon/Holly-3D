//! Boolean (ProBoolean-style) modifier: a stack of picked operands combined
//! with the input mesh via manifold CSG (`boolmesh`). UVs, material IDs and
//! smoothing groups ride along as per-corner values.

use crate::mesh::triangulate::triangulate_polygon;
use crate::mesh::{EditablePoly, FaceCorner, Polygon, VertexId};
use bevy::math::{Vec2, Vec3};
use boolmesh::prelude::{Manifold, OpType, compute_boolean};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BoolOp {
    #[default]
    Union,
    SubtractAB,
    SubtractBA,
    Intersect,
}

impl BoolOp {
    pub const ALL: [BoolOp; 4] = [BoolOp::Union, BoolOp::SubtractAB, BoolOp::SubtractBA, BoolOp::Intersect];

    pub fn label(self) -> &'static str {
        match self {
            BoolOp::Union => "Union",
            BoolOp::SubtractAB => "Subtract (A-B)",
            BoolOp::SubtractBA => "Subtract (B-A)",
            BoolOp::Intersect => "Intersect",
        }
    }
}

/// One picked operand, stored in the owning object's mesh space.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BoolOperand {
    pub name: String,
    pub op: BoolOp,
    pub enabled: bool,
    pub mesh: EditablePoly,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BooleanMod {
    /// Operation given to the next picked operand.
    pub op: BoolOp,
    pub operands: Vec<BoolOperand>,
    /// Highlighted row of the operand list.
    pub selected: Option<usize>,
    /// Rebuild n-gons from the flat triangle regions of the result.
    pub merge_coplanar: bool,
}

impl Default for BooleanMod {
    fn default() -> Self {
        Self { op: BoolOp::Union, operands: vec![], selected: None, merge_coplanar: true }
    }
}

type Corner = boolmesh::Vec4;

/// Triangulated manifold with (u, v, material, smoothing) on every corner.
pub fn to_manifold(m: &EditablePoly) -> Result<Manifold<Corner>, String> {
    let index: HashMap<VertexId, usize> = m.vertices.keys().enumerate().map(|(i, v)| (v, i)).collect();
    let pos: Vec<f64> = m.vertices.values().flat_map(|v| v.position.to_array().map(f64::from)).collect();
    let (mut idx, mut vals) = (vec![], vec![]);
    for p in m.polygons.values() {
        let pts: Vec<Vec3> = p.vertex_ids().map(|v| m.pos(v)).collect();
        for tri in triangulate_polygon(&pts) {
            for k in tri {
                let c = &p.corners[k];
                idx.push(index[&c.vertex_id]);
                vals.push(Corner::new(c.uv.x as f64, c.uv.y as f64, p.material_id as f64, p.smoothing_group as f64));
            }
        }
    }
    if idx.is_empty() {
        return Err("empty mesh".into());
    }
    Manifold::new_with_values(&pos, &idx, &vals)
}

fn from_manifold(r: &Manifold<Corner>, merge: bool) -> EditablePoly {
    let mut out = EditablePoly::new();
    let vs: Vec<VertexId> = r.ps.iter().map(|p| out.add_vertex(Vec3::new(p.x as f32, p.y as f32, p.z as f32))).collect();
    for (t, h) in r.hs.chunks(3).enumerate() {
        if h.len() < 3 || h.iter().any(|h| h.tail >= vs.len()) {
            continue;
        }
        let val = |k: usize| r.vals.get(3 * t + k).copied().unwrap_or_default();
        let corners: Vec<FaceCorner> =
            (0..3).map(|k| FaceCorner::new(vs[h[k].tail], Vec2::new(val(k).x as f32, val(k).y as f32))).collect();
        let mut template = Polygon::new(vec![]);
        template.material_id = (val(0).z.round() as u16).max(1);
        template.smoothing_group = val(0).w.round().clamp(0.0, u32::MAX as f64) as u32;
        out.add_polygon_corners(corners, &template);
    }
    out.remove_isolated_vertices();
    out.finalize();
    if merge {
        out.merge_coplanar(0.99999);
    }
    out
}

/// `a <op> b`. Empty operands follow the set algebra (A ∪ ∅ = A, A ∩ ∅ = ∅…).
pub fn compute(a: &EditablePoly, b: &EditablePoly, op: BoolOp, merge: bool) -> Result<EditablePoly, String> {
    match (a.polygons.is_empty(), b.polygons.is_empty(), op) {
        (_, true, BoolOp::Union | BoolOp::SubtractAB) => return Ok(a.clone()),
        (true, _, BoolOp::Union | BoolOp::SubtractBA) => return Ok(b.clone()),
        (true, _, _) | (_, true, _) => return Ok(EditablePoly::new()),
        _ => {}
    }
    let (p, q) = if op == BoolOp::SubtractBA { (b, a) } else { (a, b) };
    let kind = match op {
        BoolOp::Union => OpType::Add,
        BoolOp::SubtractAB | BoolOp::SubtractBA => OpType::Subtract,
        BoolOp::Intersect => OpType::Intersect,
    };
    let (mp, mq) = (to_manifold(p)?, to_manifold(q)?);
    let r = compute_boolean(&mp, &mq, kind)?;
    Ok(from_manifold(&r, merge))
}

impl BooleanMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        for o in self.operands.iter().filter(|o| o.enabled) {
            // a failed operand (not closed / not manifold) is skipped, like ProBoolean
            if let Ok(r) = compute(m, &o.mesh, o.op, self.merge_coplanar) {
                *m = r;
            }
        }
    }

    pub fn add_operand(&mut self, name: String, mesh: EditablePoly) {
        self.operands.push(BoolOperand { name, op: self.op, enabled: true, mesh });
        self.selected = Some(self.operands.len() - 1);
    }

    /// Removes and returns the highlighted operand.
    pub fn take_selected(&mut self) -> Option<BoolOperand> {
        let i = self.selected.filter(|&i| i < self.operands.len())?;
        self.selected = None;
        Some(self.operands.remove(i))
    }
}
