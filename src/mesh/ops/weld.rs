//! Weld (threshold), Target Weld and Collapse.

use super::*;
use crate::mesh::adjacency::Adjacency;
use std::collections::HashSet;

fn find(parent: &mut HashMap<VertexId, VertexId>, v: VertexId) -> VertexId {
    let p = parent[&v];
    if p == v {
        return v;
    }
    let r = find(parent, p);
    parent.insert(v, r);
    r
}

/// Replaces every vertex in `map` by its target in all polygons, deletes the
/// merged vertices and cleans up degenerate polygons.
pub fn merge_vertices(m: &mut EditablePoly, map: &HashMap<VertexId, VertexId>) {
    for p in m.polygons.values_mut() {
        for c in &mut p.corners {
            if let Some(&t) = map.get(&c.vertex_id) {
                c.vertex_id = t;
            }
        }
    }
    for (&v, &t) in map {
        if v != t {
            m.vertices.remove(v);
        }
    }
    m.finalize();
}

/// Merges groups of vertices into their centroids (one representative each).
fn collapse_groups(m: &mut EditablePoly, groups: Vec<Vec<VertexId>>) {
    let mut map = HashMap::new();
    for g in groups {
        if g.len() < 2 {
            continue;
        }
        let c = g.iter().map(|&v| m.pos(v)).sum::<Vec3>() / g.len() as f32;
        let keep = g[0];
        m.vertices[keep].position = c;
        m.vertices[keep].selected = true;
        for &v in &g[1..] {
            map.insert(v, keep);
        }
    }
    merge_vertices(m, &map);
}

/// Welds selected vertices (or the vertices of selected edges) closer than `threshold`.
pub fn weld(m: &mut EditablePoly, level: SubObject, threshold: f32) {
    let verts = m.selection_vertices(level);
    let mut parent: HashMap<VertexId, VertexId> = verts.iter().map(|&v| (v, v)).collect();
    for i in 0..verts.len() {
        for j in i + 1..verts.len() {
            if m.pos(verts[i]).distance(m.pos(verts[j])) <= threshold {
                let (a, b) = (find(&mut parent, verts[i]), find(&mut parent, verts[j]));
                if a != b {
                    parent.insert(b, a);
                }
            }
        }
    }
    let mut groups: HashMap<VertexId, Vec<VertexId>> = HashMap::new();
    for &v in &verts {
        let r = find(&mut parent, v);
        groups.entry(r).or_default().push(v);
    }
    collapse_groups(m, groups.into_values().collect());
}

/// Target Weld: snaps vertex `a` onto `b` and merges it.
pub fn target_weld(m: &mut EditablePoly, a: VertexId, b: VertexId) -> bool {
    if a == b || !m.vertices.contains_key(a) || !m.vertices.contains_key(b) {
        return false;
    }
    merge_vertices(m, &HashMap::from([(a, b)]));
    true
}

/// Collapse: each contiguous selection island is welded to its center.
pub fn collapse(m: &mut EditablePoly, level: SubObject) {
    let adj = Adjacency::build(m);
    let verts: HashSet<VertexId> = m.selection_vertices(level).into_iter().collect();
    let mut parent: HashMap<VertexId, VertexId> = verts.iter().map(|&v| (v, v)).collect();
    let link = |a: VertexId, b: VertexId, parent: &mut HashMap<VertexId, VertexId>| {
        let (ra, rb) = (find(parent, a), find(parent, b));
        if ra != rb {
            parent.insert(rb, ra);
        }
    };
    match level {
        SubObject::Polygon | SubObject::Element => {
            for p in m.selected_polygons() {
                let vs: Vec<_> = m.polygons[p].vertex_ids().collect();
                for w in vs.windows(2) {
                    link(w[0], w[1], &mut parent);
                }
            }
        }
        SubObject::Edge | SubObject::Border => {
            for e in m.selected_edges() {
                link(m.edges[e].v0, m.edges[e].v1, &mut parent);
            }
        }
        _ => {
            for &v in &verts {
                for &n in adj.neighbors(v) {
                    if verts.contains(&n) {
                        link(v, n, &mut parent);
                    }
                }
            }
        }
    }
    let mut groups: HashMap<VertexId, Vec<VertexId>> = HashMap::new();
    let mut sorted: Vec<_> = verts.into_iter().collect();
    sorted.sort();
    for v in sorted {
        let r = find(&mut parent, v);
        groups.entry(r).or_default().push(v);
    }
    collapse_groups(m, groups.into_values().collect());
}
