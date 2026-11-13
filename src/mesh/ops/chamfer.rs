//! Chamfer for edges (Amount / Segments / Tension / Open) and vertices.

use super::*;
use crate::mesh::adjacency::Adjacency;
use std::collections::HashSet;

type Key = (VertexId, VertexId);

struct Ctx {
    amount: f32,
    slide: HashMap<(VertexId, VertexId), VertexId>,
}

impl Ctx {
    fn slide_pos(&self, m: &EditablePoly, v: VertexId, other: VertexId) -> Vec3 {
        let (pv, po) = (m.pos(v), m.pos(other));
        let t = (self.amount / pv.distance(po).max(1e-6)).min(0.49);
        pv.lerp(po, t)
    }

    /// New vertex on edge (v -> other) at `amount` from v.
    fn slide(&mut self, m: &mut EditablePoly, v: VertexId, other: VertexId) -> VertexId {
        if let Some(&id) = self.slide.get(&(v, other)) {
            return id;
        }
        let p = self.slide_pos(m, v, other);
        let id = m.add_vertex(p);
        self.slide.insert((v, other), id);
        id
    }
}

fn corner_uv(f: &Polygon, i: usize) -> Vec2 {
    f.corners[i].uv
}

/// Chamfers the selected (manifold) edges. The new chamfer polygons end up selected.
pub fn chamfer_edges(m: &mut EditablePoly, amount: f32, segments: u32, open: bool) {
    chamfer_edges_ex(m, amount, segments, 0.5, open)
}

