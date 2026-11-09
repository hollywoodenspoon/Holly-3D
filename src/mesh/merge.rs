//! Coplanar polygon merging: rebuilds n-gons from triangle soups (Boolean
//! results) so flat regions stay clean, editable polygons.

use super::adjacency::Adjacency;
use super::*;
use std::collections::HashSet;

impl EditablePoly {
    /// Merges edge-connected polygons that share material, smoothing group and
    /// plane (normals within `cos_tol`) into one polygon per region, when the
    /// region's outline is a single simple loop (regions with holes stay split).
    /// Returns how many polygons were removed.
    pub fn merge_coplanar(&mut self, cos_tol: f32) -> usize {
        let adj = Adjacency::build(self);
        let normals: HashMap<PolyId, Vec3> = self.polygons.keys().map(|p| (p, self.poly_normal(p))).collect();
        let mut group: HashMap<PolyId, usize> = HashMap::new();
        let mut groups: Vec<Vec<PolyId>> = vec![];
        let keys: Vec<PolyId> = self.polygons.keys().collect();
        for &start in &keys {
            if group.contains_key(&start) {
                continue;
            }
            let gi = groups.len();
            group.insert(start, gi);
            let (mat, sg) = (self.polygons[start].material_id, self.polygons[start].smoothing_group);
            let mut stack = vec![start];
            let mut members = vec![];
            while let Some(f) = stack.pop() {
                members.push(f);
                for (a, b) in self.polygons[f].half_edges() {
                    let fs = adj.faces_of_edge(a, b);
                    if fs.len() != 2 {
                        continue;
                    }
                    for &g in fs {
                        let pg = &self.polygons[g];
                        if g != f
                            && !group.contains_key(&g)
                            && pg.material_id == mat
                            && pg.smoothing_group == sg
                            && normals[&g].dot(normals[&start]) >= cos_tol
                        {
                            group.insert(g, gi);
                            stack.push(g);
                        }
                    }
                }
            }
            groups.push(members);
        }
        let mut removed = 0;
        for members in groups.into_iter().filter(|g| g.len() > 1) {
            let Some(corners) = self.outline(&members) else { continue };
            if corners.len() < 3 {
                continue;
            }
            let mut template = self.polygons[members[0]].clone();
            template.selected = members.iter().any(|&p| self.polygons[p].selected);
            for &p in &members {
                self.polygons.remove(p);
            }
            self.add_polygon_corners(corners, &template);
            removed += members.len() - 1;
        }
        if removed > 0 {
            self.remove_isolated_vertices();
            self.finalize();
        }
        removed
    }

    /// The single boundary loop of a polygon region (corners carry their UVs),
    /// or `None` when the region has holes or pinched vertices.
    fn outline(&self, members: &[PolyId]) -> Option<Vec<FaceCorner>> {
        let mut half: HashSet<(VertexId, VertexId)> = HashSet::new();
        for &p in members {
            half.extend(self.polygons[p].half_edges());
        }
        let mut next: HashMap<VertexId, (VertexId, FaceCorner)> = HashMap::new();
        for &p in members {
            let poly = &self.polygons[p];
            for (i, (a, b)) in poly.half_edges().enumerate() {
                if half.contains(&(b, a)) {
                    continue;
                }
                if next.insert(a, (b, poly.corners[i].clone())).is_some() {
                    return None;
                }
            }
        }
        let &start = next.keys().next()?;
        let mut out = vec![];
        let mut v = start;
        loop {
            let (n, c) = next.get(&v)?;
            out.push(c.clone());
            v = *n;
            if v == start {
                break;
            }
            if out.len() > next.len() {
                return None;
            }
        }
        (out.len() == next.len()).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangulated_grid_merges_back_to_one_quad() {
        let mut m = EditablePoly::new();
        let v: Vec<_> = (0..9).map(|i| m.add_vertex(Vec3::new((i % 3) as f32, (i / 3) as f32, 0.0))).collect();
        for y in 0..2 {
            for x in 0..2 {
                let (a, b, c, d) = (v[y * 3 + x], v[y * 3 + x + 1], v[y * 3 + x + 4], v[y * 3 + x + 3]);
                m.add_polygon(&[a, b, c]);
                m.add_polygon(&[a, c, d]);
            }
        }
        m.finalize();
        assert_eq!(m.merge_coplanar(0.9999), 7);
        m.validate().unwrap();
        // the centre vertex becomes isolated and is removed; edge midpoints stay
        assert_eq!(m.counts(), (8, 8, 1));
    }

    #[test]
    fn ring_with_hole_stays_split() {
        let mut m = EditablePoly::new();
        let outer: Vec<_> = [(0., 0.), (3., 0.), (3., 3.), (0., 3.)].iter().map(|&(x, y)| m.add_vertex(Vec3::new(x, y, 0.))).collect();
        let inner: Vec<_> = [(1., 1.), (2., 1.), (2., 2.), (1., 2.)].iter().map(|&(x, y)| m.add_vertex(Vec3::new(x, y, 0.))).collect();
        for i in 0..4 {
            let j = (i + 1) % 4;
            m.add_polygon(&[outer[i], outer[j], inner[j], inner[i]]);
        }
        m.finalize();
        assert_eq!(m.merge_coplanar(0.9999), 0);
        assert_eq!(m.polygons.len(), 4);
    }
}
