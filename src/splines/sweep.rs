//! Rendering rollout (Enable In Renderer / Viewport, Radial vs Rectangular)
//! and the shared `loft` grid builder used by the sweep, Lathe and Extrude.

use super::{Polyline, Shape};
use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Quat, Vec2, Vec3};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct SplineRender {
    pub renderer: bool,
    pub viewport: bool,
    pub rectangular: bool,
    pub thickness: f32,
    pub sides: u32,
    pub angle: f32,
    pub length: f32,
    pub width: f32,
    pub rect_angle: f32,
}

impl Default for SplineRender {
    fn default() -> Self {
        Self {
            renderer: false,
            viewport: false,
            rectangular: false,
            thickness: 0.05,
            sides: 12,
            angle: 0.0,
            length: 0.1,
            width: 0.05,
            rect_angle: 0.0,
        }
    }
}

impl SplineRender {
    /// Cross-section points in the (normal, binormal) plane, counter-clockwise.
    fn profile(&self) -> Vec<Vec2> {
        if self.rectangular {
            let (hx, hy) = (self.width * 0.5, self.length * 0.5);
            let rot = Vec2::from_angle(self.rect_angle.to_radians());
            [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)].map(|(x, y)| rot.rotate(Vec2::new(x, y))).to_vec()
        } else {
            let n = self.sides.max(3);
            let r = self.thickness * 0.5;
            (0..n).map(|i| Vec2::from_angle(i as f32 / n as f32 * std::f32::consts::TAU + self.angle.to_radians()) * r).collect()
        }
    }

    /// Sweeps the profile along every spline (open ends are capped).
    pub fn sweep(&self, shape: &Shape) -> EditablePoly {
        let mut out = EditablePoly::new();
        let profile = self.profile();
        for pl in shape.polylines() {
            if pl.pts.len() < 2 || pl.length() < 1e-6 {
                continue;
            }
            let rows = sweep_rows(&pl, &profile);
            let (mut m, ids) = loft(&rows, true, pl.closed, false);
            if !pl.closed {
                let first: Vec<VertexId> = ids[0].iter().rev().copied().collect();
                cap(&mut m, &first);
                cap(&mut m, ids.last().unwrap());
            }
            m.finalize();
            out.append(&m, bevy::math::Mat4::IDENTITY);
        }
        out.finalize();
        out
    }
}

/// One profile ring per polyline point, using parallel-transport frames and
/// mitered corners.
fn sweep_rows(pl: &Polyline, profile: &[Vec2]) -> Vec<Vec<Vec3>> {
    let n = pl.pts.len();
    let p = &pl.pts;
    let seg = |i: usize| (p[(i + 1) % n] - p[i]).normalize_or(Vec3::X);
    let tangents: Vec<(Vec3, Vec3)> = (0..n)
        .map(|i| {
            let out = if i + 1 < n || pl.closed { seg(i) } else { seg(i - 1) };
            let inc = if i > 0 {
                seg(i - 1)
            } else if pl.closed {
                seg(n - 1)
            } else {
                out
            };
            (inc, out)
        })
        .collect();
    let t0 = (tangents[0].0 + tangents[0].1).normalize_or(tangents[0].1);
    let up = if t0.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let mut normal = (up - t0 * up.dot(t0)).normalize();
    let mut prev_t = t0;
    let mut rows = vec![];
    for (i, &(inc, out)) in tangents.iter().enumerate() {
        let t = (inc + out).normalize_or(out);
        normal = Quat::from_rotation_arc(prev_t, t) * normal;
        normal = (normal - t * normal.dot(t)).normalize_or(normal);
        prev_t = t;
        let b = t.cross(normal);
        let c = t.dot(out).clamp(0.2, 1.0);
        let miter = (out - inc).normalize_or_zero();
        let row = profile
            .iter()
            .map(|q| {
                let o = normal * q.x + b * q.y;
                p[i] + o + miter * o.dot(miter) * (1.0 / c - 1.0)
            })
            .collect();
        rows.push(row);
    }
    rows
}

/// Builds a quad grid from `rows` (equal-length point rows). Columns run
/// along each row; `wrap_cols` / `wrap_rows` close the grid. With `weld`,
/// coincident points share a vertex (Lathe Weld Core). Returns the mesh and
/// the vertex id of every grid point.
pub fn loft(rows: &[Vec<Vec3>], wrap_cols: bool, wrap_rows: bool, weld: bool) -> (EditablePoly, Vec<Vec<VertexId>>) {
    let mut m = EditablePoly::new();
    let mut seen: HashMap<[i64; 3], VertexId> = HashMap::new();
    let ids: Vec<Vec<VertexId>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&p| {
                    if !weld {
                        return m.add_vertex(p);
                    }
                    let key = (p * 1e4).round().as_i64vec3().to_array();
                    *seen.entry(key).or_insert_with(|| m.add_vertex(p))
                })
                .collect()
        })
        .collect();
    let (nr, nc) = (rows.len(), rows.first().map_or(0, |r| r.len()));
    let (qr, qc) = (if wrap_rows { nr } else { nr.saturating_sub(1) }, if wrap_cols { nc } else { nc.saturating_sub(1) });
    for r in 0..qr {
        for c in 0..qc {
            let (r1, c1) = ((r + 1) % nr, (c + 1) % nc);
            let verts = [ids[r][c], ids[r][c1], ids[r1][c1], ids[r1][c]];
            let uv = |rr: usize, cc: usize| Vec2::new(cc as f32 / qc.max(1) as f32, rr as f32 / qr.max(1) as f32);
            let uvs = [uv(r, c), uv(r, c + 1), uv(r + 1, c + 1), uv(r + 1, c)];
            let mut vs: Vec<VertexId> = vec![];
            let mut us: Vec<Vec2> = vec![];
            for (v, u) in verts.into_iter().zip(uvs) {
                if vs.last() != Some(&v) && !(vs.len() == 3 && vs[0] == v) {
                    vs.push(v);
                    us.push(u);
                }
            }
            if vs.len() >= 3 {
                let p = m.add_polygon_uv(&vs, &us);
                m.polygons[p].smoothing_group = 1;
            }
        }
    }
    m.finalize();
    (m, ids)
}

/// Adds an N-gon cap over `ids` (planar-projected UVs), ignoring duplicates.
pub fn cap(m: &mut EditablePoly, ids: &[VertexId]) {
    let mut vs: Vec<VertexId> = vec![];
    for &v in ids {
        if !vs.contains(&v) {
            vs.push(v);
        }
    }
    if vs.len() < 3 {
        return;
    }
    let pts: Vec<Vec3> = vs.iter().map(|&v| m.pos(v)).collect();
    let b = crate::mesh::Aabb::from_points(pts.iter().copied()).unwrap();
    let s = b.size().max(Vec3::splat(1e-6));
    let uvs: Vec<Vec2> = pts.iter().map(|p| ((*p - b.min) / s).truncate()).collect();
    let p = m.add_polygon_uv(&vs, &uvs);
    m.polygons[p].smoothing_group = 2;
    m.rebuild_edges();
}

/// Reverses every polygon's winding (Flip Normals).
pub fn flip(m: &mut EditablePoly) {
    for p in m.polygons.values_mut() {
        p.corners.reverse();
    }
}
