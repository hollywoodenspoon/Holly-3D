//! Headless geometry tests: vertex / edge / polygon counts plus validity.

use super::*;
use crate::primitives::{Primitive, PrimitiveKind};

pub(crate) fn cube() -> EditablePoly {
    Primitive::default_of(PrimitiveKind::Box).to_editable_poly()
}

pub(crate) fn grid(n: u32) -> EditablePoly {
    Primitive::Plane { length: n as f32, width: n as f32, length_segs: n, width_segs: n }.to_editable_poly()
}

pub(crate) fn select_faces_facing(m: &mut EditablePoly, dir: Vec3) {
    m.clear_selection();
    let ids: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).dot(dir) > 0.99).collect();
    ids.into_iter().for_each(|p| m.polygons[p].selected = true);
}

pub(crate) fn select_edge_where(m: &mut EditablePoly, f: impl Fn(Vec3, Vec3) -> bool) {
    let ids: Vec<_> = m.edges.iter().filter(|(_, e)| f(m.pos(e.v0), m.pos(e.v1))).map(|(k, _)| k).collect();
    ids.into_iter().for_each(|e| m.edges[e].selected = true);
}

pub(crate) fn ok(m: &EditablePoly, counts: (usize, usize, usize)) {
    if let Err(e) = m.validate() {
        panic!("invalid mesh: {e}");
    }
    assert_eq!(m.counts(), counts, "V/E/F");
}

#[test]
fn extrude_group_top_face() {
    let mut m = cube();
    select_faces_facing(&mut m, Vec3::Z);
    PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 0.5 }.apply(&mut m);
    ok(&m, (12, 20, 10));
    assert!(m.is_closed());
    assert!((m.signed_volume() - 1.5).abs() < 1e-4);
    let top = m.selected_polygons();
    assert_eq!(top.len(), 1);
    assert!((m.poly_centroid(top[0]).z - 1.5).abs() < 1e-5);
}

#[test]
fn extrude_by_polygon_two_faces() {
    let mut m = cube();
    m.clear_selection();
    let two: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).z > 0.99 || m.poly_normal(p).x > 0.99).collect();
    two.iter().for_each(|&p| m.polygons[p].selected = true);
    PolyOp::ExtrudePolygons { mode: ExtrudeMode::ByPolygon, height: 0.2 }.apply(&mut m);
    ok(&m, (16, 28, 14));
    assert!(m.is_closed());
}

#[test]
fn extrude_group_adjacent_faces_shares_boundary() {
    let mut m = cube();
    m.clear_selection();
    let two: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).z > 0.99 || m.poly_normal(p).x > 0.99).collect();
    two.iter().for_each(|&p| m.polygons[p].selected = true);
    PolyOp::ExtrudePolygons { mode: ExtrudeMode::LocalNormal, height: 0.2 }.apply(&mut m);
    // region of 2 faces: 6 boundary verts duplicated, 6 side quads
    ok(&m, (14, 24, 12));
    assert!(m.is_closed());
}

#[test]
fn bevel_shrinks_cap() {
    let mut m = cube();
    select_faces_facing(&mut m, Vec3::Z);
    PolyOp::Bevel { mode: ExtrudeMode::Group, height: 0.3, outline: -0.1 }.apply(&mut m);
    ok(&m, (12, 20, 10));
    let top = m.selected_polygons()[0];
    assert!((m.poly_area(top) - 0.64).abs() < 1e-4);
}

#[test]
fn inset_group_and_by_polygon() {
    let mut m = cube();
    select_faces_facing(&mut m, Vec3::Z);
    PolyOp::Inset { by_polygon: false, amount: 0.1 }.apply(&mut m);
    ok(&m, (12, 20, 10));
    let top = m.selected_polygons()[0];
    assert!((m.poly_area(top) - 0.64).abs() < 1e-4);
    assert!((m.signed_volume() - 1.0).abs() < 1e-4);

    let mut g = grid(2);
    g.select_all(SubObject::Polygon, true);
    PolyOp::Inset { by_polygon: true, amount: 0.1 }.apply(&mut g);
    g.validate().unwrap();
    assert_eq!(g.polygons.len(), 4 * 5);
    assert_eq!(g.vertices.len(), 9 + 16);
}

