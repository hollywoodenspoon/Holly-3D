//! UV-space rasterizing shared by the brush engine, the mesh map baker and
//! the viewport composite: world-space triangles with their UVs, and a
//! walk over the texels each one covers.

use crate::mesh::EditablePoly;
use crate::mesh::triangulate::triangulate_polygon;
use bevy::math::{Mat3, Mat4, Vec2, Vec3};

#[derive(Clone, Debug)]
pub struct PaintTri {
    /// World-space corners.
    pub p: [Vec3; 3],
    /// Smoothed corner normals (world).
    pub n: [Vec3; 3],
    pub uv: [Vec2; 3],
    /// Face normal (world).
    pub normal: Vec3,
    /// Polygon material ID.
    pub id: u16,
}

/// World-space triangles of `mesh` with their corner UVs.
pub fn paint_tris(mesh: &EditablePoly, world: Mat4, selected_only: bool) -> Vec<PaintTri> {
    let mut out = vec![];
    let normals = mesh.corner_normals();
    let nm = Mat3::from_mat4(world).inverse().transpose();
    for (pid, poly) in mesh.polygons.iter().filter(|(_, p)| !selected_only || p.selected) {
        let pts: Vec<Vec3> = poly.vertex_ids().map(|v| world.transform_point3(mesh.pos(v))).collect();
        let ns = &normals[&pid];
        for [a, b, c] in triangulate_polygon(&pts) {
            let p = [pts[a], pts[b], pts[c]];
            let normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            let n = [a, b, c].map(|i| (nm * ns[i]).normalize_or(normal));
            out.push(PaintTri { p, n, uv: [poly.corners[a].uv, poly.corners[b].uv, poly.corners[c].uv], normal, id: poly.material_id });
        }
    }
    out
}

/// Pixel-space position of a UV (row 0 = top, v = 1), unwrapped.
pub fn px(size: u32, uv: Vec2) -> Vec2 {
    Vec2::new(uv.x * size as f32, (1.0 - uv.y) * size as f32)
}

/// Distance (pixels) of `q` inside the triangle (negative outside) and
/// its barycentric weights.
pub fn edge_dist(a: Vec2, b: Vec2, c: Vec2, q: Vec2) -> Option<(f32, [f32; 3])> {
    let area = (b - a).perp_dot(c - a);
    if area.abs() < 1e-9 {
        return None;
    }
    let s = area.signum();
    let d = |p0: Vec2, p1: Vec2| s * (p1 - p0).perp_dot(q - p0) / (p1 - p0).length().max(1e-9);
    let wa = (b - q).perp_dot(c - q) / area;
    let wb = (c - q).perp_dot(a - q) / area;
    Some((d(a, b).min(d(b, c)).min(d(c, a)), [wa, wb, 1.0 - wa - wb]))
}

/// Texels a triangle paints: those inside it, plus a 0.75 px rim so UV
/// seams leave no gaps.
pub const SEAM_PAD: f32 = 0.75;

/// Visits every texel of a `size` x `size` image covered by `t` (clipped
/// to the pixel box `lo..hi` when given) with its wrapped index, its
/// barycentric weights and whether it lies truly inside.
pub fn raster_tri(t: &PaintTri, size: u32, clip: Option<(Vec2, Vec2)>, mut f: impl FnMut(usize, [f32; 3], bool)) {
    let [a, b, c] = t.uv.map(|u| px(size, u));
    let mut lo = a.min(b).min(c) - Vec2::ONE;
    let mut hi = a.max(b).max(c) + Vec2::ONE;
    if let Some((clo, chi)) = clip {
        (lo, hi) = (lo.max(clo), hi.min(chi));
    }
    let (lo, hi) = (lo.floor(), hi.ceil());
    if (hi.x - lo.x) * (hi.y - lo.y) > 64.0e6 {
        return;
    }
    let n = size as i64;
    for y in lo.y as i64..hi.y as i64 {
        for x in lo.x as i64..hi.x as i64 {
            let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if let Some((d, w)) = edge_dist(a, b, c, q)
                && d >= -SEAM_PAD
            {
                f((y.rem_euclid(n) * n + x.rem_euclid(n)) as usize, w, d >= 0.0);
            }
        }
    }
}

/// Fraction of covered texels claimed by more than one triangle (1.0 when
/// the mesh has no usable UVs). Painting needs a non-overlapping layout.
pub fn uv_overlap(mesh: &EditablePoly) -> f32 {
    const N: u32 = 128;
    let mut count = vec![0u8; (N * N) as usize];
    for t in paint_tris(mesh, Mat4::IDENTITY, false) {
        let [a, b, c] = t.uv.map(|u| px(N, u));
        let (lo, hi) = (a.min(b).min(c).floor(), a.max(b).max(c).ceil());
        for y in lo.y as i64..hi.y as i64 {
            for x in lo.x as i64..hi.x as i64 {
                let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if edge_dist(a, b, c, q).is_some_and(|e| e.0 > 1e-4) {
                    let i = (y.rem_euclid(N as i64) * N as i64 + x.rem_euclid(N as i64)) as usize;
                    count[i] = count[i].saturating_add(1);
                }
            }
        }
    }
    let covered = count.iter().filter(|&&c| c > 0).count();
    if covered == 0 {
        return 1.0;
    }
    count.iter().filter(|&&c| c > 1).count() as f32 / covered as f32
}

/// True when the mesh has UVs of its own: any polygon whose corners do
/// not all share one UV.
pub fn has_uvs(mesh: &EditablePoly) -> bool {
    mesh.polygons.values().any(|p| p.corners.iter().any(|c| c.uv != p.corners[0].uv))
}
