//! Smoothing-group driven normals.

use super::adjacency::Adjacency;
use super::*;

impl EditablePoly {
    /// Per-corner normals for every polygon (same order as `polygons.iter()`).
    ///
    /// A corner normal is its explicit normal if set, otherwise the area-weighted
    /// average of all faces around that vertex that share at least one smoothing
    /// group bit with this face. Group 0 means faceted (flat).
    pub fn corner_normals(&self) -> HashMap<PolyId, Vec<Vec3>> {
        let adj = Adjacency::build(self);
        let face_n: HashMap<PolyId, Vec3> = self.polygons.keys().map(|p| (p, self.poly_normal_raw(p))).collect();
        let mut out = HashMap::new();
        for (pid, p) in &self.polygons {
            let own = face_n[&pid];
            let ns = p
                .corners
                .iter()
                .map(|c| {
                    if let Some(n) = c.explicit_normal {
                        return n.normalize_or(Vec3::Z);
                    }
                    if p.smoothing_group == 0 {
                        return own.normalize_or(Vec3::Z);
                    }
                    let mut acc = Vec3::ZERO;
                    for &f in adj.faces_of_vertex(c.vertex_id) {
                        if f == pid || self.polygons[f].smoothing_group & p.smoothing_group != 0 {
                            acc += face_n[&f];
                        }
                    }
                    acc.normalize_or(own.normalize_or(Vec3::Z))
                })
                .collect();
            out.insert(pid, ns);
        }
        out
    }

    /// Averaged vertex normal over all adjacent faces (used for Push, Local Normal extrude…).
    pub fn vertex_normals(&self) -> HashMap<VertexId, Vec3> {
        let mut acc: HashMap<VertexId, Vec3> = HashMap::new();
        for pid in self.polygons.keys() {
            let n = self.poly_normal_raw(pid);
            for v in self.polygons[pid].vertex_ids() {
                *acc.entry(v).or_default() += n;
            }
        }
        acc.into_iter().map(|(k, v)| (k, v.normalize_or(Vec3::Z))).collect()
    }

    /// "Auto Smooth": faces whose shared-edge angle is below `threshold_deg`
    /// end up sharing a smoothing group; sharper neighbours never share one.
    /// Only affects `targets` (all polygons when empty).
    pub fn auto_smooth(&mut self, targets: &[PolyId], threshold_deg: f32) {
        let adj = Adjacency::build(self);
        let targets: Vec<PolyId> = if targets.is_empty() { self.polygons.keys().collect() } else { targets.to_vec() };
        let tset: std::collections::HashSet<PolyId> = targets.iter().copied().collect();
        let cos_t = threshold_deg.to_radians().cos();
        let normals: HashMap<PolyId, Vec3> = self.polygons.keys().map(|p| (p, self.poly_normal(p))).collect();
        // 1. cluster faces connected by smooth edges
        let mut cluster: HashMap<PolyId, usize> = HashMap::new();
        let mut clusters: Vec<Vec<PolyId>> = Vec::new();
        for &start in &targets {
            if cluster.contains_key(&start) {
                continue;
            }
            let id = clusters.len();
            let mut stack = vec![start];
            cluster.insert(start, id);
            let mut members = vec![];
            while let Some(f) = stack.pop() {
                members.push(f);
                for (a, b) in self.polygons[f].half_edges() {
                    for &g in adj.faces_of_edge(a, b) {
                        if g != f && tset.contains(&g) && !cluster.contains_key(&g) && normals[&f].dot(normals[&g]) >= cos_t {
                            cluster.insert(g, id);
                            stack.push(g);
                        }
                    }
                }
            }
            clusters.push(members);
        }
        // 2. greedy-colour clusters with 32 bits so neighbouring clusters differ
        let mut color: Vec<u32> = vec![0; clusters.len()];
        for (ci, members) in clusters.iter().enumerate() {
            let mut used = 0u32;
            for &f in members {
                for (a, b) in self.polygons[f].half_edges() {
                    for &g in adj.faces_of_edge(a, b) {
                        match cluster.get(&g) {
                            Some(&cg) if cg != ci => used |= color[cg],
                            None if g != f => used |= self.polygons[g].smoothing_group,
                            _ => {}
                        }
                    }
                }
            }
            let bit = (0..32).find(|b| used & (1 << b) == 0).unwrap_or(0);
            color[ci] = 1 << bit;
        }
        for (ci, members) in clusters.iter().enumerate() {
            for &f in members {
                self.polygons[f].smoothing_group = color[ci];
            }
        }
    }
}
