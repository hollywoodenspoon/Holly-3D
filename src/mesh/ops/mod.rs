//! Edit Poly operations. Every op reads the selection flags of the mesh, edits
//! vertices/polygons, then calls `finalize()` so the edge table stays in sync.

pub mod cap_bridge;
pub mod chamfer;
pub mod connect;
pub mod dispatch;
pub mod extrude;
pub mod geometry;
pub mod insert;
pub mod inset;
pub mod remove;
pub mod slice;
pub mod sweep;
pub mod weld;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_more;
#[cfg(test)]
mod tests_sweep;

pub use dispatch::{ExtrudeMode, PolyOp};

use super::*;

/// Splits edge (a, b) at parameter `t` (0 = a, 1 = b), inserting the new vertex
/// into every polygon that uses the edge. Edge table is NOT rebuilt.
pub fn split_edge(m: &mut EditablePoly, a: VertexId, b: VertexId, t: f32) -> VertexId {
    let pa = m.pos(a);
    let pb = m.pos(b);
    let ca = m.vertices[a].color;
    let cb = m.vertices[b].color;
    let nv = m.add_vertex(pa.lerp(pb, t));
    m.vertices[nv].color = ca.lerp(cb, t);
    for p in m.polygons.values_mut() {
        let n = p.corners.len();
        let mut insert_at = None;
        for i in 0..n {
            let (x, y) = (p.corners[i].vertex_id, p.corners[(i + 1) % n].vertex_id);
            if (x == a && y == b) || (x == b && y == a) {
                let (ua, ub) =
                    if x == a { (p.corners[i].uv, p.corners[(i + 1) % n].uv) } else { (p.corners[(i + 1) % n].uv, p.corners[i].uv) };
                insert_at = Some((i + 1, ua.lerp(ub, t)));
                break;
            }
        }
        if let Some((at, uv)) = insert_at {
            p.corners.insert(at, FaceCorner::new(nv, uv));
        }
    }
    nv
}

/// Splits polygon `pid` along a new edge between two of its (non-adjacent)
/// vertices. Returns the two resulting polygons.
pub fn split_polygon(m: &mut EditablePoly, pid: PolyId, va: VertexId, vb: VertexId) -> Option<(PolyId, PolyId)> {
    let p = m.polygons.get(pid)?.clone();
    let n = p.corners.len();
    let ia = p.position_of(va)?;
    let ib = p.position_of(vb)?;
    if ia == ib || (ia + 1) % n == ib || (ib + 1) % n == ia {
        return None;
    }
    let walk = |from: usize, to: usize| -> Vec<FaceCorner> {
        let mut out = vec![];
        let mut i = from;
        loop {
            out.push(p.corners[i].clone());
            if i == to {
                break;
            }
            i = (i + 1) % n;
        }
        out
    };
    let first = walk(ia, ib);
    let second = walk(ib, ia);
    m.polygons[pid].corners = first;
    let selected = p.selected;
    let q = m.add_polygon_corners(second, &p);
    m.polygons[q].selected = selected;
    Some((pid, q))
}

/// Unit vector from `v` towards `to`.
#[allow(dead_code)]
pub(crate) fn dir(m: &EditablePoly, v: VertexId, to: VertexId) -> Vec3 {
    (m.pos(to) - m.pos(v)).normalize_or_zero()
}

/// Inward in-plane offset for each corner of polygon `pid` such that moving the
/// corner by `offset * amount` shifts both adjacent edges inward by `amount`.
pub(crate) fn corner_inset_offsets(m: &EditablePoly, pid: PolyId) -> Vec<Vec3> {
    let p = &m.polygons[pid];
    let nrm = m.poly_normal(pid);
    let n = p.corners.len();
    (0..n)
        .map(|i| {
            let prev = m.pos(p.corners[(i + n - 1) % n].vertex_id);
            let cur = m.pos(p.corners[i].vertex_id);
            let next = m.pos(p.corners[(i + 1) % n].vertex_id);
            let e1 = (cur - prev).normalize_or_zero();
            let e2 = (next - cur).normalize_or_zero();
            let n1 = nrm.cross(e1).normalize_or_zero();
            let n2 = nrm.cross(e2).normalize_or_zero();
            miter(n1, n2)
        })
        .collect()
}

/// Miter direction for two unit inward edge normals (length grows at sharp corners).
pub(crate) fn miter(n1: Vec3, n2: Vec3) -> Vec3 {
    let d = (n1 + n2).normalize_or(n1);
    let c = d.dot(n1).max(0.25);
    d / c
}
