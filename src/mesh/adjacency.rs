//! On-demand adjacency tables (built in O(n), discarded after use).

use super::*;
use std::collections::HashSet;

#[derive(Default, Debug)]
pub struct Adjacency {
    /// Directed half-edge (a -> b) to the polygon that owns it.
    pub half_edge: HashMap<(VertexId, VertexId), PolyId>,
    /// Undirected edge to all adjacent polygons.
    pub edge_faces: HashMap<(VertexId, VertexId), Vec<PolyId>>,
    pub vertex_faces: HashMap<VertexId, Vec<PolyId>>,
    pub vertex_neighbors: HashMap<VertexId, Vec<VertexId>>,
}

impl Adjacency {
    pub fn build(m: &EditablePoly) -> Self {
        let mut a = Adjacency::default();
        for (pid, p) in &m.polygons {
            for (x, y) in p.half_edges() {
                a.half_edge.insert((x, y), pid);
                a.edge_faces.entry(edge_key(x, y)).or_default().push(pid);
            }
            for v in p.vertex_ids() {
                let list = a.vertex_faces.entry(v).or_default();
                if !list.contains(&pid) {
                    list.push(pid);
                }
            }
        }
        for e in m.edges.values() {
            a.vertex_neighbors.entry(e.v0).or_default().push(e.v1);
            a.vertex_neighbors.entry(e.v1).or_default().push(e.v0);
        }
        a
    }

    pub fn faces_of_edge(&self, a: VertexId, b: VertexId) -> &[PolyId] {
        self.edge_faces.get(&edge_key(a, b)).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn faces_of_vertex(&self, v: VertexId) -> &[PolyId] {
        self.vertex_faces.get(&v).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn neighbors(&self, v: VertexId) -> &[VertexId] {
        self.vertex_neighbors.get(&v).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn is_open_edge(&self, a: VertexId, b: VertexId) -> bool {
        self.faces_of_edge(a, b).len() == 1
    }

    /// Directed half-edges with no twin: the mesh border.
    pub fn open_half_edges(&self) -> Vec<(VertexId, VertexId)> {
        let mut v: Vec<_> = self.half_edge.keys().filter(|(a, b)| !self.half_edge.contains_key(&(*b, *a))).copied().collect();
        v.sort();
        v
    }

    pub fn is_border_vertex(&self, v: VertexId) -> bool {
        self.neighbors(v).iter().any(|&n| self.is_open_edge(v, n))
    }

    /// Faces around `v` in winding order (fan). For border vertices the fan starts
    /// at the border. Each step crosses from face `f` (containing prev->v) to the
    /// face that owns v->prev.
    pub fn ordered_fan(&self, m: &EditablePoly, v: VertexId) -> Vec<PolyId> {
        let faces = self.faces_of_vertex(v);
        if faces.is_empty() {
            return vec![];
        }
        let prev_of = |f: PolyId| -> VertexId {
            let p = &m.polygons[f];
            let i = p.position_of(v).unwrap();
            p.corners[(i + p.corners.len() - 1) % p.corners.len()].vertex_id
        };
        let next_of = |f: PolyId| -> VertexId {
            let p = &m.polygons[f];
            let i = p.position_of(v).unwrap();
            p.corners[(i + 1) % p.corners.len()].vertex_id
        };
        // Start at a face whose "next" edge (v -> next) has no twin, if any.
        let start = faces.iter().copied().find(|&f| !self.half_edge.contains_key(&(next_of(f), v))).unwrap_or(faces[0]);
        let mut out = vec![start];
        let mut seen: HashSet<PolyId> = HashSet::from([start]);
        let mut cur = start;
        // Walk backwards: face owning (v -> prev(cur)) ... use twin of prev edge.
        loop {
            let pv = prev_of(cur);
            match self.half_edge.get(&(v, pv)) {
                Some(&g) if !seen.contains(&g) => {
                    out.push(g);
                    seen.insert(g);
                    cur = g;
                }
                _ => break,
            }
        }
        out
    }

    /// Closed border loops as vertex sequences following the open half-edges.
    pub fn boundary_loops(&self) -> Vec<Vec<VertexId>> {
        let open = self.open_half_edges();
        let mut next: HashMap<VertexId, VertexId> = HashMap::new();
        for &(a, b) in &open {
            next.insert(a, b);
        }
        let mut used: HashSet<VertexId> = HashSet::new();
        let mut loops = Vec::new();
        for &(a, _) in &open {
            if used.contains(&a) {
                continue;
            }
            let mut lp = vec![a];
            used.insert(a);
            let mut cur = a;
            while let Some(&n) = next.get(&cur) {
                if n == a || used.contains(&n) {
                    break;
                }
                lp.push(n);
                used.insert(n);
                cur = n;
            }
            if lp.len() >= 3 {
                loops.push(lp);
            }
        }
        loops
    }

    /// Connected polygon islands (elements).
    pub fn elements(&self, m: &EditablePoly) -> Vec<Vec<PolyId>> {
        let mut seen: HashSet<PolyId> = HashSet::new();
        let mut out = Vec::new();
        for pid in m.polygons.keys() {
            if seen.contains(&pid) {
                continue;
            }
            out.push(self.flood(m, pid, &mut seen, |_| true));
        }
        out
    }

    /// Flood fill across shared edges, restricted by `allow`.
    pub fn flood(&self, m: &EditablePoly, start: PolyId, seen: &mut HashSet<PolyId>, allow: impl Fn(PolyId) -> bool) -> Vec<PolyId> {
        let mut stack = vec![start];
        seen.insert(start);
        let mut out = Vec::new();
        while let Some(f) = stack.pop() {
            out.push(f);
            for (a, b) in m.polygons[f].half_edges() {
                for &g in self.faces_of_edge(a, b) {
                    if !seen.contains(&g) && allow(g) {
                        seen.insert(g);
                        stack.push(g);
                    }
                }
            }
        }
        out
    }
}
