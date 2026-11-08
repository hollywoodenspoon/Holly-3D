//! More Edit Poly op tests (weld, remove, cut, slice, cap, bridge, planar, …).

use super::dispatch::CutPoint;
use super::tests::{cube, grid, ok, select_edge_where, select_faces_facing};
use super::*;
use bevy::math::Mat4;

#[test]
fn weld_two_planes() {
    let mut a = grid(1);
    let b = grid(1);
    a.append(&b, Mat4::from_translation(Vec3::new(1.0005, 0.0, 0.0)));
    assert_eq!(a.counts(), (8, 8, 2));
    a.select_all(SubObject::Vertex, true);
    PolyOp::Weld { level: SubObject::Vertex, threshold: 0.01 }.apply(&mut a);
    ok(&a, (6, 7, 2));
}

#[test]
fn target_weld_collapses_edge() {
    let mut g = grid(1);
    let vs: Vec<_> = g.vertices.keys().collect();
    PolyOp::TargetWeld { from: vs[0], to: vs[1] }.apply(&mut g);
    ok(&g, (3, 3, 1));
}

#[test]
fn collapse_polygon_to_point() {
    let mut m = cube();
    select_faces_facing(&mut m, Vec3::Z);
    PolyOp::Collapse { level: SubObject::Polygon }.apply(&mut m);
    ok(&m, (5, 8, 5)); // pyramid
    assert!(m.is_closed());
}

#[test]
fn remove_center_vertex_merges_faces() {
    let mut g = grid(2);
    let c = g.vertices.keys().find(|&v| g.pos(v).length() < 1e-5).unwrap();
    g.vertices[c].selected = true;
    PolyOp::RemoveVertices.apply(&mut g);
    ok(&g, (8, 8, 1));
}

#[test]
fn remove_edge_and_ctrl_remove() {
    let mut g = grid(2);
    select_edge_where(&mut g, |a, b| a.x.abs() < 1e-4 && b.x.abs() < 1e-4 && a.y.min(b.y) > -1e-4);
    PolyOp::RemoveEdges { remove_verts: false }.apply(&mut g);
    ok(&g, (9, 11, 3));
    let mut g = grid(2);
    select_edge_where(&mut g, |a, b| a.x.abs() < 1e-4 && b.x.abs() < 1e-4 && a.y.min(b.y) > -1e-4);
    PolyOp::RemoveEdges { remove_verts: true }.apply(&mut g);
    ok(&g, (8, 10, 3)); // border vertex (0,1) dissolved
}

#[test]
fn break_and_split() {
    let mut g = grid(2);
    let c = g.vertices.keys().find(|&v| g.pos(v).length() < 1e-5).unwrap();
    g.vertices[c].selected = true;
    PolyOp::BreakVertices.apply(&mut g);
    ok(&g, (12, 16, 4));

    let mut g = grid(2);
    select_edge_where(&mut g, |a, b| a.x.abs() < 1e-4 && b.x.abs() < 1e-4);
    PolyOp::SplitEdges.apply(&mut g);
    ok(&g, (12, 14, 4));
}

#[test]
fn delete_polygon_and_cap_border() {
    let mut m = cube();
    select_faces_facing(&mut m, Vec3::Z);
    PolyOp::Delete { level: SubObject::Polygon }.apply(&mut m);
    ok(&m, (8, 12, 5));
    assert!(!m.is_closed());
    m.clear_selection();
    select_edge_where(&mut m, |a, b| a.z > 0.9 && b.z > 0.9);
    m.select_borders();
    assert_eq!(m.selection_count(SubObject::Border), 4);
    PolyOp::Cap.apply(&mut m);
    ok(&m, (8, 12, 6));
    assert!(m.is_closed());
    assert!((m.signed_volume() - 1.0).abs() < 1e-4);
}

#[test]
fn insert_vertex_edge_and_polygon() {
    let mut m = cube();
    let e = m.edges.values().next().cloned().unwrap();
    PolyOp::InsertVertexOnEdge { a: e.v0, b: e.v1, t: 0.25 }.apply(&mut m);
    ok(&m, (9, 13, 6));
    let p = m.polygons.keys().next().unwrap();
    let c = m.poly_centroid(p);
    PolyOp::InsertVertexInPolygon { poly: p, point: c }.apply(&mut m);
    m.validate().unwrap();
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
}

