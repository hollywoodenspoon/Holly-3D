//! One-shot sketch constraints on Editable Spline selections.

use super::constraints::SplineConstraint as C;
use super::*;
use bevy::math::Vec3;

fn poly(pts: &[Vec3], closed: bool) -> Spline {
    Spline { knots: pts.iter().map(|&p| Knot::corner(p)).collect(), closed, selected: false }
}

/// Two separate lines: the reference along X, the other tilted.
fn two_lines() -> Shape {
    let mut s = Shape { splines: vec![poly(&[Vec3::ZERO, Vec3::X * 2.0], false), poly(&[Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 0.0)], false)], steps: 6 };
    s.splines[0].knots[0].seg_selected = true;
    s.splines[1].knots[0].seg_selected = true;
    s
}

fn dir(s: &Shape, si: usize) -> Vec3 {
    (s.splines[si].knots[1].pos - s.splines[si].knots[0].pos).normalize()
}

fn len(s: &Shape, si: usize) -> f32 {
    s.splines[si].knots[1].pos.distance(s.splines[si].knots[0].pos)
}

#[test]
fn parallel_turns_about_the_midpoint() {
    let mut s = two_lines();
    let mid = (s.splines[1].knots[0].pos + s.splines[1].knots[1].pos) * 0.5;
    assert_eq!(s.constrain(C::Parallel), 1);
    assert!(dir(&s, 1).abs_diff_eq(Vec3::X, 1e-5), "{:?}", dir(&s, 1));
    assert!((len(&s, 1) - 2f32.sqrt()).abs() < 1e-5, "length kept");
    assert!(((s.splines[1].knots[0].pos + s.splines[1].knots[1].pos) * 0.5).abs_diff_eq(mid, 1e-5));
    assert_eq!(s.splines[0].knots[1].pos, Vec3::X * 2.0, "reference untouched");
    assert_eq!(s.constrain(C::Parallel), 0, "already parallel");
}

#[test]
fn perpendicular_and_equal_length() {
    let mut s = two_lines();
    assert_eq!(s.constrain(C::Perpendicular), 1);
    assert!(dir(&s, 1).dot(Vec3::X).abs() < 1e-5 && dir(&s, 1).y > 0.0);
    assert_eq!(s.constrain(C::EqualLength), 1);
    assert!((len(&s, 1) - 2.0).abs() < 1e-5);
    assert!(dir(&s, 1).dot(Vec3::X).abs() < 1e-5, "stretching keeps the direction");
}

#[test]
fn shared_vertex_is_the_pivot() {
    // An L whose second leg is tilted: perpendicular pivots on the corner.
    let mut s = Shape { splines: vec![poly(&[Vec3::ZERO, Vec3::X, Vec3::new(1.5, 1.0, 0.0)], false)], steps: 6 };
    s.splines[0].knots[0].seg_selected = true;
    s.splines[0].knots[1].seg_selected = true;
    assert_eq!(s.selected_segments(), vec![(0, 0), (0, 1)]);
    assert_eq!(s.constrain(C::Perpendicular), 1);
    let k = &s.splines[0].knots;
    assert_eq!((k[0].pos, k[1].pos), (Vec3::ZERO, Vec3::X), "reference and corner stay");
    assert!((k[2].pos - Vec3::X).normalize().abs_diff_eq(Vec3::Y, 1e-5), "{:?}", k[2].pos);
    assert!((k[2].pos.distance(Vec3::X) - 1.25f32.sqrt()).abs() < 1e-5);
}

#[test]
fn tangent_smooths_a_corner() {
    let mut s = Shape { splines: vec![poly(&[Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0)], false)], steps: 6 };
    s.splines[0].knots[1].selected = true;
    s.splines[0].knots[0].selected = true; // an open end is skipped
    assert_eq!(s.constrain(C::Tangent), 1);
    let k = &s.splines[0].knots[1];
    assert_eq!(k.kind, KnotType::BezierCorner);
    assert!((k.out_h - k.pos).normalize().abs_diff_eq(Vec3::X, 1e-5), "leaves along +X like it arrived");
    assert!((k.pos - k.in_h).normalize().abs_diff_eq(Vec3::X, 1e-5));
    let pl = s.polylines().remove(0);
    assert!(pl.pts.iter().all(|p| p.is_finite()));
}

#[test]
fn concentric_moves_splines_onto_the_first_centre() {
    let circle = |c: Vec3, r: f32| poly(&[c + Vec3::X * r, c + Vec3::Y * r, c - Vec3::X * r, c - Vec3::Y * r], true);
    let mut s = Shape { splines: vec![circle(Vec3::new(1.0, 2.0, 0.0), 1.0), circle(Vec3::new(-3.0, 0.5, 0.0), 0.4), circle(Vec3::ZERO, 2.0)], steps: 6 };
    s.splines[0].selected = true;
    s.splines[1].selected = true;
    assert_eq!(s.constrain(C::Concentric), 1);
    let c1 = s.splines[1].knots.iter().map(|k| k.pos).sum::<Vec3>() / 4.0;
    assert!(c1.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-5));
    assert!((s.splines[1].knots[0].pos - c1).length() - 0.4 < 1e-5, "size kept");
    assert_eq!(s.splines[2].knots[0].pos, Vec3::X * 2.0, "unselected untouched");
    assert_eq!(Shape::default().constrain(C::Concentric), 0);
}
