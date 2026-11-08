//! Extrude for polygons (Group / Local Normal / By Polygon), edges and vertices,
//! plus the Shift+Drag border extrusion.

use super::dispatch::ExtrudeMode;
use super::*;
use crate::mesh::adjacency::Adjacency;
use std::collections::HashSet;

fn side_quad(m: &mut EditablePoly, f: &Polygon, a: VertexId, b: VertexId, a2: VertexId, b2: VertexId) -> PolyId {
    let uvs = [Vec2::new(0., 0.), Vec2::new(1., 0.), Vec2::new(1., 1.), Vec2::new(0., 1.)];
    let corners = [a, b, b2, a2].iter().zip(uvs).map(|(&v, uv)| FaceCorner::new(v, uv)).collect();
    m.add_polygon_corners(corners, f)
}

/// Extrudes the selected polygons by `height`. The extruded caps stay selected.
pub fn extrude_polygons(m: &mut EditablePoly, mode: ExtrudeMode, height: f32) {
    let sel = m.selected_polygons();
    if sel.is_empty() {
        return;
    }
    if mode == ExtrudeMode::ByPolygon {
        for pid in sel {
            let f = m.polygons[pid].clone();
            let nrm = m.poly_normal(pid);
            let olds: Vec<VertexId> = f.vertex_ids().collect();
            let news: Vec<VertexId> = olds.iter().map(|&v| m.add_vertex(m.pos(v) + nrm * height)).collect();
            let n = olds.len();
            for i in 0..n {
                side_quad(m, &f, olds[i], olds[(i + 1) % n], news[i], news[(i + 1) % n]);
            }
            for (c, &nv) in m.polygons[pid].corners.iter_mut().zip(&news) {
                c.vertex_id = nv;
            }
        }
        m.finalize();
        return;
    }
    let sset: HashSet<PolyId> = sel.iter().copied().collect();
    let adj = Adjacency::build(m);
    // boundary half-edges of the region
    let mut boundary: Vec<(PolyId, VertexId, VertexId)> = vec![];
    for &pid in &sel {
        for (a, b) in m.polygons[pid].half_edges() {
            let twin_in = adj.half_edge.get(&(b, a)).is_some_and(|g| sset.contains(g));
            if !twin_in {
                boundary.push((pid, a, b));
            }
        }
    }
    let bverts: HashSet<VertexId> = boundary.iter().flat_map(|&(_, a, b)| [a, b]).collect();
    // directions
    let normals: HashMap<PolyId, Vec3> = sel.iter().map(|&p| (p, m.poly_normal_raw(p))).collect();
    let mut vdir: HashMap<VertexId, Vec3> = HashMap::new();
    match mode {
        ExtrudeMode::Group => {
            let mut seen = HashSet::new();
            for &start in &sel {
                if seen.contains(&start) {
                    continue;
                }
                let comp = adj.flood(m, start, &mut seen, |g| sset.contains(&g));
                let d = comp.iter().map(|p| normals[p]).sum::<Vec3>().normalize_or(Vec3::Z);
                for p in comp {
                    for v in m.polygons[p].vertex_ids() {
                        vdir.insert(v, d);
                    }
                }
            }
        }
        _ => {
            for &p in &sel {
                for v in m.polygons[p].vertex_ids() {
                    *vdir.entry(v).or_default() += normals[&p].normalize_or_zero();
                }
            }
            for d in vdir.values_mut() {
                *d = d.normalize_or(Vec3::Z);
            }
        }
    }
    let mut dup: HashMap<VertexId, VertexId> = HashMap::new();
    for (&v, &d) in &vdir {
        let target = m.pos(v) + d * height;
        if bverts.contains(&v) {
            let nv = m.add_vertex(target);
            m.vertices[nv].color = m.vertices[v].color;
            dup.insert(v, nv);
        } else {
            m.vertices[v].position = target;
        }
    }
    for (pid, a, b) in boundary {
        let f = m.polygons[pid].clone();
        side_quad(m, &f, a, b, dup[&a], dup[&b]);
    }
    for &pid in &sel {
        for c in m.polygons[pid].corners.iter_mut() {
            if let Some(&nv) = dup.get(&c.vertex_id) {
                c.vertex_id = nv;
            }
        }
    }
    m.finalize();
}

