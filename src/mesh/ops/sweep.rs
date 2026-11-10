//! Region sweeps: Hinge From Edge and Extrude Along Spline extrude the
//! selected polygons through a series of rings, plus Create (polygon) and
//! the edge chains used by Create Shape from Selection.

use super::*;
use crate::mesh::adjacency::Adjacency;
use bevy::math::Quat;
use std::collections::HashSet;

/// Extrudes the selected polygon region through `steps` rings: ring `k`
/// places every boundary vertex at `f(k, original)`; the region ends at
/// ring `steps`. Vertices for which `fixed(original)` holds (on a hinge
/// axis) are shared by all rings. Returns false with nothing selected.
pub fn extrude_through(m: &mut EditablePoly, steps: u32, f: impl Fn(u32, Vec3) -> Vec3, fixed: impl Fn(Vec3) -> bool) -> bool {
    let sel = m.selected_polygons();
    if sel.is_empty() || steps == 0 {
        return false;
    }
    let sset: HashSet<PolyId> = sel.iter().copied().collect();
    let adj = Adjacency::build(m);
    let mut boundary = vec![];
    for &pid in &sel {
        for (a, b) in m.polygons[pid].half_edges() {
            if !adj.half_edge.get(&(b, a)).is_some_and(|g| sset.contains(g)) {
                boundary.push((pid, a, b));
            }
        }
    }
    let region: HashSet<VertexId> = sel.iter().flat_map(|&p| m.polygons[p].vertex_ids().collect::<Vec<_>>()).collect();
    let orig: HashMap<VertexId, Vec3> = region.iter().map(|&v| (v, m.pos(v))).collect();
    let bverts: HashSet<VertexId> = boundary.iter().flat_map(|&(_, a, b)| [a, b]).collect();
    let mut prev: HashMap<VertexId, VertexId> = bverts.iter().map(|&v| (v, v)).collect();
    for k in 1..=steps {
        let mut ring = HashMap::new();
        for &v in &bverts {
            let nv = if fixed(orig[&v]) {
                v
            } else {
                let nv = m.add_vertex(f(k, orig[&v]));
                m.vertices[nv].color = m.vertices[v].color;
                nv
            };
            ring.insert(v, nv);
        }
        for &(pid, a, b) in &boundary {
            let mut vs = vec![prev[&a], prev[&b], ring[&b], ring[&a]];
            vs.dedup();
            if vs.first() == vs.last() {
                vs.pop();
            }
            if vs.len() < 3 {
                continue;
            }
            let uvs = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];
            let template = m.polygons[pid].clone();
            m.add_polygon_corners(vs.iter().zip(uvs).map(|(&v, uv)| FaceCorner::new(v, uv)).collect(), &template);
        }
        prev = ring;
    }
    for &v in &region {
        if !bverts.contains(&v) {
            m.vertices[v].position = f(steps, orig[&v]);
        }
    }
    for &pid in &sel {
        for c in m.polygons[pid].corners.iter_mut() {
            if let Some(&nv) = prev.get(&c.vertex_id) {
                c.vertex_id = nv;
            }
        }
    }
    m.finalize();
    true
}

/// Normal and centroid of the selected polygons.
fn region_frame(m: &EditablePoly) -> (Vec3, Vec3) {
    let sel = m.selected_polygons();
    let n = sel.iter().map(|&p| m.poly_normal_raw(p)).sum::<Vec3>().normalize_or(Vec3::Z);
    let c = sel.iter().map(|&p| m.poly_centroid(p)).sum::<Vec3>() / sel.len().max(1) as f32;
    (n, c)
}

/// Hinge From Edge: swings the selected polygons `angle` degrees about the
/// hinge axis (through `origin` along `axis`) in `segments` steps. Positive
/// angles swing the region out along its normal.
pub fn hinge(m: &mut EditablePoly, origin: Vec3, axis: Vec3, angle: f32, segments: u32) -> bool {
    if axis.length_squared() < 1e-12 || m.selected_polygons().is_empty() {
        return false;
    }
    let (n, c) = region_frame(m);
    let mut d = axis.normalize();
    if d.cross(c - origin).dot(n) < 0.0 {
        d = -d;
    }
    let segs = segments.max(1);
    let on_axis = |p: Vec3| (p - origin).reject_from(d).length() < 1e-5;
    extrude_through(
        m,
        segs,
        |k, p| origin + Quat::from_axis_angle(d, (angle * k as f32 / segs as f32).to_radians()) * (p - origin),
        on_axis,
    )
}

/// Points of `path` resampled to `n + 1` evenly spaced points (by length).
fn resample(path: &[Vec3], n: u32) -> Vec<Vec3> {
    let total: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut out = vec![path[0]];
    let (mut seg, mut acc) = (0, 0.0);
    for k in 1..=n {
        let want = total * k as f32 / n as f32;
        while seg + 1 < path.len() - 1 && acc + path[seg].distance(path[seg + 1]) < want {
            acc += path[seg].distance(path[seg + 1]);
            seg += 1;
        }
        let len = path[seg].distance(path[seg + 1]).max(1e-9);
        out.push(path[seg].lerp(path[seg + 1], ((want - acc) / len).clamp(0.0, 1.0)));
    }
    out
}