pub fn chamfer_edges_ex(m: &mut EditablePoly, amount: f32, segments: u32, tension: f32, open: bool) {
    let adj = Adjacency::build(m);
    let sel: HashSet<Key> = m
        .selected_edges()
        .iter()
        .map(|&e| edge_key(m.edges[e].v0, m.edges[e].v1))
        .filter(|&(a, b)| adj.faces_of_edge(a, b).len() == 2)
        .collect();
    if sel.is_empty() || amount <= 0.0 {
        return;
    }
    let mut k: HashMap<VertexId, usize> = HashMap::new();
    for &(a, b) in &sel {
        *k.entry(a).or_default() += 1;
        *k.entry(b).or_default() += 1;
    }
    // slide edges: other edge of a corner that has exactly one selected edge
    let mut slide_edges: HashSet<Key> = HashSet::new();
    for p in m.polygons.values() {
        let n = p.corners.len();
        for i in 0..n {
            let v = p.corners[i].vertex_id;
            if !k.contains_key(&v) {
                continue;
            }
            let prev = p.corners[(i + n - 1) % n].vertex_id;
            let next = p.corners[(i + 1) % n].vertex_id;
            let (si, so) = (sel.contains(&edge_key(prev, v)), sel.contains(&edge_key(v, next)));
            if si && !so {
                slide_edges.insert(edge_key(v, next));
            } else if so && !si {
                slide_edges.insert(edge_key(prev, v));
            }
        }
    }
    let mut ctx = Ctx { amount, slide: HashMap::new() };
    let mut repl: HashMap<(PolyId, VertexId), Vec<VertexId>> = HashMap::new();
    let mut kept: HashSet<VertexId> = HashSet::new();
    let pids: Vec<PolyId> = m.polygons.keys().collect();
    for pid in pids {
        let f = m.polygons[pid].clone();
        let n = f.corners.len();
        if !f.vertex_ids().any(|v| k.contains_key(&v)) {
            continue;
        }
        let mut corners = Vec::new();
        for i in 0..n {
            let v = f.corners[i].vertex_id;
            if !k.contains_key(&v) {
                corners.push(f.corners[i].clone());
                continue;
            }
            let (ip, inx) = ((i + n - 1) % n, (i + 1) % n);
            let prev = f.corners[ip].vertex_id;
            let next = f.corners[inx].vertex_id;
            let (si, so) = (sel.contains(&edge_key(prev, v)), sel.contains(&edge_key(v, next)));
            let uv = corner_uv(&f, i);
            let uv_to = |j: usize| uv.lerp(corner_uv(&f, j), 0.2);
            let mut pts: Vec<(VertexId, Vec2)> = vec![];
            if si && so {
                let pos = ctx.slide_pos(m, v, prev) + ctx.slide_pos(m, v, next) - m.pos(v);
                pts.push((m.add_vertex(pos), uv_to(ip) + uv_to(inx) - uv));
            } else if si {
                pts.push((ctx.slide(m, v, next), uv_to(inx)));
            } else if so {
                pts.push((ctx.slide(m, v, prev), uv_to(ip)));
            } else {
                let p_in = slide_edges.contains(&edge_key(prev, v));
                let p_out = slide_edges.contains(&edge_key(v, next));
                if p_in {
                    pts.push((ctx.slide(m, v, prev), uv_to(ip)));
                }
                if !(p_in && p_out) {
                    pts.push((v, uv));
                    kept.insert(v);
                }
                if p_out {
                    pts.push((ctx.slide(m, v, next), uv_to(inx)));
                }
            }
            repl.insert((pid, v), pts.iter().map(|p| p.0).collect());
            corners.extend(pts.into_iter().map(|(id, uv)| FaceCorner::new(id, uv)));
        }
        m.polygons[pid].corners = corners;
    }
    // strips
    let mut strips: Vec<(PolyId, [VertexId; 4], VertexId, VertexId)> = vec![];
    let mut cap_next: HashMap<VertexId, HashMap<VertexId, VertexId>> = HashMap::new();
    let mut new_polys = vec![];
    for &(x, y) in &sel {
        let (u, v) = if adj.half_edge.contains_key(&(x, y)) { (x, y) } else { (y, x) };
        let (f1, f2) = (adj.half_edge[&(u, v)], adj.half_edge[&(v, u)]);
        let a = *repl[&(f1, v)].first().unwrap();
        let b = *repl[&(f1, u)].last().unwrap();
        let c = *repl[&(f2, u)].first().unwrap();
        let d = *repl[&(f2, v)].last().unwrap();
        let mut ring = vec![a, b];
        if kept.contains(&u) && k[&u] == 1 {
            ring.push(u);
        }
        ring.extend([c, d]);
        if kept.contains(&v) && k[&v] == 1 {
            ring.push(v);
        }
        ring.dedup();
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        cap_next.entry(u).or_default().insert(c, b);
        cap_next.entry(v).or_default().insert(a, d);
        if ring.len() >= 3 {
            let tmpl = m.polygons[f1].clone();
            let pid = m.add_polygon(&ring);
            m.polygons[pid].smoothing_group = tmpl.smoothing_group;
            m.polygons[pid].material_id = tmpl.material_id;
            if ring.len() == 4 {
                strips.push((pid, [a, b, c, d], u, v));
            }
            new_polys.push(pid);
        }
    }
    // corner caps where >= 3 chamfered edges meet
    let mut caps: Vec<(Vec<VertexId>, VertexId)> = vec![];
    for (&vtx, next) in &cap_next {
        if k[&vtx] < 3 || kept.contains(&vtx) {
            continue;
        }
        let Some(&start) = next.keys().min() else {
            continue;
        };
        let mut lp = vec![start];
        let mut cur = start;
        while let Some(&n) = next.get(&cur) {
            if n == start || lp.len() > next.len() {
                break;
            }
            lp.push(n);
            cur = n;
        }
        if lp.len() >= 3 {
            caps.push((lp, vtx));
        }
    }
    if segments > 1 {
        subdivide_strips(m, &strips, &mut caps, segments, tension, &mut new_polys);
    }
    for (lp, _) in caps {
        new_polys.push(m.add_polygon(&lp));
    }
    for v in k.keys() {
        if !kept.contains(v) {
            m.vertices.remove(*v);
        }
    }
    m.clear_selection();
    if open {
        for &p in &new_polys {
            m.polygons.remove(p);
        }
    } else {
        for &p in &new_polys {
            if let Some(p) = m.polygons.get_mut(p) {
                p.selected = true;
            }
        }
    }
    m.finalize();
    // Keep the new chamfer edges selected in Edge mode.
    let hes: Vec<(VertexId, VertexId)> =
        new_polys.iter().filter_map(|&p| m.polygons.get(p)).flat_map(|p| p.half_edges().collect::<Vec<_>>()).collect();
    for (a, b) in hes {
        if let Some(e) = m.edge_between(a, b) {
            m.edges[e].selected = true;
        }
    }
}

