use super::edit::SplineLevel;
use super::*;
use crate::modifiers::{ExtrudeMod, LatheMod};
use bevy::math::Vec3;

fn shape_of(kind: ShapeKind) -> Shape {
    ShapeParams::default_of(kind).unwrap().to_shape()
}

fn line(pts: &[Vec3], closed: bool) -> Shape {
    Shape { splines: vec![Spline { knots: pts.iter().map(|&p| Knot::corner(p)).collect(), closed, selected: false }], steps: 6 }
}

#[test]
fn circle_is_round_and_closed() {
    let s = shape_of(ShapeKind::Circle);
    assert_eq!((s.knot_count(), s.segment_count()), (4, 4));
    let pl = &s.polylines()[0];
    assert!(pl.closed);
    assert_eq!(pl.pts.len(), 4 * 7, "4 curved segments x (steps + 1)");
    for p in &pl.pts {
        assert!((p.length() - 0.6).abs() < 0.002, "on the circle: {}", p.length());
    }
    assert!((pl.length() - std::f32::consts::TAU * 0.6).abs() < 0.02);
    assert!(pl.signed_area_xy() > 0.0, "counter-clockwise");
}

#[test]
fn every_parametric_shape_interpolates() {
    for kind in ShapeKind::ALL.into_iter().filter(|k| *k != ShapeKind::Line) {
        let s = shape_of(kind);
        let pl = s.polylines();
        assert_eq!(pl.len(), 1, "{kind:?}");
        assert!(pl[0].pts.len() >= 3 && pl[0].pts.iter().all(|p| p.is_finite()), "{kind:?}");
    }
    let r = shape_of(ShapeKind::Rectangle).polylines().remove(0);
    assert_eq!(r.pts.len(), 4, "straight segments are not subdivided");
    let star = shape_of(ShapeKind::Star);
    assert_eq!(star.knot_count(), 12);
    let rounded = ShapeParams::Rectangle { length: 1.0, width: 1.0, corner_radius: 0.2 }.to_shape();
    assert_eq!(rounded.knot_count(), 8);
    let pl = rounded.polylines().remove(0);
    assert_eq!(pl.pts.len(), 4 * 7 + 4, "4 arcs + 4 straight edges");
    let helix = shape_of(ShapeKind::Helix).polylines().remove(0);
    assert!((helix.pts.last().unwrap().z - 1.2).abs() < 1e-4 && !helix.closed);
}

#[test]
fn refine_keeps_the_curve() {
    let mut s = shape_of(ShapeKind::Circle);
    let before = s.polylines()[0].length();
    let idx = s.splines[0].refine(1, 0.5);
    assert_eq!(idx, 2);
    assert_eq!(s.knot_count(), 5);
    assert!((s.splines[0].knots[2].pos.length() - 0.6).abs() < 0.002);
    assert!((s.polylines()[0].length() - before).abs() < 0.01);
}

#[test]
fn fillet_chamfer_delete_and_divide() {
    let mut s = shape_of(ShapeKind::Rectangle);
    s.splines[0].knots[0].selected = true;
    assert_eq!(s.chamfer_selected(0.2, false), 1);
    assert_eq!((s.knot_count(), s.segment_count()), (5, 5));
    s.clear_selection();
    s.splines[0].knots[3].selected = true;
    assert_eq!(s.chamfer_selected(0.2, true), 1);
    assert_eq!(s.knot_count(), 6);
    assert!(!s.splines[0].segment_is_line(3), "fillet arc is curved");
    s.clear_selection();
    s.splines[0].knots[0].seg_selected = true;
    s.divide_selected(2);
    assert_eq!(s.knot_count(), 8);
    s.delete_selected(SplineLevel::Segment);
    assert_eq!(s.splines.len(), 1);
    assert!(!s.splines[0].closed, "deleting segments opens the spline");
    assert_eq!(s.segment_count(), 5);
}