/// Extrude Along Spline: sweeps the selected region along `path` (mesh
/// space) in `segments` steps, scaling the cross-section by `1 + taper * t`
/// and twisting it `twist` degrees in total. With `align` the path is turned
/// so its start follows the region normal; it always starts at the region.
pub fn extrude_along_path(m: &mut EditablePoly, path: &[Vec3], segments: u32, taper: f32, twist: f32, align: bool) -> bool {
    if path.len() < 2 || m.selected_polygons().is_empty() {
        return false;
    }
    let (n, c) = region_frame(m);
    let segs = segments.max(1);
    let pts = resample(path, segs);
    let t0 = (pts[1] - pts[0]).normalize_or(n);
    let turn = if align { Quat::from_rotation_arc(t0, n) } else { Quat::IDENTITY };
    let pts: Vec<Vec3> = pts.iter().map(|&p| c + turn * (p - pts[0])).collect();
    let tangent = |k: usize| (pts[(k + 1).min(pts.len() - 1)] - pts[k.saturating_sub(1)]).normalize_or(n);
    // Parallel-transported frames along the path.
    let mut frames = vec![Quat::IDENTITY];
    for k in 1..pts.len() {
        let r = Quat::from_rotation_arc(tangent(k - 1), tangent(k)) * frames[k - 1];
        frames.push(r.normalize());
    }
    let start = tangent(0);
    extrude_through(
        m,
        segs,
        |k, p| {
            let t = k as f32 / segs as f32;
            let q = p - c;
            let along = start * q.dot(start);
            let section = (q - along) * (1.0 + taper * t).max(0.01) + along;
            let tw = Quat::from_axis_angle(tangent(k as usize), (twist * t).to_radians());
            pts[k as usize] + tw * (frames[k as usize] * section)
        },
        |_| false,
    )
}

/// Create (polygon): builds a polygon through existing and new points,
/// wound so its normal faces away from `view_dir`. Returns false for fewer
/// than three distinct points.
pub fn create_polygon(m: &mut EditablePoly, points: &[CreatePoint], view_dir: Vec3) -> bool {
    let mut vs: Vec<VertexId> = vec![];
    for p in points {
        let v = match *p {
            CreatePoint::Vertex(v) if m.vertices.contains_key(v) => v,
            CreatePoint::Vertex(_) => return false,
            CreatePoint::New(pos) => m.add_vertex(pos),
        };
        if !vs.contains(&v) {
            vs.push(v);
        }
    }
    if vs.len() < 3 {
        return false;
    }
    let ps: Vec<Vec3> = vs.iter().map(|&v| m.pos(v)).collect();
    let normal: Vec3 = (0..ps.len()).map(|i| ps[i].cross(ps[(i + 1) % ps.len()])).sum();
    if normal.dot(view_dir) > 0.0 {
        vs.reverse();
    }
    m.clear_selection();
    let pid = m.add_polygon(&vs);
    m.polygons[pid].selected = true;
    m.finalize();
    true
}

/// A Create click: an existing vertex or a new point (mesh space).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CreatePoint {
    Vertex(VertexId),
    New(Vec3),
}

/// Selected edges joined into chains of positions (closed chains do not
/// repeat their first point), for Create Shape from Selection.
pub fn selected_edge_chains(m: &EditablePoly) -> Vec<(Vec<Vec3>, bool)> {
    let mut left: Vec<(VertexId, VertexId)> = m.selected_edges().iter().map(|&e| (m.edges[e].v0, m.edges[e].v1)).collect();
    let degree = |l: &[(VertexId, VertexId)], v: VertexId| l.iter().filter(|&&(a, b)| a == v || b == v).count();
    let mut out = vec![];
    while !left.is_empty() {
        // Start open chains at an end so they come out whole.
        let i = left.iter().position(|&(a, b)| degree(&left, a) == 1 || degree(&left, b) == 1).unwrap_or(0);
        let (a, b) = left.remove(i);
        let (mut chain, mut tip) = if degree(&left, a) == 0 && degree(&left, b) > 0 { (vec![a, b], b) } else { (vec![b, a], a) };
        while let Some(j) = left.iter().position(|&(x, y)| x == tip || y == tip) {
            let (x, y) = left.remove(j);
            tip = if x == tip { y } else { x };
            chain.push(tip);
        }
        let closed = chain.len() > 3 && chain.first() == chain.last();
        if closed {
            chain.pop();
        }
        out.push((chain.iter().map(|&v| m.pos(v)).collect(), closed));
    }
    out
}
