//! Ring / Loop shift (the spinner arrows next to Ring and Loop):
//! moves every selected edge one step along its ring (to the parallel edge
//! of the next quad) or along its loop (to the next edge through a vertex).

use super::adjacency::Adjacency;
use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

type Dir = (VertexId, VertexId);

/// Opposite side of half-edge `he` in its quad, as a half-edge.
fn quad_opposite(m: &EditablePoly, adj: &Adjacency, he: Dir) -> Option<Dir> {
    let p = &m.polygons[*adj.half_edge.get(&he)?];
    if p.corners.len() != 4 {
        return None;
    }
    let i = p.half_edges().position(|h| h == he)?;
    p.half_edges().nth((i + 2) % 4)
}

impl EditablePoly {
    /// Selected edges oriented consistently: neighbours along a loop point
    /// head to tail, neighbours across a quad point the same way.
    fn oriented_selected_edges(&self, adj: &Adjacency) -> Vec<Dir> {
        let sel: HashSet<(VertexId, VertexId)> =
            self.selected_edges().iter().map(|&e| edge_key(self.edges[e].v0, self.edges[e].v1)).collect();
        let mut out: HashMap<(VertexId, VertexId), Dir> = HashMap::new();
        let mut keys: Vec<_> = sel.iter().copied().collect();
        keys.sort();
        for k in keys {
            if out.contains_key(&k) {
                continue;
            }
            let mut queue = VecDeque::from([k]);
            out.insert(k, k);
            while let Some(key) = queue.pop_front() {
                let (a, b) = out[&key];
                let mut next: Vec<Dir> = vec![];
                if let Some(n) = self.loop_next(adj, a, b) {
                    next.push((b, n));
                }
                if let Some(p) = self.loop_next(adj, b, a) {
                    next.push((p, a));
                }
                // Same direction on the opposite side of each adjacent quad.
                next.extend(quad_opposite(self, adj, (a, b)).map(|(s, t)| (t, s)));
                next.extend(quad_opposite(self, adj, (b, a)));
                for d in next {
                    let nk = edge_key(d.0, d.1);
                    if sel.contains(&nk) && !out.contains_key(&nk) {
                        out.insert(nk, d);
                        queue.push_back(nk);
                    }
                }
            }
        }
        out.into_values().collect()
    }

    fn shift_edges(&mut self, step: impl Fn(&EditablePoly, &Adjacency, Dir) -> Option<Dir>) -> bool {
        let adj = Adjacency::build(self);
        let targets: Vec<Dir> = self.oriented_selected_edges(&adj).into_iter().map(|d| step(self, &adj, d).unwrap_or(d)).collect();
        let before = self.selected_edges();
        for e in self.edges.values_mut() {
            e.selected = false;
        }
        for (a, b) in targets {
            if let Some(e) = self.edge_between(a, b) {
                self.edges[e].selected = true;
            }
        }
        self.selected_edges() != before
    }

    /// Moves the edge selection one quad along its rings (`forward` picks the side).
    pub fn shift_ring(&mut self, forward: bool) -> bool {
        self.shift_edges(
            |m, adj, (a, b)| {
                if forward { quad_opposite(m, adj, (a, b)).map(|(s, t)| (t, s)) } else { quad_opposite(m, adj, (b, a)) }
            },
        )
    }

    /// Moves the edge selection one edge along its loops.
    pub fn shift_loop(&mut self, forward: bool) -> bool {
        self.shift_edges(
            |m, adj, (a, b)| {
                if forward { m.loop_next(adj, a, b).map(|n| (b, n)) } else { m.loop_next(adj, b, a).map(|p| (p, a)) }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::primitives::Primitive;

    /// Cylinder side: a 12-sided, 4-high quad band (plus caps).
    fn cylinder() -> crate::mesh::EditablePoly {
        Primitive::Cylinder { radius: 1.0, height: 2.0, height_segs: 4, cap_segs: 1, sides: 12 }.to_editable_poly()
    }

    fn vertical(m: &crate::mesh::EditablePoly, e: crate::mesh::EdgeId) -> bool {
        let d = m.pos(m.edges[e].v0) - m.pos(m.edges[e].v1);
        d.x.abs() + d.y.abs() < 1e-4
    }

    #[test]
    fn ring_and_loop_shift_move_a_whole_ring_or_loop() {
        let mut m = cylinder();
        let e = m.edges.keys().find(|&e| vertical(&m, e)).unwrap();
        m.edges[e].selected = true;
        m.select_ring();
        let ring: Vec<_> = m.selected_edges();
        assert_eq!(ring.len(), 12, "a horizontal ring of 12 vertical edges");
        let mid = |m: &crate::mesh::EditablePoly| {
            m.selected_edges().iter().map(|&e| (m.pos(m.edges[e].v0) + m.pos(m.edges[e].v1)).z * 0.5).sum::<f32>() / 12.0
        };
        let z0 = mid(&m);
        // Loop shift: every edge moves one segment up (or down) its vertical loop.
        assert!(m.shift_loop(true));
        assert_eq!(m.selected_edges().len(), 12);
        let z1 = mid(&m);
        assert!((z1 - z0).abs() > 1e-3);
        assert!(m.shift_loop(false));
        assert!((mid(&m) - z0).abs() < 1e-4, "back where it started");
        // Ring shift of a full ring is the same ring again; of one edge it moves sideways.
        m.shift_ring(true);
        assert_eq!(m.selected_edges().len(), 12);
        m.clear_selection();
        m.edges[e].selected = true;
        assert!(m.shift_ring(true));
        let moved = m.selected_edges();
        assert_eq!(moved.len(), 1);
        assert_ne!(moved[0], e);
        assert!(vertical(&m, moved[0]));
        m.shift_ring(false);
        assert_eq!(m.selected_edges(), vec![e]);
    }
}