#[test]
fn weld_outline_mirror_make_first() {
    let mut s = line(&[Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 0.0005, 0.0)], false);
    let n = s.splines[0].knots.len();
    s.splines[0].knots[0].selected = true;
    s.splines[0].knots[n - 1].selected = true;
    assert_eq!(s.weld_selected(0.01), 1);
    assert!(s.splines[0].closed);
    assert_eq!(s.knot_count(), 3);
    s.splines[0].selected = true;
    s.outline_selected(0.1);
    assert_eq!(s.splines.len(), 2);
    s.mirror_selected(0, true);
    assert_eq!(s.splines.len(), 4, "both selected splines copied");
    let mut open = line(&[Vec3::ZERO, Vec3::X, Vec3::Y], false);
    open.splines[0].make_first(2);
    assert_eq!(open.splines[0].knots[0].pos, Vec3::Y);
    let mut o2 = line(&[Vec3::ZERO, Vec3::X * 2.0], false);
    o2.splines[0].selected = true;
    o2.outline_selected(0.1);
    assert!(o2.splines[0].closed && o2.knot_count() == 4, "open outline becomes a closed loop");
}

#[test]
fn rendering_sweep_counts() {
    let mut r = SplineRender { viewport: true, sides: 8, ..Default::default() };
    let m = r.sweep(&shape_of(ShapeKind::Rectangle));
    assert_eq!(m.counts(), (32, 64, 32), "closed: 4 rings x 8 sides");
    assert!(m.validate().is_ok() && m.is_closed());
    r.rectangular = true;
    let m = r.sweep(&line(&[Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0)], false));
    assert_eq!(m.counts(), (12, 20, 10), "open: 3 rings x 4 + 2 caps");
    assert!(m.validate().is_ok() && m.is_closed() && m.signed_volume() > 0.0);
}

#[test]
fn lathe_vase_counts_and_outward_normals() {
    // L-shaped profile in XY touching the Y axis at both ends (vase / goblet).
    let prof = line(&[Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.0, 0.0), Vec3::new(0.5, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)], false);
    let lathe = LatheMod { segments: 12, ..Default::default() };
    let m = lathe.build(&prof);
    // core points weld into single poles: 2 rings x 12 + 2 poles
    assert_eq!(m.counts(), (26, 60, 36));
    assert!(m.validate().is_ok() && m.is_closed());
    assert!(m.signed_volume() > 0.0, "outward by default");
    let flipped = LatheMod { flip_normals: true, ..lathe.clone() }.build(&prof);
    assert!(flipped.signed_volume() < 0.0);
    let unwelded = LatheMod { weld_core: false, ..lathe.clone() }.build(&prof);
    assert_eq!(unwelded.vertices.len(), 48);
    let half = LatheMod { degrees: 180.0, segments: 6, ..lathe }.build(&prof);
    assert_eq!(half.polygons.len(), 3 * 6 + 2, "half lathe + 2 caps");
    assert!(half.validate().is_ok());
}

#[test]
fn extrude_closed_and_open() {
    let ex = ExtrudeMod { amount: 1.0, segments: 2, ..Default::default() };
    let m = ex.build(&shape_of(ShapeKind::Star));
    assert_eq!(m.counts(), (36, 60, 26), "12-point star x 3 levels, 24 walls + 2 caps");
    assert!(m.validate().is_ok() && m.is_closed() && m.signed_volume() > 0.0);
    let mut cw = shape_of(ShapeKind::Rectangle);
    cw.splines[0].reverse();
    assert!(ex.build(&cw).signed_volume() > 0.0, "clockwise outlines are reoriented");
    let open = ex.build(&line(&[Vec3::ZERO, Vec3::X, Vec3::Y], false));
    assert_eq!(open.counts(), (9, 12, 4), "open spline: walls only");
}

#[test]
fn shape_stack_lathe_consumes_the_shape() {
    use crate::scene::{BaseObject, Scene};
    let mut sc = Scene::default();
    let id = sc.add(BaseObject::Shape(ShapeObject::param(ShapeParams::default_of(ShapeKind::Rectangle).unwrap())), Vec3::ZERO);
    let o = &mut sc.objects[id];
    assert!(o.shape.is_some() && o.evaluated_mesh.polygons.is_empty());
    o.add_modifier(crate::modifiers::make_modifier("Extrude").unwrap());
    assert!(o.shape.is_none());
    assert_eq!(o.evaluated_mesh.counts(), (8, 12, 6), "extruded rectangle is a box");
    o.modifiers[0].enabled_in_viewport = false;
    o.evaluate();
    assert!(o.shape.is_some() && o.evaluated_mesh.polygons.is_empty());
    o.shape_object_mut().unwrap().render.renderer = true;
    o.evaluate();
    assert!(o.evaluated_mesh.polygons.is_empty() && !o.render_mesh().polygons.is_empty(), "renderer-only sweep");
}