#[test]
fn cut_across_grid() {
    let mut g = grid(2);
    // from the middle of the left border edge to the middle of the right border edge
    let find_edge = |g: &EditablePoly, x: f32, ylo: f32| {
        g.edges
            .values()
            .find(|e| {
                (g.pos(e.v0).x - x).abs() < 1e-4
                    && (g.pos(e.v1).x - x).abs() < 1e-4
                    && g.pos(e.v0).y.min(g.pos(e.v1).y) > ylo - 1e-4
                    && g.pos(e.v0).y.min(g.pos(e.v1).y) < ylo + 1e-4
            })
            .map(|e| (e.v0, e.v1))
            .unwrap()
    };
    let (a0, a1) = find_edge(&g, -1.0, 0.0);
    let (b0, b1) = find_edge(&g, 1.0, 0.0);
    PolyOp::Cut { from: CutPoint::Edge(a0, a1, 0.5), to: CutPoint::Edge(b0, b1, 0.5), view_dir: -Vec3::Z }.apply(&mut g);
    g.validate().unwrap();
    // two new border points + one on the middle vertical edge; both top faces split
    assert_eq!(g.vertices.len(), 12);
    assert_eq!(g.polygons.len(), 6);
}

#[test]
fn slice_cube_in_half() {
    let mut m = cube();
    PolyOp::Slice { point: Vec3::new(0., 0., 0.5), normal: Vec3::Z, only_selected: false, split: false }.apply(&mut m);
    ok(&m, (12, 20, 10));
    assert!(m.is_closed());
    let mut m = cube();
    PolyOp::Slice { point: Vec3::new(0., 0., 0.5), normal: Vec3::Z, only_selected: false, split: true }.apply(&mut m);
    ok(&m, (16, 24, 10));
    assert!(!m.is_closed());
}

#[test]
fn bridge_two_holes_through_cube() {
    let mut m = cube();
    m.clear_selection();
    let ids: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).z.abs() > 0.99).collect();
    ids.iter().for_each(|&p| m.polygons[p].selected = true);
    assert!(PolyOp::BridgePolygons { segments: 2, taper: 0.0, twist: 0 }.apply(&mut m));
    m.validate().unwrap();
    assert!(m.is_closed());
    assert_eq!(m.euler(), 0, "a tunnel makes a torus");
    assert_eq!(m.polygons.len(), 4 + 8);
}

#[test]
fn bridge_borders_two_facing_planes() {
    let mut m = EditablePoly::new();
    m.append(&grid(1), Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0)));
    m.append(&grid(1), Mat4::from_translation(Vec3::Z));
    m.select_all(SubObject::Edge, true);
    m.select_borders();
    assert!(PolyOp::BridgeBorders { segments: 1, taper: 0.0, twist: 0 }.apply(&mut m));
    ok(&m, (8, 12, 6));
    assert!(m.is_closed());
    assert!((m.signed_volume() - 1.0).abs() < 1e-4);
}

#[test]
fn make_planar_and_axis() {
    let mut g = grid(2);
    let c = g.vertices.keys().find(|&v| g.pos(v).length() < 1e-5).unwrap();
    g.vertices[c].position.z = 0.7;
    g.select_all(SubObject::Polygon, true);
    PolyOp::Flatten { level: SubObject::Polygon, axis: 2 }.apply(&mut g);
    let zs: Vec<f32> = g.vertices.values().map(|v| v.position.z).collect();
    assert!(zs.iter().all(|z| (z - 0.7 / 9.0).abs() < 1e-5));
    g.vertices[c].position.z = 1.0;
    PolyOp::MakePlanar { level: SubObject::Polygon }.apply(&mut g);
    let first = g.vertices.values().next().unwrap().position.z;
    assert!(g.vertices.values().all(|v| (v.position.z - first).abs() < 1e-4));
    ok(&g, (9, 12, 4));
}

