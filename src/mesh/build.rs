//! Construction & bookkeeping helpers for `EditablePoly`.

use super::*;
use bevy::math::Mat4;
use std::collections::HashSet;

impl EditablePoly {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vertex(&mut self, p: Vec3) -> VertexId {
        self.vertices.insert(Vertex::new(p))
    }

    /// Adds a polygon from vertex ids with zero UVs. Call `rebuild_edges` afterwards
    /// (or use `add_polygon_uv` + `rebuild_edges` in batch).
    pub fn add_polygon(&mut self, verts: &[VertexId]) -> PolyId {
        let corners = verts.iter().map(|&v| FaceCorner::new(v, Vec2::ZERO)).collect();
        self.polygons.insert(Polygon::new(corners))
    }

    pub fn add_polygon_uv(&mut self, verts: &[VertexId], uvs: &[Vec2]) -> PolyId {
        let corners = verts.iter().zip(uvs.iter().chain(std::iter::repeat(&Vec2::ZERO))).map(|(&v, &uv)| FaceCorner::new(v, uv)).collect();
        self.polygons.insert(Polygon::new(corners))
    }

    pub fn add_polygon_corners(&mut self, corners: Vec<FaceCorner>, template: &Polygon) -> PolyId {
        self.polygons.insert(Polygon {
            corners,
            smoothing_group: template.smoothing_group,
            material_id: template.material_id,
            selected: false,
        })
    }

    pub fn pos(&self, v: VertexId) -> Vec3 {
        self.vertices.get(v).map(|v| v.position).unwrap_or(Vec3::ZERO)
    }

    pub fn edge_between(&self, a: VertexId, b: VertexId) -> Option<EdgeId> {
        self.edge_map.get(&edge_key(a, b)).copied()
    }

    /// Recreates the edge arena from polygon boundaries, preserving the key and
    /// flags of every edge whose vertex pair still exists.
    pub fn rebuild_edges(&mut self) {
        let mut wanted: HashSet<(VertexId, VertexId)> = HashSet::new();
        for p in self.polygons.values() {
            for (a, b) in p.half_edges() {
                if a != b {
                    wanted.insert(edge_key(a, b));
                }
            }
        }
        let stale: Vec<EdgeId> = self.edges.iter().filter(|(_, e)| !wanted.contains(&edge_key(e.v0, e.v1))).map(|(id, _)| id).collect();
        for id in stale {
            if let Some(e) = self.edges.remove(id) {
                self.edge_map.remove(&edge_key(e.v0, e.v1));
            }
        }
        // Deterministic insertion order: follow polygon order.
        let polys: Vec<PolyId> = self.polygons.keys().collect();
        for pid in polys {
            let hes: Vec<_> = self.polygons[pid].half_edges().collect();
            for (a, b) in hes {
                let k = edge_key(a, b);
                if a == b || self.edge_map.contains_key(&k) {
                    continue;
                }
                let id = self.edges.insert(Edge { v0: k.0, v1: k.1, creasing: 0.0, selected: false, hidden: false });
                self.edge_map.insert(k, id);
            }
        }
    }

    /// Removes consecutive duplicate corners and polygons with < 3 distinct corners.
    pub fn cleanup_degenerate(&mut self) {
        let mut dead = Vec::new();
        for (pid, p) in self.polygons.iter_mut() {
            p.corners.dedup_by(|a, b| a.vertex_id == b.vertex_id);
            while p.corners.len() > 1 && p.corners.first().map(|c| c.vertex_id) == p.corners.last().map(|c| c.vertex_id) {
                p.corners.pop();
            }
            let distinct: HashSet<VertexId> = p.vertex_ids().collect();
            if p.corners.len() < 3 || distinct.len() < 3 {
                dead.push(pid);
            }
        }
        for pid in dead {
            self.polygons.remove(pid);
        }
    }

    /// Deletes vertices no polygon references. Returns how many were removed.
    pub fn remove_isolated_vertices(&mut self) -> usize {
        let used: HashSet<VertexId> = self.polygons.values().flat_map(|p| p.vertex_ids()).collect();
        let dead: Vec<VertexId> = self.vertices.keys().filter(|v| !used.contains(v)).collect();
        for v in &dead {
            self.vertices.remove(*v);
        }
        dead.len()
    }

    /// Full topology refresh after an operation.
    pub fn finalize(&mut self) {
        self.cleanup_degenerate();
        self.rebuild_edges();
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        (self.vertices.len(), self.edges.len(), self.polygons.len())
    }

    pub fn bounds(&self) -> Option<Aabb> {
        Aabb::from_points(self.vertices.values().map(|v| v.position))
    }

    pub fn poly_centroid(&self, p: PolyId) -> Vec3 {
        let poly = &self.polygons[p];
        let sum: Vec3 = poly.vertex_ids().map(|v| self.pos(v)).sum();
        sum / poly.corners.len().max(1) as f32
    }

    /// Unnormalized Newell normal (length = 2 * area).
    pub fn poly_normal_raw(&self, p: PolyId) -> Vec3 {
        let poly = &self.polygons[p];
        let n = poly.corners.len();
        let mut acc = Vec3::ZERO;
        for i in 0..n {
            let a = self.pos(poly.corners[i].vertex_id);
            let b = self.pos(poly.corners[(i + 1) % n].vertex_id);
            acc += Vec3::new((a.y - b.y) * (a.z + b.z), (a.z - b.z) * (a.x + b.x), (a.x - b.x) * (a.y + b.y));
        }
        acc
    }

