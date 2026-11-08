//! Connect for edges (Segments / Pinch / Slide) and vertices.

use super::*;
use std::collections::HashSet;

/// Inserts the given interior points (ordered from key.0 to key.1) into every
/// polygon side that runs along each keyed edge.
pub(crate) fn insert_edge_points(m: &mut EditablePoly, pts: &HashMap<(VertexId, VertexId), Vec<(VertexId, f32)>>) {
    for p in m.polygons.values_mut() {
        let n = p.corners.len();
        let mut out = Vec::with_capacity(n + 4);
        for i in 0..n {
            let (cx, cy) = (&p.corners[i], &p.corners[(i + 1) % n]);
            out.push(cx.clone());
            let key = edge_key(cx.vertex_id, cy.vertex_id);
            if let Some(list) = pts.get(&key) {
                let forward = key.0 == cx.vertex_id;
                let iter: Box<dyn Iterator<Item = &(VertexId, f32)>> =
                    if forward { Box::new(list.iter()) } else { Box::new(list.iter().rev()) };
                for &(v, t) in iter {
                    let lt = if forward { t } else { 1.0 - t };
                    out.push(FaceCorner::new(v, cx.uv.lerp(cy.uv, lt)));
                }
            }
        }
        p.corners = out;
    }
}

/// Connects selected edges with `segments` new edge loops through the polygons
/// that contain exactly two of them.
pub fn connect_edges(m: &mut EditablePoly, segments: u32, pinch: f32, slide: f32) {
    let sel: Vec<(VertexId, VertexId)> = m.selected_edges().iter().map(|&e| edge_key(m.edges[e].v0, m.edges[e].v1)).collect();
    let sset: HashSet<_> = sel.iter().copied().collect();
    let n = segments.max(1) as usize;
    // polygons with exactly two selected edges, recorded before insertion
    type HalfEdge = (VertexId, VertexId);
    let mut targets: Vec<(PolyId, HalfEdge, HalfEdge)> = vec![];
    for (pid, p) in &m.polygons {
        let hits: Vec<_> = p.half_edges().filter(|&(a, b)| sset.contains(&edge_key(a, b))).collect();
        if hits.len() == 2 {
            targets.push((pid, hits[0], hits[1]));
        }
    }
    let mut pts: HashMap<(VertexId, VertexId), Vec<(VertexId, f32)>> = HashMap::new();
    let spread = (1.0 - pinch / 100.0).clamp(0.0, 2.0);
    for &(a, b) in &sel {
        let list = (1..=n)
            .map(|i| {
                let t = i as f32 / (n + 1) as f32;
                let t = (0.5 + (t - 0.5) * spread + slide / 200.0).clamp(0.02, 0.98);
                let v = m.add_vertex(m.pos(a).lerp(m.pos(b), t));
                (v, t)
            })
            .collect::<Vec<_>>();
        let mut list = list;
        list.sort_by(|x, y| x.1.total_cmp(&y.1));
        pts.insert((a, b), list);
    }
    insert_edge_points(m, &pts);
    let mut new_edges = vec![];
    for (pid, e1, e2) in targets {
        let along = |(a, b): (VertexId, VertexId)| -> Vec<VertexId> {
            let list = &pts[&edge_key(a, b)];
            if edge_key(a, b).0 == a { list.iter().map(|x| x.0).collect() } else { list.iter().rev().map(|x| x.0).collect() }
        };
        let p = along(e1);
        let q = along(e2);
        let mut cur = pid;
        for i in 0..n {
            let (x, y) = (p[i], q[n - 1 - i]);
            let Some((r1, r2)) = split_polygon(m, cur, x, y) else {
                break;
            };
            new_edges.push((x, y));
            if i + 1 < n {
                cur = if m.polygons[r1].position_of(p[i + 1]).is_some() { r1 } else { r2 };
            }
        }
    }
    m.finalize();
    m.clear_selection();
    for (a, b) in new_edges {
        if let Some(e) = m.edge_between(a, b) {
            m.edges[e].selected = true;
        }
    }
}

/// Connects selected vertices that share a polygon with new edges.
pub fn connect_vertices(m: &mut EditablePoly) {
    let sel: HashSet<VertexId> = m.selected_vertices().into_iter().collect();
    let pids: Vec<PolyId> = m.polygons.keys().collect();
    let mut new_edges = vec![];
    for pid in pids {
        let vs: Vec<VertexId> = m.polygons[pid].vertex_ids().filter(|v| sel.contains(v)).collect();
        if vs.len() < 2 {
            continue;
        }
        let pairs: Vec<(VertexId, VertexId)> =
            if vs.len() == 2 { vec![(vs[0], vs[1])] } else { (0..vs.len()).map(|i| (vs[i], vs[(i + 1) % vs.len()])).collect() };
        let mut pieces = vec![pid];
        for (a, b) in pairs {
            let Some(&holder) = pieces.iter().find(|&&p| m.polygons[p].position_of(a).is_some() && m.polygons[p].position_of(b).is_some())
            else {
                continue;
            };
            if let Some((_, q)) = split_polygon(m, holder, a, b) {
                pieces.push(q);
                new_edges.push((a, b));
            }
        }
    }
    m.finalize();
    for (a, b) in new_edges {
        if let Some(e) = m.edge_between(a, b) {
            m.edges[e].selected = true;
        }
    }
}
