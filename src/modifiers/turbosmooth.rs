//! TurboSmooth: Catmull-Clark subdivision on N-gons with open borders.
//! Every n-gon becomes n quads per iteration. Borders, creased edges
//! (creasing >= 1) and, optionally, smoothing-group / material boundaries
//! are kept sharp (cubic B-spline curve rules).

use crate::mesh::{EditablePoly, FaceCorner, PolyId, Polygon, VertexId, edge_key};
use bevy::math::{Vec2, Vec3};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TurboSmoothMod {
    pub iterations: u32,
    /// Wireframe overlay shows only the original cage edges.
    pub isoline: bool,
    pub separate_smoothing: bool,
    pub separate_materials: bool,
}

impl Default for TurboSmoothMod {
    fn default() -> Self {
        Self { iterations: 1, isoline: true, separate_smoothing: false, separate_materials: false }
    }
}

impl TurboSmoothMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        for _ in 0..self.iterations.min(4) {
            *m = subdivide(m, self.separate_smoothing, self.separate_materials, self.isoline);
        }
    }
}

type Key = (VertexId, VertexId);

/// One Catmull-Clark step. With `isoline`, edges inside the original faces
/// are hidden so the wireframe shows only the (smoothed) cage.
pub fn subdivide(m: &EditablePoly, by_smoothing: bool, by_material: bool, isoline: bool) -> EditablePoly {
    let mut faces_of: HashMap<Key, Vec<PolyId>> = HashMap::new();
    for (pid, p) in &m.polygons {
        for (a, b) in p.half_edges() {
            faces_of.entry(edge_key(a, b)).or_default().push(pid);
        }
    }
    let hidden_in: std::collections::HashSet<Key> = m.edges.values().filter(|e| e.hidden).map(|e| edge_key(e.v0, e.v1)).collect();
    let mut face_verts = std::collections::HashSet::new();
    let mut hide: Vec<(VertexId, VertexId)> = vec![];
    let crease: HashMap<Key, f32> = m.edges.values().map(|e| (edge_key(e.v0, e.v1), e.creasing)).collect();
    let sharp = |k: &Key, fs: &[PolyId]| {
        if fs.len() != 2 || crease.get(k).copied().unwrap_or(0.0) >= 1.0 {
            return true;
        }
        let (p, q) = (&m.polygons[fs[0]], &m.polygons[fs[1]]);
        (by_smoothing && p.smoothing_group & q.smoothing_group == 0) || (by_material && p.material_id != q.material_id)
    };
    let face_pt: HashMap<PolyId, Vec3> = m.polygons.keys().map(|p| (p, m.poly_centroid(p))).collect();
    let mut edge_pt: HashMap<Key, Vec3> = HashMap::new();
    let mut is_sharp: HashMap<Key, bool> = HashMap::new();
    for (k, fs) in &faces_of {
        let mid = (m.pos(k.0) + m.pos(k.1)) * 0.5;
        let s = sharp(k, fs);
        is_sharp.insert(*k, s);
        edge_pt.insert(*k, if s { mid } else { (mid * 2.0 + face_pt[&fs[0]] + face_pt[&fs[1]]) * 0.25 });
    }
    // Vertex rules: smooth (Q + 2R + (n-3)P)/n, crease (a + 6P + b)/8, corner fixed.
    let mut q_sum: HashMap<VertexId, (Vec3, f32)> = HashMap::new();
    for (pid, p) in &m.polygons {
        for v in p.vertex_ids() {
            let e = q_sum.entry(v).or_default();
            e.0 += face_pt[&pid];
            e.1 += 1.0;
        }
    }
    let mut r_sum: HashMap<VertexId, (Vec3, f32)> = HashMap::new();
    let mut sharp_nb: HashMap<VertexId, Vec<VertexId>> = HashMap::new();
    for (k, s) in &is_sharp {
        let mid = (m.pos(k.0) + m.pos(k.1)) * 0.5;
        for (v, o) in [(k.0, k.1), (k.1, k.0)] {
            let e = r_sum.entry(v).or_default();
            e.0 += mid;
            e.1 += 1.0;
            if *s {
                sharp_nb.entry(v).or_default().push(o);
            }
        }
    }
    let mut out = EditablePoly::new();
    let mut vmap: HashMap<VertexId, VertexId> = HashMap::new();
    for (id, v) in &m.vertices {
        let p = v.position;
        let np = match sharp_nb.get(&id).map(|s| s.len()).unwrap_or(0) {
            0 | 1 => match (q_sum.get(&id), r_sum.get(&id)) {
                (Some(&(q, nq)), Some(&(r, nr))) if nr >= 3.0 => {
                    let n = nr;
                    (q / nq + r / nr * 2.0 + p * (n - 3.0)) / n
                }
                _ => p,
            },
            // a border vertex on a single face is a corner: it stays put
            2 if q_sum.get(&id).is_some_and(|q| q.1 > 1.0) => {
                let s = &sharp_nb[&id];
                (m.pos(s[0]) + p * 6.0 + m.pos(s[1])) / 8.0
            }
            _ => p,
        };
        let mut nv = v.clone();
        nv.position = np;
        vmap.insert(id, out.vertices.insert(nv));
    }
    let mut emap: HashMap<Key, VertexId> = HashMap::new();
    for (k, p) in &edge_pt {
        let mut nv = m.vertices[k.0].clone();
        nv.position = *p;
        nv.color = (m.vertices[k.0].color + m.vertices[k.1].color) * 0.5;
        nv.selected = false;
        emap.insert(*k, out.vertices.insert(nv));
    }
    let mut crease_out: Vec<(VertexId, VertexId, f32)> = vec![];
    for (pid, p) in &m.polygons {
        let n = p.corners.len();
        let mut fv = m.vertices[p.corners[0].vertex_id].clone();
        fv.position = face_pt[&pid];
        fv.selected = false;
        let f = out.vertices.insert(fv);
        face_verts.insert(f);
        let fuv = p.corners.iter().map(|c| c.uv).sum::<Vec2>() / n as f32;
        for i in 0..n {
            let (prev, cur, next) = (&p.corners[(i + n - 1) % n], &p.corners[i], &p.corners[(i + 1) % n]);
            let (e_next, e_prev) = (edge_key(cur.vertex_id, next.vertex_id), edge_key(prev.vertex_id, cur.vertex_id));
            let corners = vec![
                FaceCorner::new(vmap[&cur.vertex_id], cur.uv),
                FaceCorner::new(emap[&e_next], (cur.uv + next.uv) * 0.5),
                FaceCorner::new(f, fuv),
                FaceCorner::new(emap[&e_prev], (prev.uv + cur.uv) * 0.5),
            ];
            out.polygons.insert(Polygon { corners, ..p.clone() });
            for k in [e_next, e_prev] {
                if hidden_in.contains(&k) {
                    hide.push((vmap[&cur.vertex_id], emap[&k]));
                }
            }
            for (k, a) in [(e_next, cur.vertex_id), (e_prev, cur.vertex_id)] {
                if let Some(&c) = crease.get(&k).filter(|c| **c > 0.0) {
                    crease_out.push((vmap[&a], emap[&k], c));
                }
            }
        }
    }
    out.finalize();
    // creases carry over to both halves of a split edge
    for (a, b, c) in crease_out {
        if let Some(e) = out.edge_between(a, b) {
            out.edges[e].creasing = c;
        }
    }
    if isoline {
        for e in out.edges.values_mut() {
            e.hidden = face_verts.contains(&e.v0) || face_verts.contains(&e.v1);
        }
        for (a, b) in hide {
            if let Some(e) = out.edge_between(a, b) {
                out.edges[e].hidden = true;
            }
        }
    }
    out
}
