//! Texture projection paint: brush stamps are evaluated per texel in world
//! space (each triangle is rasterized in UV space only over the brush's
//! footprint), so strokes cross UV seams cleanly.

use super::{Brush, BrushTool};
use crate::materials::Texture;
use crate::mesh::EditablePoly;
use crate::mesh::triangulate::triangulate_polygon;
use bevy::math::{Mat2, Mat4, Vec2, Vec3};

#[derive(Clone, Debug)]
pub struct PaintTri {
    /// World-space corners.
    pub p: [Vec3; 3],
    pub uv: [Vec2; 3],
    pub normal: Vec3,
}

/// World-space triangles of `mesh` with their corner UVs.
pub fn paint_tris(mesh: &EditablePoly, world: Mat4, selected_only: bool) -> Vec<PaintTri> {
    let mut out = vec![];
    for (_, poly) in mesh.polygons.iter().filter(|(_, p)| !selected_only || p.selected) {
        let pts: Vec<Vec3> = poly.vertex_ids().map(|v| world.transform_point3(mesh.pos(v))).collect();
        for [a, b, c] in triangulate_polygon(&pts) {
            let p = [pts[a], pts[b], pts[c]];
            let normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            out.push(PaintTri { p, uv: [poly.corners[a].uv, poly.corners[b].uv, poly.corners[c].uv], normal });
        }
    }
    out
}

/// Pixel-space position of a UV (row 0 = top, v = 1), unwrapped.
fn px(t: &Texture, uv: Vec2) -> Vec2 {
    Vec2::new(uv.x * t.width as f32, (1.0 - uv.y) * t.height as f32)
}

/// Signed distances (pixels) of `q` to the three edges, positive inside.
fn edge_dist(a: Vec2, b: Vec2, c: Vec2, q: Vec2) -> Option<[f32; 4]> {
    let area = (b - a).perp_dot(c - a);
    if area.abs() < 1e-9 {
        return None;
    }
    let s = area.signum();
    let d = |p0: Vec2, p1: Vec2| s * (p1 - p0).perp_dot(q - p0) / (p1 - p0).length().max(1e-9);
    // Barycentric weight of a, b (c = rest).
    let wa = (b - q).perp_dot(c - q) / area;
    let wb = (c - q).perp_dot(a - q) / area;
    Some([d(a, b).min(d(b, c)).min(d(c, a)), wa, wb, 1.0 - wa - wb])
}