    pub fn poly_normal(&self, p: PolyId) -> Vec3 {
        self.poly_normal_raw(p).normalize_or(Vec3::Z)
    }

    pub fn poly_area(&self, p: PolyId) -> f32 {
        self.poly_normal_raw(p).length() * 0.5
    }

    /// Appends all geometry of `other` (transformed by `m`). Returns the new poly ids.
    pub fn append(&mut self, other: &EditablePoly, m: Mat4) -> Vec<PolyId> {
        let mut map = HashMap::new();
        for (id, v) in &other.vertices {
            let mut nv = v.clone();
            nv.position = m.transform_point3(v.position);
            map.insert(id, self.vertices.insert(nv));
        }
        let flips = m.determinant() < 0.0;
        let mut out = Vec::new();
        for p in other.polygons.values() {
            let mut np = p.clone();
            for c in &mut np.corners {
                c.vertex_id = map[&c.vertex_id];
            }
            if flips {
                np.corners.reverse();
            }
            out.push(self.polygons.insert(np));
        }
        self.rebuild_edges();
        out
    }

    pub fn transform(&mut self, m: Mat4) {
        for v in self.vertices.values_mut() {
            v.position = m.transform_point3(v.position);
        }
        if m.determinant() < 0.0 {
            for p in self.polygons.values_mut() {
                p.corners.reverse();
            }
        }
    }

    // ---- selection snapshots -------------------------------------------------

    pub fn selection_sets(&self) -> SelectionSets {
        SelectionSets {
            vertices: self.vertices.iter().filter(|(_, v)| v.selected).map(|(k, _)| k).collect(),
            edges: self.edges.values().filter(|e| e.selected).map(|e| (e.v0, e.v1)).collect(),
            polygons: self.polygons.iter().filter(|(_, p)| p.selected).map(|(k, _)| k).collect(),
        }
    }

    pub fn apply_selection_sets(&mut self, s: &SelectionSets) {
        self.clear_selection();
        for v in &s.vertices {
            if let Some(v) = self.vertices.get_mut(*v) {
                v.selected = true;
            }
        }
        for (a, b) in &s.edges {
            if let Some(e) = self.edge_between(*a, *b) {
                self.edges[e].selected = true;
            }
        }
        for p in &s.polygons {
            if let Some(p) = self.polygons.get_mut(*p) {
                p.selected = true;
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.vertices.values_mut().for_each(|v| v.selected = false);
        self.edges.values_mut().for_each(|e| e.selected = false);
        self.polygons.values_mut().for_each(|p| p.selected = false);
    }

    pub fn selected_vertices(&self) -> Vec<VertexId> {
        self.vertices.iter().filter(|(_, v)| v.selected).map(|(k, _)| k).collect()
    }
    pub fn selected_edges(&self) -> Vec<EdgeId> {
        self.edges.iter().filter(|(_, e)| e.selected).map(|(k, _)| k).collect()
    }
    pub fn selected_polygons(&self) -> Vec<PolyId> {
        self.polygons.iter().filter(|(_, p)| p.selected).map(|(k, _)| k).collect()
    }

    /// Vertices touched by the selection at the given level.
    pub fn selection_vertices(&self, level: SubObject) -> Vec<VertexId> {
        let mut set: Vec<VertexId> = match level {
            SubObject::Vertex => self.selected_vertices(),
            SubObject::Edge | SubObject::Border => {
                self.selected_edges().iter().flat_map(|&e| [self.edges[e].v0, self.edges[e].v1]).collect()
            }
            SubObject::Polygon | SubObject::Element => {
                self.selected_polygons().iter().flat_map(|&p| self.polygons[p].vertex_ids().collect::<Vec<_>>()).collect()
            }
            SubObject::Object => self.vertices.keys().collect(),
        };
        set.sort();
        set.dedup();
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn quad_plane() -> EditablePoly {
        let mut m = EditablePoly::new();
        let v: Vec<_> = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].iter().map(|&(x, y)| m.add_vertex(Vec3::new(x, y, 0.))).collect();
        m.add_polygon(&v);
        m.rebuild_edges();
        m
    }

    #[test]
    fn single_quad_counts() {
        let m = quad_plane();
        assert_eq!(m.counts(), (4, 4, 1));
        let p = m.polygons.keys().next().unwrap();
        assert!((m.poly_normal(p) - Vec3::Z).length() < 1e-6);
        assert!((m.poly_area(p) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn rebuild_preserves_edge_flags() {
        let mut m = quad_plane();
        let e = m.edges.keys().next().unwrap();
        m.edges[e].selected = true;
        m.rebuild_edges();
        assert!(m.edges[e].selected);
    }

    #[test]
    fn degenerate_cleanup_and_isolated() {
        let mut m = quad_plane();
        let extra = m.add_vertex(Vec3::ONE);
        let vs: Vec<_> = m.vertices.keys().take(2).collect();
        m.add_polygon(&[vs[0], vs[1], vs[1]]);
        m.finalize();
        assert_eq!(m.polygons.len(), 1);
        assert_eq!(m.remove_isolated_vertices(), 1);
        assert!(!m.vertices.contains_key(extra));
    }
}
