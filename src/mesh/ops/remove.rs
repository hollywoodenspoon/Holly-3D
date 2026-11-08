//! Delete, Remove (vertex/edge), Break and Split.

use super::*;
use crate::mesh::adjacency::Adjacency;
use std::collections::HashSet;

/// Delete key: removes the selected sub-objects together with dependent polygons.
pub fn delete_selected(m: &mut EditablePoly, level: SubObject) {
    let doomed: Vec<PolyId> = match level {
        SubObject::Vertex => {
            let vs: HashSet<_> = m.selected_vertices().into_iter().collect();
            m.polygons.iter().filter(|(_, p)| p.vertex_ids().any(|v| vs.contains(&v))).map(|(k, _)| k).collect()
        }
        SubObject::Edge | SubObject::Border => {
            let es: HashSet<_> = m.selected_edges().iter().map(|&e| edge_key(m.edges[e].v0, m.edges[e].v1)).collect();
            m.polygons.iter().filter(|(_, p)| p.half_edges().any(|(a, b)| es.contains(&edge_key(a, b)))).map(|(k, _)| k).collect()
        }
        SubObject::Polygon | SubObject::Element => m.selected_polygons(),
        SubObject::Object => vec![],
    };
    for p in doomed {
        m.polygons.remove(p);
    }
    if level == SubObject::Vertex {
        for v in m.selected_vertices() {
            m.vertices.remove(v);
        }
    }
    m.remove_isolated_vertices();
    m.finalize();
}

/// Corners of `p` starting after vertex `v` and ending just before it.
fn ring_after(p: &Polygon, v: VertexId) -> Vec<FaceCorner> {
    let i = p.position_of(v).unwrap();
    let n = p.corners.len();
    (1..n).map(|k| p.corners[(i + k) % n].clone()).collect()
}

/// Removes a vertex by dissolving it out of its polygons (merging them).
pub fn remove_vertex(m: &mut EditablePoly, v: VertexId) {
    let adj = Adjacency::build(m);
    let fan = adj.ordered_fan(m, v);
    if fan.is_empty() {
        m.vertices.remove(v);
        return;
    }
    if adj.neighbors(v).len() == 2 || fan.len() == 1 {
        for &f in &fan {
            m.polygons[f].corners.retain(|c| c.vertex_id != v);
        }
    } else {
        let tmpl = m.polygons[fan[0]].clone();
        let mut union: Vec<FaceCorner> = ring_after(&m.polygons[fan[0]], v);
        for &f in &fan[1..] {
            let seq = ring_after(&m.polygons[f], v);
            union.extend(seq.into_iter().skip(1));
        }
        if union.len() > 1 && union.first().map(|c| c.vertex_id) == union.last().map(|c| c.vertex_id) {
            union.pop();
        }
        let sel = fan.iter().any(|&f| m.polygons[f].selected);
        for &f in &fan {
            m.polygons.remove(f);
        }
        let p = m.add_polygon_corners(union, &tmpl);
        m.polygons[p].selected = sel;
    }
    m.vertices.remove(v);
    m.finalize();
}

pub fn remove_vertices(m: &mut EditablePoly) {
    for v in m.selected_vertices() {
        if m.vertices.contains_key(v) {
            remove_vertex(m, v);
        }
    }
}

/// Removes selected edges, merging the polygons on both sides. With
/// `remove_verts` (Ctrl+Remove) endpoints left with only two edges dissolve too.
pub fn remove_edges(m: &mut EditablePoly, remove_verts: bool) {
    let pairs: Vec<(VertexId, VertexId)> = m.selected_edges().iter().map(|&e| (m.edges[e].v0, m.edges[e].v1)).collect();
    let mut ends = HashSet::new();
    for (a, b) in pairs {
        let adj = Adjacency::build(m);
        let (Some(&f), Some(&g)) = (adj.half_edge.get(&(a, b)), adj.half_edge.get(&(b, a))) else { continue };
        if f == g {
            continue;
        }
        let pf = m.polygons[f].clone();
        let pg = m.polygons[g].clone();
        let rot = |p: &Polygon, start: VertexId| -> Vec<FaceCorner> {
            let i = p.position_of(start).unwrap();
            let n = p.corners.len();
            (0..n).map(|k| p.corners[(i + k) % n].clone()).collect()
        };
        let mut union = rot(&pf, b); // b ... a
        let gr = rot(&pg, a); // a ... b
        union.extend(gr[1..gr.len() - 1].iter().cloned());
        m.polygons[f].corners = union;
        m.polygons[f].selected |= pg.selected;
        m.polygons.remove(g);
        ends.insert(a);
        ends.insert(b);
        m.finalize();
    }
    if remove_verts {
        for v in ends {
            let adj = Adjacency::build(m);
            if m.vertices.contains_key(v) && adj.neighbors(v).len() == 2 {
                remove_vertex(m, v);
            }
        }
    }
    m.finalize();
}

/// Break: gives each polygon around a selected vertex its own copy of it.
pub fn break_vertices(m: &mut EditablePoly) {
    let adj = Adjacency::build(m);
    for v in m.selected_vertices() {
        let faces = adj.faces_of_vertex(v).to_vec();
        for &f in faces.iter().skip(1) {
            let mut nv = m.vertices[v].clone();
            nv.selected = true;
            let id = m.vertices.insert(nv);
            for c in &mut m.polygons[f].corners {
                if c.vertex_id == v {
                    c.vertex_id = id;
                }
            }
        }
    }
    m.finalize();
}

/// Split: detaches the surface along the selected edges (vertices duplicated
/// per side so the faces no longer share them).
pub fn split_edges(m: &mut EditablePoly) {
    let adj = Adjacency::build(m);
    let sel: HashSet<_> = m.selected_edges().iter().map(|&e| edge_key(m.edges[e].v0, m.edges[e].v1)).collect();
    let verts: HashSet<VertexId> = sel.iter().flat_map(|&(a, b)| [a, b]).collect();
    for v in verts {
        let faces = adj.faces_of_vertex(v).to_vec();
        let mut group: HashMap<PolyId, usize> = faces.iter().enumerate().map(|(i, &f)| (f, i)).collect();
        // merge faces adjacent across unselected edges at v
        for &n in adj.neighbors(v) {
            if sel.contains(&edge_key(v, n)) {
                continue;
            }
            let fs = adj.faces_of_edge(v, n);
            if fs.len() == 2 {
                let (ga, gb) = (group[&fs[0]], group[&fs[1]]);
                for g in group.values_mut() {
                    if *g == gb {
                        *g = ga;
                    }
                }
            }
        }
        let mut ids: Vec<usize> = group.values().copied().collect();
        ids.sort();
        ids.dedup();
        for &gid in ids.iter().skip(1) {
            let id = m.vertices.insert(m.vertices[v].clone());
            for (&f, &g) in &group {
                if g == gid {
                    for c in &mut m.polygons[f].corners {
                        if c.vertex_id == v {
                            c.vertex_id = id;
                        }
                    }
                }
            }
        }
    }
    m.finalize();
}
