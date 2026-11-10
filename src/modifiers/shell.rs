//! Shell: gives a surface thickness. The outer layer is pushed out along
//! the vertex normals by Outer Amount, the inner layer (reversed) pushed
//! in by Inner Amount, and open borders are bridged by `segments` rows of
//! edge faces.

use crate::mesh::adjacency::Adjacency;
use crate::mesh::{EditablePoly, FaceCorner, Polygon, VertexId};
use bevy::math::{Vec2, Vec3};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShellMod {
    pub inner: f32,
    pub outer: f32,
    pub segments: u32,
    /// Keeps the wall thickness even at corners (offset / cos of the face angle).
    pub straighten: bool,
    pub override_edge_mat: Option<u16>,
    pub select_inner: bool,
    pub select_outer: bool,
    pub select_edges: bool,
}

impl Default for ShellMod {
    fn default() -> Self {
        Self {
            inner: 0.05,
            outer: 0.0,
            segments: 1,
            straighten: true,
            override_edge_mat: None,
            select_inner: false,
            select_outer: false,
            select_edges: false,
        }
    }
}

impl ShellMod {
    /// Per-vertex offset direction, scaled for even thickness if Straighten Corners.
    fn directions(&self, m: &EditablePoly) -> HashMap<VertexId, Vec3> {
        let normals = m.vertex_normals();
        if !self.straighten {
            return normals;
        }
        let mut min_dot: HashMap<VertexId, f32> = HashMap::new();
        for pid in m.polygons.keys() {
            let fnrm = m.poly_normal(pid);
            for v in m.polygons[pid].vertex_ids() {
                let d = normals[&v].dot(fnrm);
                let e = min_dot.entry(v).or_insert(1.0);
                *e = e.min(d);
            }
        }
        normals.into_iter().map(|(v, n)| (v, n / min_dot.get(&v).copied().unwrap_or(1.0).clamp(0.5, 1.0))).collect()
    }

    pub fn apply(&self, m: &mut EditablePoly) {
        if m.polygons.is_empty() || (self.inner == 0.0 && self.outer == 0.0) {
            return;
        }
        let segs = self.segments.clamp(1, 32) as usize;
        let dirs = self.directions(m);
        let adj = Adjacency::build(m);
        let border = adj.open_half_edges();
        let base = m.clone();
        let mut out = EditablePoly::new();
        // rings[k][v]: ring 0 is the outer layer, ring `segs` the inner one;
        // only border vertices need the in-between rings.
        let mut rings: Vec<HashMap<VertexId, VertexId>> = vec![HashMap::new(); segs + 1];
        let total = self.outer + self.inner;
        for (k, ring) in rings.iter_mut().enumerate() {
            let off = self.outer - total * k as f32 / segs as f32;
            for (id, v) in &base.vertices {
                let is_layer = k == 0 || k == segs;
                if !is_layer && !adj.is_border_vertex(id) {
                    continue;
                }
                let mut nv = v.clone();
                nv.position += dirs.get(&id).copied().unwrap_or(Vec3::Z) * off;
                ring.insert(id, out.vertices.insert(nv));
            }
        }
        for p in base.polygons.values() {
            let mut outer = p.clone();
            outer.corners.iter_mut().for_each(|c| c.vertex_id = rings[0][&c.vertex_id]);
            outer.selected = self.select_outer;
            out.polygons.insert(outer);
            let mut inner = p.clone();
            inner.corners.iter_mut().for_each(|c| c.vertex_id = rings[segs][&c.vertex_id]);
            inner.corners.reverse();
            inner.selected = self.select_inner;
            out.polygons.insert(inner);
        }
        // Edge faces: one strip per open half-edge a -> b.
        let template = base.polygons.values().next().cloned().unwrap_or_else(|| Polygon::new(vec![]));
        for (i, &(a, b)) in border.iter().enumerate() {
            let owner = &base.polygons[adj.half_edge[&(a, b)]];
            for k in 0..segs {
                let ids = [rings[k][&b], rings[k][&a], rings[k + 1][&a], rings[k + 1][&b]];
                let (u0, u1) = (i as f32 / border.len() as f32, (i + 1) as f32 / border.len() as f32);
                let (v0, v1) = (k as f32 / segs as f32, (k + 1) as f32 / segs as f32);
                let uvs = [Vec2::new(u1, v0), Vec2::new(u0, v0), Vec2::new(u0, v1), Vec2::new(u1, v1)];
                let mut poly = Polygon {
                    corners: ids.iter().zip(uvs).map(|(&v, uv)| FaceCorner::new(v, uv)).collect(),
                    selected: self.select_edges,
                    ..template.clone()
                };
                poly.smoothing_group = 0;
                poly.material_id = self.override_edge_mat.unwrap_or(owner.material_id);
                out.polygons.insert(poly);
            }
        }
        out.finalize();
        *m = out;
    }
}
