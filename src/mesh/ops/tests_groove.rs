//! Panel Groove: counts, depth, lip and inner material ID.

use super::groove::panel_groove;
use super::tests::{grid, ok, select_edge_where};
use super::*;
use crate::primitives::Primitive;

fn center(m: &EditablePoly, p: PolyId) -> Vec3 {
    let vs: Vec<Vec3> = m.polygons[p].vertex_ids().map(|v| m.pos(v)).collect();
    vs.iter().sum::<Vec3>() / vs.len() as f32
}

#[test]
fn groove_along_a_plane_line() {
    let mut m = grid(4);
    m.clear_selection();
    select_edge_where(&mut m, |a, b| a.x.abs() < 1e-5 && b.x.abs() < 1e-5);
    assert_eq!(m.selected_edges().len(), 4);
    assert!(panel_groove(&mut m, 0.1, 0.05, 0.01, 3));
    // Chamfer splits the 5 line vertices (+5 V, +4 strip F), then the lip
    // bevel and the channel each add a 10-vertex ring and 10 side quads.
    ok(&m, (50, 89, 40));
    let lo = m.vertices.values().map(|v| v.position.z).fold(f32::MAX, f32::min);
    assert!((lo + 0.05).abs() < 1e-5, "groove depth {lo}");
    // The floor: 4 selected polygons at full depth, inner material, faceted.
    let floor = m.selected_polygons();
    assert_eq!(floor.len(), 4);
    for &p in &floor {
        assert!((center(&m, p).z + 0.05).abs() < 1e-5);
        assert_eq!((m.polygons[p].material_id, m.polygons[p].smoothing_group), (3, 0));
    }
    // Floor width is the Groove Width; the lip steps in by Bevel Lip Width.
    let xs: Vec<f32> = floor.iter().flat_map(|&p| m.polygons[p].vertex_ids().map(|v| m.pos(v).x).collect::<Vec<_>>()).collect();
    let w = xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min);
    assert!((w - 0.1).abs() < 1e-4, "floor width {w}");
    let walls = m.polygons.values().filter(|p| p.material_id == 3).count();
    assert_eq!(walls, 4 + 10, "floor + channel walls");
    // The surface around the groove keeps material 1.
    assert_eq!(m.polygons.values().filter(|p| p.material_id == 1).count(), 40 - 14);
}

#[test]
fn groove_loop_around_a_box_stays_watertight() {
    let b = Primitive::Box { length: 2.0, width: 2.0, height: 2.0, length_segs: 2, width_segs: 1, height_segs: 1 };
    let mut m = b.to_editable_poly();
    m.clear_selection();
    select_edge_where(&mut m, |a, b| a.y.abs() < 1e-5 && b.y.abs() < 1e-5);
    assert_eq!(m.selected_edges().len(), 4);
    let vol = m.signed_volume();
    assert!(panel_groove(&mut m, 0.1, 0.05, 0.02, 2));
    ok(&m, (32, 60, 30));
    assert!(m.is_closed() && m.euler() == 2);
    assert!(m.signed_volume() < vol && m.signed_volume() > vol * 0.9, "the channel removes a little volume");
    let top = m.selected_polygons().into_iter().find(|&p| m.poly_normal(p).z > 0.99).unwrap();
    assert!((center(&m, top).z - 1.95).abs() < 1e-4, "top floor at height - depth: {}", center(&m, top).z);
    // Nothing selected, or zero width: no change.
    let before = m.counts();
    m.clear_selection();
    assert!(!panel_groove(&mut m, 0.1, 0.05, 0.0, 2));
    select_edge_where(&mut m, |a, b| a.y.abs() < 1e-5 && b.y.abs() < 1e-5);
    assert!(!panel_groove(&mut m, 0.0, 0.05, 0.0, 2));
    assert_eq!(m.counts(), before);
}

#[test]
fn panel_groove_modifier_filters_edges_and_keeps_the_polygon_selection() {
    use crate::modifiers::{ChamferFilter, PanelGrooveMod};
    // All 12 cube edges: the chamfer network (12 strips + 8 corners) sinks
    // as one region bounded by 6 face loops (24 vertices) — twice.
    let mut m = super::tests::cube();
    m.clear_selection();
    let keep = m.polygons.keys().next().unwrap();
    m.polygons[keep].selected = true;
    PanelGrooveMod { filter: ChamferFilter::All, material_id: 9, ..PanelGrooveMod::default() }.apply(&mut m);
    ok(&m, (72, 144, 74));
    assert!(m.is_closed() && m.euler() == 2 && m.signed_volume() > 0.0);
    assert_eq!(m.selected_polygons(), vec![keep], "incoming polygon selection flows up");
    // Floor (20) and channel walls (24) take the inner ID; lips keep their face's.
    assert_eq!(m.polygons.values().filter(|p| p.material_id == 9).count(), 20 + 24);
    assert_eq!(m.polygons.values().filter(|p| p.smoothing_group == 0).count(), 20 + 24 + 24);
    // Selected Edges with nothing selected leaves the mesh alone.
    let mut c = super::tests::cube();
    c.clear_selection();
    PanelGrooveMod::default().apply(&mut c);
    ok(&c, (8, 12, 6));
    // As a recorded Edit Poly op it reports when nothing was grooved.
    assert!(!PolyOp::PanelGroove { width: 0.04, depth: 0.03, lip: 0.0, material_id: 2 }.apply(&mut c));
    select_edge_where(&mut c, |a, b| a.z > 0.99 && b.z > 0.99);
    assert!(PolyOp::PanelGroove { width: 0.04, depth: 0.03, lip: 0.0, material_id: 2 }.apply(&mut c));
    assert!(c.is_closed() && c.euler() == 2);
}
