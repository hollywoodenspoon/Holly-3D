//! Hinge From Edge, Extrude Along Spline, Create and edge-chain tests.

use super::sweep::{CreatePoint, selected_edge_chains};
use super::tests::{cube, grid, ok, select_edge_where, select_faces_facing};
use super::*;

fn top(m: &EditablePoly) -> f32 {
    m.vertices.values().map(|v| v.position.z).fold(f32::MIN, f32::max)
}

#[test]
fn hinge_top_face_about_its_edge() {
    let mut m = cube();
    let z = top(&m);
    let y0 = m.vertices.values().map(|v| v.position.y).fold(f32::MAX, f32::min);
    select_faces_facing(&mut m, Vec3::Z);
    let origin = Vec3::new(0.0, y0, z);
    let op = PolyOp::Hinge { origin, axis: Vec3::X, angle: 90.0, segments: 1 };
    assert!(op.apply(&mut m));
    // Hinge edge shared, two triangles and one quad of sides.
    ok(&m, (10, 17, 9));
    assert!(m.is_closed());
    let cap = m.selected_polygons()[0];
    let n = m.poly_normal(cap);
    assert!(n.z.abs() < 1e-4 && n.y < -0.99, "swung out over the hinge side: {n}");
    let mut segs = cube();
    select_faces_facing(&mut segs, Vec3::Z);
    PolyOp::Hinge { origin, axis: Vec3::NEG_X, angle: 90.0, segments: 3 }.apply(&mut segs);
    ok(&segs, (14, 27, 15));
    assert!(segs.signed_volume() > cube().signed_volume(), "axis sign does not matter: always outward");
    // Without a picked hinge nothing happens.
    assert!(!PolyOp::Hinge { origin, axis: Vec3::ZERO, angle: 90.0, segments: 1 }.apply(&mut segs));
}

#[test]
fn extrude_along_straight_and_bent_paths() {
    let mut m = cube();
    let z = top(&m);
    select_faces_facing(&mut m, Vec3::Z);
    let area = m.poly_area(m.selected_polygons()[0]);
    let path = vec![Vec3::ZERO, Vec3::Z * 3.0];
    let op = PolyOp::ExtrudeAlongSpline { path, segments: 4, taper: -0.5, twist: 90.0, align: false };
    assert!(op.apply(&mut m));
    ok(&m, (24, 44, 22));
    let cap = m.selected_polygons()[0];
    assert!((m.poly_centroid(cap).z - (z + 3.0)).abs() < 1e-4);
    assert!((m.poly_area(cap) - area * 0.25).abs() < 1e-4, "taper -0.5 halves the section");
    // Bent path aligned to the face normal: ends facing sideways.
    let mut b = cube();
    select_faces_facing(&mut b, Vec3::Z);
    let bent = vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 1.0)];
    assert!(PolyOp::ExtrudeAlongSpline { path: bent, segments: 6, taper: 0.0, twist: 0.0, align: true }.apply(&mut b));
    ok(&b, (32, 60, 30));
    let n = b.poly_normal(b.selected_polygons()[0]);
    assert!(n.x > 0.7, "follows the bend: {n}");
    assert!(b.is_closed());
}

#[test]
fn create_polygon_from_existing_and_new_points() {
    let mut g = grid(1);
    let mut vs: Vec<_> = g.vertices.keys().collect();
    vs.sort_by(|a, b| g.pos(*a).x.total_cmp(&g.pos(*b).x).then(g.pos(*a).y.total_cmp(&g.pos(*b).y)));
    // the two right-hand corners plus a new point further right
    let (a, b) = (vs[2], vs[3]);
    let pts = [CreatePoint::Vertex(a), CreatePoint::New(Vec3::new(1.5, 0.0, 0.0)), CreatePoint::Vertex(b)];
    assert!(PolyOp::CreatePolygon { points: pts.to_vec(), view_dir: Vec3::NEG_Z }.apply(&mut g));
    ok(&g, (5, 6, 2));
    assert!(g.poly_normal(g.selected_polygons()[0]).z > 0.99, "faces the viewer");
    let two = [CreatePoint::Vertex(a), CreatePoint::Vertex(b)];
    assert!(!PolyOp::CreatePolygon { points: two.to_vec(), view_dir: Vec3::NEG_Z }.apply(&mut g));
}

#[test]
fn edge_chains_for_create_shape() {
    let mut m = cube();
    let z = top(&m);
    select_edge_where(&mut m, |a, b| a.z == z && b.z == z);
    let chains = selected_edge_chains(&m);
    assert_eq!(chains.len(), 1);
    assert_eq!((chains[0].0.len(), chains[0].1), (4, true));
    m.clear_selection();
    let x0 = m.vertices.values().map(|v| v.position.x).fold(f32::MAX, f32::min);
    select_edge_where(&mut m, |a, b| a.z == z && b.z == z && (a.x != x0 || b.x != x0) && (a.y > 0.0 || b.y > 0.0));
    let chains = selected_edge_chains(&m);
    assert_eq!(chains.len(), 1);
    assert_eq!((chains[0].0.len(), chains[0].1), (3, false), "two connected edges: an open chain");
}
