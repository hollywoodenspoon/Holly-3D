//! Sub-object selection topology tools: Grow, Shrink, Loop, Ring, Element, Border.

use super::adjacency::Adjacency;
use super::*;
use std::collections::HashSet;

impl EditablePoly {
    pub fn grow_selection(&mut self, level: SubObject) {
        let adj = Adjacency::build(self);
        match level {
            SubObject::Vertex => {
                let sel = self.selected_vertices();
                for v in sel {
                    for &n in adj.neighbors(v) {
                        self.vertices[n].selected = true;
                    }
                }
            }
            SubObject::Edge | SubObject::Border => {
                let verts = self.selection_vertices(SubObject::Edge);
                let vs: HashSet<_> = verts.into_iter().collect();
                for e in self.edges.values_mut() {
                    if vs.contains(&e.v0) || vs.contains(&e.v1) {
                        e.selected = true;
                    }
                }
            }
            SubObject::Polygon | SubObject::Element => {
                let vs: HashSet<_> = self.selection_vertices(SubObject::Polygon).into_iter().collect();
                for p in self.polygons.values_mut() {
                    if p.vertex_ids().any(|v| vs.contains(&v)) {
                        p.selected = true;
                    }
                }
            }
            SubObject::Object => {}
        }
    }

    pub fn shrink_selection(&mut self, level: SubObject) {
        let adj = Adjacency::build(self);
        match level {
            SubObject::Vertex => {
                let sel: HashSet<_> = self.selected_vertices().into_iter().collect();
                for &v in &sel {
                    if adj.neighbors(v).iter().any(|n| !sel.contains(n)) || adj.is_border_vertex(v) {
                        self.vertices[v].selected = false;
                    }
                }
            }
            SubObject::Edge | SubObject::Border => {
                // drop edges having an endpoint touched by an unselected edge
                let mut touched_unsel: HashSet<VertexId> = HashSet::new();
                for e in self.edges.values().filter(|e| !e.selected) {
                    touched_unsel.insert(e.v0);
                    touched_unsel.insert(e.v1);
                }
                for e in self.edges.values_mut() {
                    if e.selected && (touched_unsel.contains(&e.v0) || touched_unsel.contains(&e.v1)) {
                        e.selected = false;
                    }
                }
            }
            SubObject::Polygon | SubObject::Element => {
                let mut border: HashSet<VertexId> = HashSet::new();
                for p in self.polygons.values().filter(|p| !p.selected) {
                    border.extend(p.vertex_ids());
                }
                for (a, b) in adj.open_half_edges() {
                    border.insert(a);
                    border.insert(b);
                }
                for p in self.polygons.values_mut() {
                    if p.selected && p.vertex_ids().any(|v| border.contains(&v)) {
                        p.selected = false;
                    }
                }
            }
            SubObject::Object => {}
        }
    }

    /// Edge continuing straight through `v` coming from `from` (valence-4 rule;
    /// along the border for border edges).
    fn loop_next(&self, adj: &Adjacency, from: VertexId, v: VertexId) -> Option<VertexId> {
        if adj.is_open_edge(from, v) {
            return adj.neighbors(v).iter().copied().find(|&n| n != from && adj.is_open_edge(v, n));
        }
        if adj.neighbors(v).len() != 4 || adj.is_border_vertex(v) {
            return None;
        }
        // Faces touching edge (from,v) — the loop edge is the neighbour sharing none of them.
        let faces = adj.faces_of_edge(from, v);
        adj.neighbors(v).iter().copied().find(|&n| n != from && !adj.faces_of_edge(v, n).iter().any(|f| faces.contains(f)))
    }

    pub fn select_loop(&mut self) {
        let adj = Adjacency::build(self);
        let seeds: Vec<_> = self.selected_edges().iter().map(|&e| (self.edges[e].v0, self.edges[e].v1)).collect();
        for (a, b) in seeds {
            for (mut from, mut cur) in [(a, b), (b, a)] {
                for _ in 0..10_000 {
                    let Some(n) = self.loop_next(&adj, from, cur) else {
                        break;
                    };
                    let Some(e) = self.edge_between(cur, n) else {
                        break;
                    };
                    if self.edges[e].selected {
                        break;
                    }
                    self.edges[e].selected = true;
                    from = cur;
                    cur = n;
                }
            }
        }
    }

