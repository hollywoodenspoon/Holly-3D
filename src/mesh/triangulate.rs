//! N-gon triangulation (ear clipping) and render export.

use super::*;

/// GPU-ready triangle soup with split per-corner attributes.
#[derive(Clone, Debug, Default)]
pub struct TriMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    /// Source polygon of each triangle (for picking).
    pub tri_poly: Vec<PolyId>,
}

/// Triangulates a polygon given its 3D points; returns index triples into `pts`.
pub fn triangulate_polygon(pts: &[Vec3]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 3 {
        return vec![];
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }
    // Newell normal to choose a projection plane.
    let mut nrm = Vec3::ZERO;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        nrm += Vec3::new((a.y - b.y) * (a.z + b.z), (a.z - b.z) * (a.x + b.x), (a.x - b.x) * (a.y + b.y));
    }
    let nrm = nrm.normalize_or(Vec3::Z);
    let u = nrm.any_orthonormal_vector();
    let v = nrm.cross(u);
    let p2: Vec<(f32, f32)> = pts.iter().map(|p| (p.dot(u), p.dot(v))).collect();
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let inside = |p: (f32, f32), a, b, c| cross(a, b, p) >= -1e-9 && cross(b, c, p) >= -1e-9 && cross(c, a, p) >= -1e-9;

    let mut idx: Vec<usize> = (0..n).collect();
    let mut tris = Vec::with_capacity(n - 2);
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (p2[ia], p2[ib], p2[ic]);
            if cross(a, b, c) <= 1e-12 {
                continue; // reflex or degenerate
            }
            let blocked = idx.iter().any(|&j| j != ia && j != ib && j != ic && inside(p2[j], a, b, c));
            if !blocked {
                tris.push([ia, ib, ic]);
                idx.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        tris.push([idx[0], idx[1], idx[2]]);
    } else if idx.len() > 3 {
        // Fallback: fan the remainder (degenerate/self-intersecting input).
        for k in 1..idx.len() - 1 {
            tris.push([idx[0], idx[k], idx[k + 1]]);
        }
    }
    tris
}

impl EditablePoly {
    /// Triangulated, corner-split render mesh honoring smoothing groups.
    pub fn to_tri_mesh(&self) -> TriMesh {
        let normals = self.corner_normals();
        let mut t = TriMesh::default();
        for (pid, p) in &self.polygons {
            let pts: Vec<Vec3> = p.vertex_ids().map(|v| self.pos(v)).collect();
            let base = t.positions.len() as u32;
            let ns = &normals[&pid];
            for (i, c) in p.corners.iter().enumerate() {
                t.positions.push(pts[i].to_array());
                t.normals.push(ns[i].to_array());
                t.uvs.push([c.uv.x, 1.0 - c.uv.y]);
                t.colors.push(self.vertices[c.vertex_id].color.to_array());
            }
            for tri in triangulate_polygon(&pts) {
                t.indices.extend(tri.iter().map(|&k| base + k as u32));
                t.tri_poly.push(pid);
            }
        }
        t
    }

    /// Triangles of a single polygon as world-space-independent points.
    pub fn poly_triangles(&self, pid: PolyId) -> Vec<[Vec3; 3]> {
        let pts: Vec<Vec3> = self.polygons[pid].vertex_ids().map(|v| self.pos(v)).collect();
        triangulate_polygon(&pts).into_iter().map(|[a, b, c]| [pts[a], pts[b], pts[c]]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concave_l_shape_triangulates() {
        let pts: Vec<Vec3> = [(0., 0.), (2., 0.), (2., 1.), (1., 1.), (1., 2.), (0., 2.)]
            .iter()
            .map(|&(x, y)| Vec3::new(x, y, 0.))
            .collect();
        let tris = triangulate_polygon(&pts);
        assert_eq!(tris.len(), 4);
        let area: f32 = tris
            .iter()
            .map(|t| (pts[t[1]] - pts[t[0]]).cross(pts[t[2]] - pts[t[0]]).z * 0.5)
            .sum();
        assert!((area - 3.0).abs() < 1e-5, "area {area}");
    }

    #[test]
    fn quad_export_has_valid_attributes() {
        let mut m = EditablePoly::new();
        let v: Vec<_> = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].iter().map(|&(x, y)| m.add_vertex(Vec3::new(x, y, 0.))).collect();
        m.add_polygon(&v);
        m.rebuild_edges();
        let t = m.to_tri_mesh();
        assert_eq!(t.positions.len(), 4);
        assert_eq!(t.indices.len(), 6);
        assert!(t.normals.iter().all(|n| (Vec3::from_array(*n) - Vec3::Z).length() < 1e-5));
    }
}
