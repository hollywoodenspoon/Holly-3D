//! Insert Vertex (edge / polygon) and the Cut knife.

use super::*;
use std::collections::HashSet;

/// Insert Vertex on an edge at parameter t (0 at `a`).
pub fn insert_vertex_on_edge(m: &mut EditablePoly, a: VertexId, b: VertexId, t: f32) -> VertexId {
    let v = split_edge(m, a, b, t.clamp(0.0, 1.0));
    m.finalize();
    v
}

/// Insert Vertex in a polygon: pokes a new vertex at `point` connected to every corner.
pub fn poke_polygon(m: &mut EditablePoly, pid: PolyId, point: Vec3) -> VertexId {
    let p = m.polygons[pid].clone();
    let center = m.add_vertex(point);
    let n = p.corners.len();
    let cuv = p.corners.iter().map(|c| c.uv).sum::<Vec2>() / n as f32;
    m.polygons.remove(pid);
    for i in 0..n {
        let a = p.corners[i].clone();
        let b = p.corners[(i + 1) % n].clone();
        let id = m.add_polygon_corners(vec![a, b, FaceCorner::new(center, cuv)], &p);
        m.polygons[id].selected = p.selected;
    }
    m.finalize();
    center
}

/// Cuts a new edge path across the surface from vertex `a` to vertex `b`.
/// The cutting plane contains the segment and `view_dir` (the camera ray), so
/// the cut follows the straight screen-space line just like 3ds Max.
/// Returns the vertices along the path (including both ends).
pub fn cut(m: &mut EditablePoly, a: VertexId, b: VertexId, view_dir: Vec3) -> Vec<VertexId> {
    let mut path = vec![a];
    if a == b {
        return path;
    }
    let pa = m.pos(a);
    let pb = m.pos(b);
    let ab = pb - pa;
    let mut pn = ab.cross(view_dir);
    if pn.length_squared() < 1e-12 {
        pn = ab.any_orthogonal_vector();
    }
    let pn = pn.normalize();
    let eps = 1e-5 * (1.0 + ab.length());
    let dist = |p: Vec3| (p - pa).dot(pn);
    let prog = |p: Vec3| (p - pa).dot(ab) / ab.length_squared();
    let mut cur = a;
    let mut visited: HashSet<PolyId> = HashSet::new();
    for _ in 0..4096 {
        if cur == b {
            break;
        }
        let tc = prog(m.pos(cur));
        // candidates: (t, face, kind)
        enum Hit {
            Vertex(VertexId),
            Edge(VertexId, VertexId, f32),
        }
        let mut best: Option<(f32, PolyId, Hit)> = None;
        for (pid, p) in &m.polygons {
            if visited.contains(&pid) || p.position_of(cur).is_none() {
                continue;
            }
            let mut consider = |t: f32, h: Hit| {
                if t > tc + 1e-6 && t <= 1.0 + 1e-4 && best.as_ref().is_none_or(|bb| t < bb.0) {
                    best = Some((t, pid, h));
                }
            };
            if p.position_of(b).is_some() {
                consider(1.0, Hit::Vertex(b));
                continue;
            }
            for (x, y) in p.half_edges() {
                let (dx, dy) = (dist(m.pos(x)), dist(m.pos(y)));
                if x != cur && dx.abs() <= eps {
                    consider(prog(m.pos(x)), Hit::Vertex(x));
                } else if x != cur && y != cur && dx.abs() > eps && dy.abs() > eps && dx * dy < 0.0 {
                    let t = dx / (dx - dy);
                    consider(prog(m.pos(x).lerp(m.pos(y), t)), Hit::Edge(x, y, t));
                }
            }
        }
        let Some((_, pid, hit)) = best else { break };
        let next = match hit {
            Hit::Vertex(v) => v,
            Hit::Edge(x, y, t) => split_edge(m, x, y, t),
        };
        if let Some((p1, p2)) = split_polygon(m, pid, cur, next) {
            visited.insert(p1);
            visited.insert(p2);
        } else {
            visited.insert(pid);
        }
        path.push(next);
        cur = next;
    }
    m.finalize();
    path
}