    pub fn select_ring(&mut self) {
        let adj = Adjacency::build(self);
        let seeds: Vec<_> = self.selected_edges().iter().map(|&e| (self.edges[e].v0, self.edges[e].v1)).collect();
        for (a, b) in seeds {
            let mut stack = vec![(a, b)];
            while let Some((x, y)) = stack.pop() {
                for &f in adj.faces_of_edge(x, y) {
                    let p = &self.polygons[f];
                    if p.corners.len() != 4 {
                        continue;
                    }
                    let i = p.half_edges().position(|(s, t)| edge_key(s, t) == edge_key(x, y)).unwrap();
                    let (s, t) = p.half_edges().nth((i + 2) % 4).unwrap();
                    if let Some(e) = self.edge_between(s, t)
                        && !self.edges[e].selected
                    {
                        self.edges[e].selected = true;
                        stack.push((s, t));
                    }
                }
            }
        }
    }

    /// Expands polygon selection to whole connected elements.
    pub fn select_elements(&mut self) {
        let adj = Adjacency::build(self);
        for el in adj.elements(self) {
            if el.iter().any(|&p| self.polygons[p].selected) {
                el.iter().for_each(|&p| self.polygons[p].selected = true);
            }
        }
    }

    /// Expands selected open edges to their complete border loops.
    pub fn select_borders(&mut self) {
        let adj = Adjacency::build(self);
        for lp in adj.boundary_loops() {
            let n = lp.len();
            let edges: Vec<EdgeId> = (0..n).filter_map(|i| self.edge_between(lp[i], lp[(i + 1) % n])).collect();
            if edges.iter().any(|&e| self.edges[e].selected) {
                edges.iter().for_each(|&e| self.edges[e].selected = true);
            }
        }
        // interior edges are not borders
        let interior: Vec<EdgeId> =
            self.edges.iter().filter(|(_, e)| e.selected && !adj.is_open_edge(e.v0, e.v1)).map(|(k, _)| k).collect();
        interior.into_iter().for_each(|e| self.edges[e].selected = false);
    }

    pub fn select_by_material_id(&mut self, id: u16, add: bool) {
        for p in self.polygons.values_mut() {
            p.selected = (add && p.selected) || p.material_id == id;
        }
    }

    pub fn select_by_smoothing_group(&mut self, bits: u32) {
        for p in self.polygons.values_mut() {
            p.selected = p.smoothing_group & bits != 0;
        }
    }

    pub fn select_all(&mut self, level: SubObject, on: bool) {
        match level {
            SubObject::Vertex => self.vertices.values_mut().for_each(|v| v.selected = on),
            SubObject::Edge | SubObject::Border => self.edges.values_mut().for_each(|e| e.selected = on),
            _ => self.polygons.values_mut().for_each(|p| p.selected = on),
        }
    }

    pub fn invert_selection(&mut self, level: SubObject) {
        match level {
            SubObject::Vertex => self.vertices.values_mut().for_each(|v| v.selected = !v.selected),
            SubObject::Edge | SubObject::Border => self.edges.values_mut().for_each(|e| e.selected = !e.selected),
            _ => self.polygons.values_mut().for_each(|p| p.selected = !p.selected),
        }
    }

    pub fn selection_count(&self, level: SubObject) -> usize {
        match level {
            SubObject::Vertex => self.vertices.values().filter(|v| v.selected).count(),
            SubObject::Edge | SubObject::Border => self.edges.values().filter(|e| e.selected).count(),
            SubObject::Polygon | SubObject::Element => self.polygons.values().filter(|p| p.selected).count(),
            SubObject::Object => 0,
        }
    }
}
