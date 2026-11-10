//! Spline Boolean tests: squares (exact areas), circles (curves kept),
//! disjoint, nested and open splines.

use super::boolean::SplineBoolOp;
use super::*;
use bevy::math::{Mat4, Vec3};

fn square(x: f32, y: f32, size: f32) -> Spline {
    let pts = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)];
    Spline {
        knots: pts.iter().map(|&(a, b)| Knot::corner(Vec3::new(x + a * size, y + b * size, 0.0))).collect(),
        closed: true,
        selected: false,
    }
}

fn circle(x: f32, r: f32) -> Spline {
    let mut s = ShapeParams::default_of(ShapeKind::Circle).unwrap().to_shape();
    let k = r / 0.6;
    s.transform(Mat4::from_translation(Vec3::X * x) * Mat4::from_scale(Vec3::splat(k)));
    s.splines.remove(0)
}

fn two(a: Spline, b: Spline) -> Shape {
    Shape { splines: vec![a, b], steps: 12 }
}

fn areas(s: &Shape) -> Vec<f32> {
    s.polylines().iter().map(|p| p.signed_area_xy()).collect()
}

#[test]
fn square_union_subtract_intersect() {
    let cases = [(SplineBoolOp::Union, 7.0, 8), (SplineBoolOp::Subtraction, 3.0, 6), (SplineBoolOp::Intersection, 1.0, 4)];
    for (op, want, knots) in cases {
        let mut s = two(square(0.0, 0.0, 2.0), square(1.0, 1.0, 2.0));
        assert!(s.spline_boolean(0, 1, op), "{op:?}");
        assert_eq!(s.splines.len(), 1, "{op:?}");
        assert!(s.splines[0].closed);
        assert_eq!(s.knot_count(), knots, "{op:?}");
        let a = areas(&s)[0];
        assert!((a - want).abs() < 1e-3, "{op:?}: {a}");
    }
    // B - A with the operands swapped
    let mut s = two(square(1.0, 1.0, 2.0), square(0.0, 0.0, 2.0));
    assert!(s.spline_boolean(1, 0, SplineBoolOp::Subtraction));
    assert!((areas(&s)[0] - 3.0).abs() < 1e-3);
    // clockwise input is handled
    let mut cw = square(0.0, 0.0, 2.0);
    cw.reverse();
    let mut s = two(cw, square(1.0, 1.0, 2.0));
    assert!(s.spline_boolean(0, 1, SplineBoolOp::Union));
    assert!((areas(&s)[0] - 7.0).abs() < 1e-3);
}

#[test]
fn circle_lens_keeps_its_curves() {
    let mut s = two(circle(0.0, 1.0), circle(1.0, 1.0));
    assert!(s.spline_boolean(0, 1, SplineBoolOp::Intersection));
    assert_eq!(s.splines.len(), 1);
    let lens = 2.0 * (0.5f32).acos() - 0.5 * 3f32.sqrt();
    let a = areas(&s)[0];
    assert!((a - lens).abs() < lens * 0.02, "lens {a} vs {lens}");
    assert!(s.splines[0].knots.iter().any(|k| k.kind == KnotType::BezierCorner), "curves kept");
    for p in &s.polylines()[0].pts {
        let (d0, d1) = (p.truncate().length(), (p.truncate() - bevy::math::Vec2::X).length());
        assert!(d0 < 1.0 + 2e-3 && d1 < 1.0 + 2e-3, "inside both circles: {p}");
    }
    let mut u = two(circle(0.0, 1.0), circle(1.0, 1.0));
    assert!(u.spline_boolean(0, 1, SplineBoolOp::Union));
    let pi = std::f32::consts::PI;
    assert!((areas(&u)[0] - (2.0 * pi - lens)).abs() < 0.05, "{}", areas(&u)[0]);
}

#[test]
fn disjoint_nested_and_open() {
    let mut d = two(square(0.0, 0.0, 1.0), square(3.0, 0.0, 1.0));
    assert!(d.spline_boolean(0, 1, SplineBoolOp::Union));
    assert_eq!(d.splines.len(), 2, "both kept");
    let mut i = two(square(0.0, 0.0, 1.0), square(3.0, 0.0, 1.0));
    assert!(i.spline_boolean(0, 1, SplineBoolOp::Intersection));
    assert!(i.splines.is_empty());
    // a hole: outer counter-clockwise, inner clockwise
    let mut h = two(square(0.0, 0.0, 4.0), square(1.0, 1.0, 1.0));
    assert!(h.spline_boolean(0, 1, SplineBoolOp::Subtraction));
    let mut a = areas(&h);
    a.sort_by(f32::total_cmp);
    assert_eq!(a.len(), 2);
    assert!((a[0] + 1.0).abs() < 1e-3 && (a[1] - 16.0).abs() < 1e-3, "{a:?}");
    let mut open = two(square(0.0, 0.0, 1.0), square(0.5, 0.5, 1.0));
    open.splines[1].closed = false;
    let before = open.clone();
    assert!(!open.spline_boolean(0, 1, SplineBoolOp::Union));
    assert!(!open.spline_boolean(0, 0, SplineBoolOp::Union));
    assert_eq!(open, before);
}
