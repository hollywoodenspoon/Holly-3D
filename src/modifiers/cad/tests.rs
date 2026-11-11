//! Headless tests for the SolidCraft-style CAD tools: Loft and the hole cutter.

use super::*;
use crate::mesh::ops::PolyOp;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::splines::{Knot, Shape, Spline};
use bevy::math::{Vec2, Vec3};

fn line(pts: &[Vec3]) -> Shape {
    Shape { splines: vec![Spline { knots: pts.iter().map(|&p| Knot::corner(p)).collect(), closed: false, selected: false }], steps: 6 }
}

fn square(half: f32, at: f32) -> LoftSection {
    let points = vec![Vec2::new(-half, -half), Vec2::new(half, -half), Vec2::new(half, half), Vec2::new(-half, half)];
    LoftSection { name: "Square".into(), at, points, closed: true }
}

fn path() -> Shape {
    line(&[Vec3::ZERO, Vec3::Z * 2.0])
}

#[test]
fn loft_default_circle_is_a_capped_solid() {
    let l = LoftMod { path_steps: 8, shape_steps: 16, ..Default::default() };
    let m = l.build(&path());
    let (v, e, f) = m.counts();
    assert_eq!(v, 16 * 9);
    assert_eq!(f, 16 * 8 + 2, "side quads + two caps");
    assert_eq!(e, 16 * 9 + 16 * 8);
    assert_eq!(m.euler(), 2);
    let vol = m.signed_volume();
    let exact = std::f32::consts::PI * 0.04 * 2.0;
    assert!(vol > 0.0 && (vol - exact).abs() < exact * 0.05, "{vol} vs {exact}");
}

#[test]
fn loft_blends_two_sections_and_scales() {
    let mut l = LoftMod { path_steps: 10, shape_steps: 16, ..Default::default() };
    l.add_section(square(0.5, 100.0));
    l.add_section(LoftSection { at: 0.0, ..LoftSection::circle(0.25, 24) });
    assert_eq!(l.sections[0].name, "Circle", "kept in path order");
    let m = l.build(&path());
    assert_eq!(m.euler(), 2);
    assert!(m.signed_volume() > 0.0);
    let b = m.bounds().unwrap();
    assert!((b.max.x - 0.5).abs() < 0.02 && (b.max.z - 2.0).abs() < 1e-4, "{b:?}");
    l.scale_end = 50.0;
    let half = l.build(&path()).bounds().unwrap();
    assert!(half.max.x < 0.3, "end scaled to 50 %: {half:?}");
    assert!((l.scale_at(0.5) - 0.75).abs() < 1e-5);
}

#[test]
fn loft_twist_and_open_ends() {
    let mut l = LoftMod { path_steps: 6, shape_steps: 8, twist: 90.0, cap_start: false, cap_end: false, ..Default::default() };
    l.add_section(square(0.5, 0.0));
    let m = l.build(&path());
    assert_eq!(m.counts().2, 8 * 6, "no caps");
    assert!(m.vertices.values().all(|v| v.position.is_finite()));
    assert!(l.build(&Shape::default()).polygons.is_empty(), "no path, no mesh");
}

fn cube() -> crate::mesh::EditablePoly {
    Primitive::default_of(PrimitiveKind::Box).to_editable_poly()
}

fn top_selected() -> crate::mesh::EditablePoly {
    let mut m = cube();
    let top = m.polygons.keys().max_by(|a, b| m.poly_centroid(*a).z.total_cmp(&m.poly_centroid(*b).z)).unwrap();
    m.polygons[top].selected = true;
    m
}

#[test]
fn hole_profiles_step_and_cone() {
    let mut s = HoleSpec::default();
    assert_eq!(s.profile(1.0).len(), 4);
    s.kind = HoleKind::Counterbore;
    assert_eq!(s.profile(1.0).len(), 6);
    s.kind = HoleKind::Countersink;
    let p = s.profile(1.0);
    assert_eq!(p.len(), 5);
    assert!(p[1].x > s.top_diameter * 0.5, "the cone carries on above the surface");
    let c = s.cutter(Vec3::ZERO, Vec3::Z, 0.3);
    assert_eq!(c.euler(), 2);
    assert!(c.signed_volume() > 0.0);
}

#[test]
fn drilling_removes_material() {
    let m = top_selected();
    let before = m.signed_volume();
    for kind in HoleKind::ALL {
        let spec = HoleSpec { kind, ..Default::default() };
        let out = hole::drill(&m, &m.selected_polygons(), &spec).expect("drill");
        let v = out.signed_volume();
        assert!(v < before && v > before * 0.8, "{kind:?}: {v} vs {before}");
        assert_eq!(out.euler(), 2, "{kind:?}: a blind hole keeps genus 0");
    }
}

#[test]
fn through_hole_adds_a_handle_and_op_applies() {
    let mut m = top_selected();
    let spec = HoleSpec { through: true, ..Default::default() };
    let out = hole::drill(&m, &m.selected_polygons(), &spec).unwrap();
    assert_eq!(out.euler(), 0, "a through hole makes a torus");
    assert!(PolyOp::Hole { spec }.apply(&mut m));
    assert_eq!(m.euler(), 0);
    let mut none = cube();
    assert!(!PolyOp::Hole { spec }.apply(&mut none), "nothing selected");
    assert!(hole::drill(&none, &[], &spec).is_err());
}