#[test]
fn flip_and_smoothing() {
    let mut m = cube();
    m.select_all(SubObject::Polygon, true);
    PolyOp::Flip.apply(&mut m);
    assert!(m.signed_volume() < 0.0);
    PolyOp::ClearSmoothing.apply(&mut m);
    assert!(m.polygons.values().all(|p| p.smoothing_group == 0));
    PolyOp::SetSmoothing { bit: 4, on: true }.apply(&mut m);
    assert!(m.polygons.values().all(|p| p.smoothing_group == 1 << 4));
    PolyOp::AutoSmooth { threshold: 45.0 }.apply(&mut m);
    // all cube neighbours are 90 degrees: no adjacent faces may share a bit
    let adj = crate::mesh::adjacency::Adjacency::build(&m);
    for fs in adj.edge_faces.values() {
        assert_eq!(m.polygons[fs[0]].smoothing_group & m.polygons[fs[1]].smoothing_group, 0);
    }
    let mut s = crate::primitives::Primitive::Sphere { radius: 1.0, segments: 16, hemisphere: 0.0 }.to_editable_poly();
    s.polygons.values_mut().for_each(|p| p.smoothing_group = 0);
    s.select_all(SubObject::Polygon, true);
    PolyOp::AutoSmooth { threshold: 45.0 }.apply(&mut s);
    let first = s.polygons.values().next().unwrap().smoothing_group;
    assert!(first != 0 && s.polygons.values().all(|p| p.smoothing_group == first));
    PolyOp::SetMaterialId { id: 7 }.apply(&mut s);
    s.select_by_material_id(7, false);
    assert_eq!(s.selected_polygons().len(), s.polygons.len());
}

#[test]
fn tessellate_and_msmooth() {
    let mut m = cube();
    m.select_all(SubObject::Polygon, true);
    PolyOp::Tessellate { face_center: false }.apply(&mut m);
    ok(&m, (26, 48, 24));
    let mut m = cube();
    m.select_all(SubObject::Polygon, true);
    PolyOp::Tessellate { face_center: true }.apply(&mut m);
    ok(&m, (14, 36, 24));
    let mut m = cube();
    m.select_all(SubObject::Polygon, true);
    PolyOp::MSmooth { strength: 0.5 }.apply(&mut m);
    m.validate().unwrap();
    assert!(m.is_closed());
}

#[test]
fn detach_and_attach() {
    let mut m = cube();
    select_faces_facing(&mut m, Vec3::Z);
    let part = geometry::detach(&mut m, false);
    ok(&part, (4, 4, 1));
    ok(&m, (8, 12, 5));
    PolyOp::Attach { mesh: Box::new(part) }.apply(&mut m);
    ok(&m, (12, 16, 6));
}

#[test]
fn shift_extrude_border_edges() {
    let mut g = grid(1);
    select_edge_where(&mut g, |a, b| a.y > 0.4 && b.y > 0.4);
    PolyOp::ShiftExtrudeEdges.apply(&mut g);
    ok(&g, (6, 7, 2));
    assert_eq!(g.selection_count(SubObject::Edge), 1);
    let moved = g.selection_vertices(SubObject::Edge);
    PolyOp::Transform { verts: moved, matrix: Mat4::from_translation(Vec3::Y) }.apply(&mut g);
    g.validate().unwrap();
    let total: f32 = g.polygons.keys().map(|p| g.poly_area(p)).sum();
    assert!((total - 2.0).abs() < 1e-4);
}

#[test]
fn edge_and_vertex_extrude() {
    let mut m = cube();
    select_edge_where(&mut m, |a, b| a.z > 0.9 && b.z > 0.9 && a.y < -0.4 && b.y < -0.4);
    PolyOp::ExtrudeEdges { height: 0.2, width: 0.2 }.apply(&mut m);
    m.validate().unwrap();
    assert!(m.is_closed());
    let mut m = cube();
    let v = m.vertices.keys().next().unwrap();
    m.vertices[v].selected = true;
    PolyOp::ExtrudeVertices { height: 0.3, width: 0.2 }.apply(&mut m);
    ok(&m, (11, 18, 9));
    assert!(m.is_closed());
}

#[test]
fn selection_grow_shrink_loop_ring() {
    let mut g = grid(4);
    let c = g.polygons.keys().min_by(|&a, &b| g.poly_centroid(a).length().total_cmp(&g.poly_centroid(b).length())).unwrap();
    g.polygons[c].selected = true;
    g.grow_selection(SubObject::Polygon);
    assert_eq!(g.selection_count(SubObject::Polygon), 9);
    g.shrink_selection(SubObject::Polygon);
    assert_eq!(g.selection_count(SubObject::Polygon), 1);

    let mut g = grid(4);
    let e = g.edges.iter().find(|(_, e)| (g.pos(e.v0).y.abs() < 1e-4) && (g.pos(e.v1).y.abs() < 1e-4)).map(|(k, _)| k).unwrap();
    g.edges[e].selected = true;
    g.select_loop();
    assert_eq!(g.selection_count(SubObject::Edge), 4);
    g.clear_selection();
    g.edges[e].selected = true;
    g.select_ring();
    assert_eq!(g.selection_count(SubObject::Edge), 5);
}