/// Fraction of covered texels claimed by more than one triangle (1.0 when
/// the mesh has no usable UVs). Painting needs a non-overlapping layout.
pub fn uv_overlap(mesh: &EditablePoly) -> f32 {
    const N: u32 = 128;
    let probe = Texture { width: N, height: N, data: vec![] };
    let mut count = vec![0u8; (N * N) as usize];
    for t in paint_tris(mesh, Mat4::IDENTITY, false) {
        let [a, b, c] = t.uv.map(|u| px(&probe, u));
        let (lo, hi) = (a.min(b).min(c).floor(), a.max(b).max(c).ceil());
        for y in lo.y as i64..hi.y as i64 {
            for x in lo.x as i64..hi.x as i64 {
                let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if edge_dist(a, b, c, q).is_some_and(|e| e[0] > 1e-4) {
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

pub struct TextureStroke {
    tris: Vec<PaintTri>,
    /// Canvas at stroke start + per-texel stroke coverage.
    pre: Vec<[u8; 4]>,
    cov: Vec<f32>,
}

fn mix(a: [u8; 4], c: [f32; 3], w: f32) -> [u8; 4] {
    let ch = |i: usize| (a[i] as f32 + (c[i] * 255.0 - a[i] as f32) * w).round().clamp(0.0, 255.0) as u8;
    [ch(0), ch(1), ch(2), (a[3] as f32 + (255.0 - a[3] as f32) * w).round() as u8]
}

impl TextureStroke {
    pub fn new(tris: Vec<PaintTri>, tex: &Texture) -> Self {
        Self { tris, pre: tex.data.clone(), cov: vec![0.0; tex.data.len()] }
    }

    /// One stamp at `center`; triangles facing away from `view_dir` are
    /// skipped (pass `Vec3::ZERO` to paint both sides). Returns texels hit.
    pub fn stamp(&mut self, tex: &mut Texture, center: Vec3, view_dir: Vec3, brush: &Brush) -> usize {
        let r = brush.size;
        let (w, h) = (tex.width as i64, tex.height as i64);
        let mut writes: Vec<(usize, f32)> = vec![];
        for t in &self.tris {
            if view_dir != Vec3::ZERO && t.normal.dot(view_dir) > 0.0 {
                continue;
            }
            let (lo, hi) = (t.p[0].min(t.p[1]).min(t.p[2]), t.p[0].max(t.p[1]).max(t.p[2]));
            if center.clamp(lo, hi).distance(center) > r {
                continue;
            }
            let [pa, pb, pc] = t.uv.map(|u| px(tex, u));
            // Affine world -> pixel map on the triangle's plane.
            let (e1, e2) = (t.p[1] - t.p[0], t.p[2] - t.p[0]);
            let g = Mat2::from_cols_array(&[e1.dot(e1), e1.dot(e2), e1.dot(e2), e2.dot(e2)]);
            if g.determinant().abs() < 1e-12 {
                continue;
            }
            let gi = g.inverse();
            let (d1, d2) = (pb - pa, pc - pa);
            let rel = center - t.p[0];
            let st = gi * Vec2::new(e1.dot(rel), e2.dot(rel));
            let cpx = pa + d1 * st.x + d2 * st.y;
            let kx = gi * Vec2::new(d1.x, d2.x);
            let ky = gi * Vec2::new(d1.y, d2.y);
            let ext = Vec2::new((e1 * kx.x + e2 * kx.y).length(), (e1 * ky.x + e2 * ky.y).length()) * r + Vec2::splat(1.0);
            let tri_lo = pa.min(pb).min(pc) - Vec2::ONE;
            let tri_hi = pa.max(pb).max(pc) + Vec2::ONE;
            let lo = (cpx - ext).max(tri_lo).floor();
            let hi = (cpx + ext).min(tri_hi).ceil();
            if (hi.x - lo.x) * (hi.y - lo.y) > 16.0e6 {
                continue;
            }
            for y in lo.y as i64..hi.y as i64 {
                for x in lo.x as i64..hi.x as i64 {
                    let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let Some([d, wa, wb, wc]) = edge_dist(pa, pb, pc, q) else { continue };
                    // Dilate ~0.75 px past the edges so seams leave no gaps.
                    if d < -0.75 {
                        continue;
                    }
                    let p = t.p[0] * wa + t.p[1] * wb + t.p[2] * wc;
                    let wt = brush.weight(p.distance(center));
                    if wt > 0.0 {
                        writes.push(((y.rem_euclid(h) * w + x.rem_euclid(w)) as usize, wt));
                    }
                }
            }
        }
        match brush.tool {
            BrushTool::Paint => {
                for &(i, wt) in &writes {
                    if wt > self.cov[i] {
                        self.cov[i] = wt;
                        tex.data[i] = mix(self.pre[i], brush.color, wt);
                    }
                }
            }
            BrushTool::Blur => {
                let blurred: Vec<(usize, [u8; 4])> = writes
                    .iter()
                    .map(|&(i, wt)| {
                        let (x, y) = ((i as i64) % w, (i as i64) / w);
                        let mut acc = [0.0f32; 3];
                        for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                            let s = tex.texel(x + dx, y + dy);
                            (0..3).for_each(|c| acc[c] += s[c] as f32 / (255.0 * 9.0));
                        }
                        (i, mix(tex.data[i], acc, wt * 0.5))
                    })
                    .collect();
                for (i, c) in blurred {
                    tex.data[i] = c;
                }
            }
            // Canvases have no layer to erase; the PaintBox disables it.
            BrushTool::Erase => {}
        }
        writes.len()
    }
}

/// PaintBox "Fill Object" for canvases: floods every texel.
pub fn fill_texture(tex: &mut Texture, color: [f32; 3], opacity: f32) {
    for t in tex.data.iter_mut() {
        *t = mix(*t, color, opacity.clamp(0.0, 1.0));
    }
}
