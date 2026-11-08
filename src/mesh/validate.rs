//! Structural validation used by unit tests (and debug assertions).

use super::*;
use std::collections::HashSet;

impl EditablePoly {
    /// Checks referential integrity, finite attributes, consistent winding
    /// (each directed half-edge used at most once) and edge-table sync.
    pub fn validate(&self) -> Result<(), String> {
        let mut half = HashSet::new();
        let mut wanted = HashSet::new();
        for (pid, p) in &self.polygons {
            if p.corners.len() < 3 {
                return Err(format!("{pid:?} has {} corners", p.corners.len()));
            }
            for c in &p.corners {
                if !self.vertices.contains_key(c.vertex_id) {
                    return Err(format!("{pid:?} references missing vertex"));
                }
                if !c.uv.is_finite() {
                    return Err("non-finite uv".into());
                }
            }
            for (a, b) in p.half_edges() {
                if a == b {
                    return Err(format!("{pid:?} has repeated consecutive vertex"));
                }
                if !half.insert((a, b)) {
                    return Err(format!("half-edge used twice (bad winding / non-manifold) in {pid:?}"));
                }
                wanted.insert(edge_key(a, b));
            }
            if !self.poly_normal_raw(pid).is_finite() {
                return Err("non-finite normal".into());
            }
        }
        for v in self.vertices.values() {
            if !v.position.is_finite() {
                return Err("non-finite position".into());
            }
        }
        if wanted.len() != self.edges.len() {
            return Err(format!("edge table out of sync: {} vs {}", wanted.len(), self.edges.len()));
        }
        for e in self.edges.values() {
            if !wanted.contains(&edge_key(e.v0, e.v1)) {
                return Err("stale edge".into());
            }
        }
        for ns in self.corner_normals().values() {
            if ns.iter().any(|n| !n.is_finite() || (n.length() - 1.0).abs() > 1e-3) {
                return Err("bad corner normal".into());
            }
        }
        Ok(())
    }

    /// True when every edge has exactly two faces.
    pub fn is_closed(&self) -> bool {
        let adj = super::adjacency::Adjacency::build(self);
        adj.edge_faces.values().all(|f| f.len() == 2)
    }

    /// V - E + F
    pub fn euler(&self) -> i64 {
        let (v, e, f) = self.counts();
        v as i64 - e as i64 + f as i64
    }

    /// Signed volume (positive for outward-facing closed meshes).
    pub fn signed_volume(&self) -> f32 {
        let mut vol = 0.0;
        for pid in self.polygons.keys() {
            for [a, b, c] in self.poly_triangles(pid) {
                vol += a.dot(b.cross(c)) / 6.0;
            }
        }
        vol
    }
}