/// Replaces each 4-sided strip by `segs` quads whose cross-section follows a
/// quadratic curve pulled toward the original edge by `tension`.
fn subdivide_strips(
    m: &mut EditablePoly,
    strips: &[(PolyId, [VertexId; 4], VertexId, VertexId)],
    caps: &mut [(Vec<VertexId>, VertexId)],
    segs: u32,
    tension: f32,
    new_polys: &mut Vec<PolyId>,
) {
    let mut cache: HashMap<Key, Vec<VertexId>> = HashMap::new();
    let orig: HashMap<VertexId, Vec3> = strips.iter().flat_map(|s| [(s.2, m.pos(s.2)), (s.3, m.pos(s.3))]).collect();
    let mut chain = |m: &mut EditablePoly, from: VertexId, to: VertexId, o: Vec3| -> Vec<VertexId> {
        let key = edge_key(from, to);
        let list = cache.entry(key).or_insert_with(|| {
            let (p0, p2) = (m.pos(key.0), m.pos(key.1));
            let ctrl = ((p0 + p2) * 0.5).lerp(o, tension.clamp(0.0, 1.0));
            let mut l = vec![key.0];
            for i in 1..segs {
                let t = i as f32 / segs as f32;
                let p = p0 * (1. - t) * (1. - t) + ctrl * 2. * t * (1. - t) + p2 * t * t;
                l.push(m.add_vertex(p));
            }
            l.push(key.1);
            l
        });
        if key.0 == from { list.clone() } else { list.iter().rev().copied().collect() }
    };
    for &(pid, [a, b, c, d], u, v) in strips {
        let p = chain(m, b, c, orig[&u]);
        let q = chain(m, d, a, orig[&v]);
        let tmpl = m.polygons[pid].clone();
        m.polygons.remove(pid);
        new_polys.retain(|&x| x != pid);
        let n = segs as usize;
        for i in 0..n {
            let id = m.add_polygon(&[q[n - i], p[i], p[i + 1], q[n - i - 1]]);
            m.polygons[id].smoothing_group = tmpl.smoothing_group;
            m.polygons[id].material_id = tmpl.material_id;
            new_polys.push(id);
        }
    }
    // existing faces that share a subdivided end edge (e.g. valence-3 corners)
    for p in m.polygons.values_mut() {
        let n = p.corners.len();
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let (x, y) = (p.corners[i].vertex_id, p.corners[(i + 1) % n].vertex_id);
            out.push(p.corners[i].clone());
            if let Some(list) = cache.get(&edge_key(x, y)) {
                let uv = p.corners[i].uv;
                let inner: Vec<VertexId> = list[1..list.len() - 1].to_vec();
                let ordered: Vec<VertexId> = if list[0] == x { inner } else { inner.into_iter().rev().collect() };
                out.extend(ordered.into_iter().map(|v| FaceCorner::new(v, uv)));
            }
        }
        p.corners = out;
    }
    for (lp, _) in caps.iter_mut() {
        let mut out = vec![];
        let n = lp.len();
        for i in 0..n {
            let (x, y) = (lp[i], lp[(i + 1) % n]);
            out.push(x);
            if let Some(list) = cache.get(&edge_key(x, y)) {
                let inner: Vec<VertexId> = list[1..list.len() - 1].to_vec();
                if list[0] == x {
                    out.extend(inner);
                } else {
                    out.extend(inner.into_iter().rev());
                }
            }
        }
        *lp = out;
    }
}

/// Chamfers each selected vertex into an N-gon cap (selected afterwards).
pub fn chamfer_vertices(m: &mut EditablePoly, amount: f32) {
    let sel: HashSet<VertexId> = m.selected_vertices().into_iter().collect();
    if sel.is_empty() || amount <= 0.0 {
        return;
    }
    let mut ctx = Ctx { amount, slide: HashMap::new() };
    let mut cap_next: HashMap<VertexId, HashMap<VertexId, VertexId>> = HashMap::new();
    let pids: Vec<PolyId> = m.polygons.keys().collect();
    for pid in pids {
        let f = m.polygons[pid].clone();
        let n = f.corners.len();
        let mut corners = vec![];
        for i in 0..n {
            let v = f.corners[i].vertex_id;
            if !sel.contains(&v) {
                corners.push(f.corners[i].clone());
                continue;
            }
            let prev = f.corners[(i + n - 1) % n].vertex_id;
            let next = f.corners[(i + 1) % n].vertex_id;
            let a = ctx.slide(m, v, prev);
            let b = ctx.slide(m, v, next);
            let uv = f.corners[i].uv;
            corners.push(FaceCorner::new(a, uv.lerp(f.corners[(i + n - 1) % n].uv, 0.2)));
            corners.push(FaceCorner::new(b, uv.lerp(f.corners[(i + 1) % n].uv, 0.2)));
            cap_next.entry(v).or_default().insert(b, a);
        }
        m.polygons[pid].corners = corners;
    }
    m.clear_selection();
    for (v, next) in cap_next {
        m.vertices.remove(v);
        let Some(&start) = next.keys().min() else {
            continue;
        };
        let mut lp = vec![start];
        let mut cur = start;
        while let Some(&n) = next.get(&cur) {
            if n == start || lp.len() > next.len() {
                break;
            }
            lp.push(n);
            cur = n;
        }
        if lp.len() >= 3 && next.get(&cur) == Some(&start) {
            let p = m.add_polygon(&lp);
            m.polygons[p].selected = true;
        }
    }
    m.finalize();
}