/// Shift+Drag on open (border) edges: builds a zero-length strip of new
/// polygons off the selected border edges and selects the new outer edges so
/// the following Move drags them out. Returns the new vertices.
pub fn extrude_border_edges(m: &mut EditablePoly, offset: Vec3) -> Vec<VertexId> {
    let adj = Adjacency::build(m);
    let sel: Vec<(VertexId, VertexId)> = m.selected_edges().iter().map(|&e| (m.edges[e].v0, m.edges[e].v1)).collect();
    let mut dup: HashMap<VertexId, VertexId> = HashMap::new();
    let mut new_edges = vec![];
    for (x, y) in sel {
        // orient as the owning face's half-edge
        let (a, b) = if adj.half_edge.contains_key(&(x, y)) && !adj.half_edge.contains_key(&(y, x)) {
            (x, y)
        } else if adj.half_edge.contains_key(&(y, x)) && !adj.half_edge.contains_key(&(x, y)) {
            (y, x)
        } else {
            continue; // not a border edge
        };
        let f = m.polygons[adj.half_edge[&(a, b)]].clone();
        for v in [a, b] {
            dup.entry(v).or_insert_with(|| m.add_vertex(m.pos(v) + offset));
        }
        let (a2, b2) = (dup[&a], dup[&b]);
        let corners =
            [(b, 0., 0.), (a, 1., 0.), (a2, 1., 1.), (b2, 0., 1.)].iter().map(|&(v, u, w)| FaceCorner::new(v, Vec2::new(u, w))).collect();
        m.add_polygon_corners(corners, &f);
        new_edges.push((a2, b2));
    }
    m.finalize();
    m.clear_selection();
    for (a, b) in new_edges {
        if let Some(e) = m.edge_between(a, b) {
            m.edges[e].selected = true;
        }
    }
    dup.values().copied().collect()
}

/// Edge Extrude: chamfer the selected edges by `width`, then raise the new
/// strip by `height` to form a ridge.
pub fn extrude_edges(m: &mut EditablePoly, height: f32, width: f32) {
    let adj = Adjacency::build(m);
    if m.selected_edges().iter().all(|&e| adj.is_open_edge(m.edges[e].v0, m.edges[e].v1)) {
        let n = m.vertex_normals();
        let avg = m.selection_vertices(SubObject::Edge).iter().map(|v| n[v]).sum::<Vec3>().normalize_or(Vec3::Z);
        extrude_border_edges(m, avg * height);
        return;
    }
    super::chamfer::chamfer_edges(m, width.max(1e-4) * 0.5, 1, false);
    extrude_polygons(m, ExtrudeMode::Group, height);
}

/// Vertex Extrude: chamfer the vertex into a base of `width`, then pull a
/// pyramid tip out along the vertex normal by `height`.
pub fn extrude_vertices(m: &mut EditablePoly, height: f32, width: f32) {
    let normals = m.vertex_normals();
    let sel = m.selected_vertices();
    let dirs: Vec<(Vec3, Vec3)> = sel.iter().map(|v| (m.pos(*v), normals.get(v).copied().unwrap_or(Vec3::Z))).collect();
    super::chamfer::chamfer_vertices(m, width.max(1e-4) * 0.5);
    let caps = m.selected_polygons();
    m.clear_selection();
    for pid in caps {
        let c = m.poly_centroid(pid);
        let d = dirs.iter().min_by(|a, b| a.0.distance(c).total_cmp(&b.0.distance(c))).map(|x| x.1).unwrap_or(Vec3::Z);
        let tip = super::insert::poke_polygon(m, pid, c + d * height);
        m.vertices[tip].selected = true;
    }
    m.finalize();
}
