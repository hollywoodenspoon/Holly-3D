//! Ray / screen-space picking for objects and sub-objects (pure, testable).

use super::camera::Ray;
use crate::mesh::{EdgeId, EditablePoly, PolyId, VertexId};
use bevy::math::{Mat4, Vec2, Vec3};

/// Möller–Trumbore; returns distance along the ray.
pub fn ray_triangle(ray: &Ray, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = ray.dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = ray.origin - a;
    let u = s.dot(p) * inv;
    if !(-1e-6..=1.0 + 1e-6).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = ray.dir.dot(q) * inv;
    if v < -1e-6 || u + v > 1.0 + 1e-6 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 0.0).then_some(t)
}

#[derive(Clone, Copy, Debug)]
pub struct PolyHit {
    pub poly: PolyId,
    /// World-space hit point and distance from the ray origin.
    pub point: Vec3,
    pub distance: f32,
}

/// Closest polygon hit by a world ray on a mesh placed by `world`.
pub fn pick_polygon(mesh: &EditablePoly, world: Mat4, ray: &Ray, ignore_backfacing: bool) -> Option<PolyHit> {
    let inv = world.inverse();
    let local = Ray { origin: inv.transform_point3(ray.origin), dir: inv.transform_vector3(ray.dir).normalize_or_zero() };
    let mut best: Option<PolyHit> = None;
    for pid in mesh.polygons.keys() {
        if ignore_backfacing && mesh.poly_normal_raw(pid).dot(local.dir) > 0.0 {
            continue;
        }
        for [a, b, c] in mesh.poly_triangles(pid) {
            if let Some(t) = ray_triangle(&local, a, b, c) {
                let point = world.transform_point3(local.at(t));
                let distance = (point - ray.origin).length();
                if best.is_none_or(|h| distance < h.distance) {
                    best = Some(PolyHit { poly: pid, point, distance });
                }
            }
        }
    }
    best
}

/// Distance from point to segment in 2D, and the segment parameter.
pub fn point_segment(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let ab = b - a;
    let t = if ab.length_squared() < 1e-12 { 0.0 } else { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) };
    ((a + ab * t - p).length(), t)
}

/// Screen-projection callback: world point -> screen pixel (None if clipped).
pub type Project<'a> = &'a dyn Fn(Vec3) -> Option<Vec2>;

pub fn pick_vertex(
    mesh: &EditablePoly,
    world: Mat4,
    project: Project,
    cursor: Vec2,
    radius: f32,
    visible: &dyn Fn(VertexId) -> bool,
) -> Option<VertexId> {
    mesh.vertices
        .iter()
        .filter(|(id, _)| visible(*id))
        .filter_map(|(id, v)| project(world.transform_point3(v.position)).map(|s| (id, s.distance(cursor))))
        .filter(|(_, d)| *d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// Nearest edge to the cursor (screen space) and the parameter along v0 -> v1.
pub fn pick_edge(
    mesh: &EditablePoly,
    world: Mat4,
    project: Project,
    cursor: Vec2,
    radius: f32,
    visible: &dyn Fn(EdgeId) -> bool,
) -> Option<(EdgeId, f32)> {
    let mut best: Option<(EdgeId, f32, f32)> = None;
    for (id, e) in &mesh.edges {
        if !visible(id) {
            continue;
        }
        let (Some(a), Some(b)) = (project(world.transform_point3(mesh.pos(e.v0))), project(world.transform_point3(mesh.pos(e.v1)))) else {
            continue;
        };
        let (d, t) = point_segment(cursor, a, b);
        if d <= radius && best.is_none_or(|x| d < x.1) {
            best = Some((id, d, t));
        }
    }
    best.map(|(id, _, t)| (id, t))
}

/// Screen-space rectangle test helper for marquee (window) selection.
pub fn in_rect(p: Vec2, min: Vec2, max: Vec2) -> bool {
    p.x >= min.x && p.y >= min.y && p.x <= max.x && p.y <= max.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};

    #[test]
    fn ray_hits_box_top() {
        let m = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
        let ray = Ray { origin: Vec3::new(0.1, 0.1, 5.0), dir: -Vec3::Z };
        let hit = pick_polygon(&m, Mat4::IDENTITY, &ray, true).unwrap();
        assert!((hit.point.z - 1.0).abs() < 1e-5);
        assert!(m.poly_normal(hit.poly).z > 0.99);
        let moved = pick_polygon(&m, Mat4::from_translation(Vec3::new(0.0, 0.0, 2.0)), &ray, true).unwrap();
        assert!((moved.point.z - 3.0).abs() < 1e-5);
        let miss = Ray { origin: Vec3::new(3.0, 0.0, 5.0), dir: -Vec3::Z };
        assert!(pick_polygon(&m, Mat4::IDENTITY, &miss, false).is_none());
    }

    #[test]
    fn screen_space_vertex_and_edge() {
        let m = Primitive::Plane { length: 1.0, width: 1.0, length_segs: 1, width_segs: 1 }.to_editable_poly();
        let project = |p: Vec3| Some(Vec2::new(p.x * 100.0, p.y * 100.0));
        let v = pick_vertex(&m, Mat4::IDENTITY, &project, Vec2::new(48.0, 52.0), 8.0, &|_| true).unwrap();
        assert!((m.pos(v) - Vec3::new(0.5, 0.5, 0.0)).length() < 1e-5);
        let (e, t) = pick_edge(&m, Mat4::IDENTITY, &project, Vec2::new(0.0, 49.0), 4.0, &|_| true).unwrap();
        let ed = &m.edges[e];
        assert!((m.pos(ed.v0).y - 0.5).abs() < 1e-5 && (m.pos(ed.v1).y - 0.5).abs() < 1e-5);
        assert!((t - 0.5).abs() < 0.01);
    }
}