#[test]
fn outline_grows_region() {
    let mut g = grid(3);
    g.clear_selection();
    let center = g.polygons.keys().find(|&p| g.poly_centroid(p).length() < 1e-4).unwrap();
    g.polygons[center].selected = true;
    PolyOp::Outline { amount: 0.25 }.apply(&mut g);
    ok(&g, (16, 24, 9));
    assert!((g.poly_area(center) - 2.25).abs() < 1e-4);
}

#[test]
fn chamfer_single_cube_edge() {
    let mut m = cube();
    select_edge_where(&mut m, |a, b| a.z > 0.9 && b.z > 0.9 && a.y < -0.4 && b.y < -0.4);
    PolyOp::ChamferEdges { amount: 0.1, segments: 1, tension: 0.5, open: false }.apply(&mut m);
    ok(&m, (10, 15, 7));
    assert!(m.is_closed());
    assert!(m.signed_volume() < 1.0 && m.signed_volume() > 0.98);
}

#[test]
fn chamfer_all_cube_edges() {
    let mut m = cube();
    m.select_all(SubObject::Edge, true);
    PolyOp::ChamferEdges { amount: 0.1, segments: 1, tension: 0.5, open: false }.apply(&mut m);
    // 6 faces + 12 strips + 8 corner triangles
    ok(&m, (24, 48, 26));
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
}

#[test]
fn chamfer_segments_rounds() {
    let mut m = cube();
    m.select_all(SubObject::Edge, true);
    PolyOp::ChamferEdges { amount: 0.1, segments: 3, tension: 0.5, open: false }.apply(&mut m);
    m.validate().unwrap();
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
    assert_eq!(m.polygons.len(), 6 + 12 * 3 + 8);
}

#[test]
fn chamfer_grid_interior_edge_keeps_vertices() {
    let mut g = grid(2);
    // interior edge along x = 0 from y = 0 to y = 1 (endpoints: center valence 4, border valence 3)
    select_edge_where(&mut g, |a, b| a.x.abs() < 1e-4 && b.x.abs() < 1e-4 && a.y.min(b.y) > -1e-4);
    PolyOp::ChamferEdges { amount: 0.2, segments: 1, tension: 0.5, open: false }.apply(&mut g);
    g.validate().unwrap();
    assert_eq!(g.polygons.len(), 5);
    assert_eq!(g.euler(), 1);
}

#[test]
fn chamfer_vertex_corner() {
    let mut m = cube();
    let v = m.vertices.keys().next().unwrap();
    m.vertices[v].selected = true;
    PolyOp::ChamferVertices { amount: 0.2 }.apply(&mut m);
    ok(&m, (10, 15, 7));
    assert!(m.is_closed());
}

#[test]
fn connect_ring_on_grid() {
    let mut g = grid(2);
    // the two edges crossing y = 0.5 band at x=-1 and x=0 and x=1 (vertical edges in the top row)
    select_edge_where(&mut g, |a, b| (a.x - b.x).abs() < 1e-4 && a.y.min(b.y) > -1e-4);
    assert_eq!(g.selection_count(SubObject::Edge), 3);
    PolyOp::ConnectEdges { segments: 2, pinch: 0.0, slide: 0.0 }.apply(&mut g);
    g.validate().unwrap();
    assert_eq!(g.vertices.len(), 9 + 6);
    assert_eq!(g.polygons.len(), 4 + 4);
    assert_eq!(g.selection_count(SubObject::Edge), 4);
}

#[test]
fn connect_vertices_diagonal() {
    let mut g = grid(1);
    let vs: Vec<_> = g.vertices.keys().collect();
    g.vertices[vs[0]].selected = true;
    g.vertices[vs[3]].selected = true; // opposite corner (0,0)-(1,1)
    PolyOp::ConnectVertices.apply(&mut g);
    ok(&g, (4, 5, 2));
}
