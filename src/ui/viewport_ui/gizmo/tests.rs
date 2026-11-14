//! Headless tests for gizmo picking and drag math.

use super::drag_math::{self, Delta};
use super::handles::{Gz, Handle, VIEW_RING};
use crate::editor::TransformMode;
use crate::scene::Xform;
use crate::viewport::camera::ViewKind;
use crate::viewport::layout::{ScreenRect, ViewState};
use bevy::math::{Quat, Vec2, Vec3};

fn view(kind: ViewKind) -> ViewState {
    let mut v = ViewState::new(kind);
    v.rect = ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) };
    v
}

fn gz(v: &ViewState) -> Gz<'_> {
    Gz::new(v, Vec3::ZERO, Quat::IDENTITY, 80.0).expect("gizmo on screen")
}

#[test]
fn move_picks_arrows_planes_and_centre() {
    let v = view(ViewKind::Perspective);
    let g = gz(&v);
    for k in 0..3 {
        assert_eq!(g.pick(TransformMode::Move, g.along(k, 0.95)), Some(Handle::Axis(k)), "axis {k}");
    }
    let sq = g.move_square(2);
    let mid = (sq[0] + sq[1] + sq[2] + sq[3]) / 4.0;
    assert_eq!(g.pick(TransformMode::Move, mid), Some(Handle::Plane(2)));
    assert_eq!(g.pick(TransformMode::Move, g.c2), Some(Handle::Screen));
    assert_eq!(g.pick(TransformMode::Move, g.c2 + Vec2::new(300.0, 250.0)), None);
}

#[test]
fn top_view_hides_the_axis_pointing_at_the_viewer() {
    let v = view(ViewKind::Top);
    let g = gz(&v);
    assert!(!g.axis_shown(2), "Z points at the viewer in Top");
    assert!(g.axis_shown(0) && g.axis_shown(1));
    assert!(g.plane_shown(2) && !g.plane_shown(0) && !g.plane_shown(1));
}

#[test]
fn rotate_picks_rings_view_ring_and_trackball() {
    let v = view(ViewKind::Top);
    let g = gz(&v);
    // In Top the Z ring is a circle of the full ring radius.
    let r = g.ring_px();
    let on_z = g.c2 + Vec2::splat(r * std::f32::consts::FRAC_1_SQRT_2);
    assert_eq!(g.pick(TransformMode::Rotate, on_z), Some(Handle::Axis(2)));
    assert_eq!(g.pick(TransformMode::Rotate, g.c2 + Vec2::new(-r * 0.3, r * 0.4)), Some(Handle::Trackball));
    let a = 1.0f32;
    let on_view = g.c2 + Vec2::new(a.cos(), a.sin()) * r * VIEW_RING;
    assert_eq!(g.pick(TransformMode::Rotate, on_view), Some(Handle::ViewRing));
    assert_eq!(g.pick(TransformMode::Rotate, g.c2 + Vec2::splat(r * 2.0)), None);
}

#[test]
fn back_of_a_ring_loses_to_the_front() {
    let v = view(ViewKind::Perspective);
    let g = gz(&v);
    for k in 0..3 {
        if let Some((d, t)) = g.ring_hit(k, g.c2 + Vec2::new(0.0, -g.ring_px())) {
            assert!(g.front(g.ring_point(k, t)), "ring {k} hit a back point at {d}px");
        }
    }
}

#[test]
fn scale_picks_uniform_triangle_strips_and_axes() {
    let v = view(ViewKind::Perspective);
    let g = gz(&v);
    let tri = g.scale_triangle();
    assert_eq!(g.pick(TransformMode::Scale, (tri[0] + tri[1] + tri[2]) / 3.0), Some(Handle::Uniform));
    let s = g.scale_strip(2);
    assert_eq!(g.pick(TransformMode::Scale, (s[0] + s[1] + s[2] + s[3]) / 4.0), Some(Handle::Plane(2)));
    assert_eq!(g.pick(TransformMode::Scale, g.along(0, 0.95)), Some(Handle::Axis(0)));
}

#[test]
fn axis_move_follows_the_cursor_along_that_axis_only() {
    let v = view(ViewKind::Perspective);
    let g = gz(&v);
    let ray0 = v.ray(g.along(0, 0.9));
    let ray1 = v.ray(g.along(0, 0.9) + Vec2::new(40.0, 25.0));
    let s0 = drag_math::axis_param(&ray0, Vec3::ZERO, Vec3::X).unwrap();
    let s1 = drag_math::axis_param(&ray1, Vec3::ZERO, Vec3::X).unwrap();
    assert!((s0 - g.len * 0.9).abs() < g.len * 0.02, "grab lands on the arrow: {s0} vs {}", g.len * 0.9);
    let x = Delta::Move(Vec3::X * (s1 - s0)).apply(&Xform::default(), Vec3::ZERO, Quat::IDENTITY);
    assert!(x.translation.y == 0.0 && x.translation.z == 0.0 && x.translation.x != 0.0);
}

#[test]
fn plane_move_stays_in_the_plane_and_snaps_per_axis() {
    let v = view(ViewKind::Perspective);
    let ray = v.ray(Vec2::new(500.0, 350.0));
    let h = drag_math::plane_hit(&ray, Vec3::ZERO, Vec3::Z).unwrap();
    assert!(h.z.abs() < 1e-4);
    let m = drag_math::snap_move(Vec3::new(0.37, -0.62, 0.0), Quat::IDENTITY, 0.25);
    assert!((m - Vec3::new(0.25, -0.5, 0.0)).length() < 1e-6);
}

#[test]
fn rotation_angles_and_snap() {
    // A quarter of the ring radius along the tangent = 0.25 rad.
    let a = drag_math::ring_angle(Vec2::ZERO, Vec2::new(20.0, 0.0), Vec2::X, 80.0);
    assert!((a - 0.25).abs() < 1e-6);
    let s = drag_math::snap(a, 5f32.to_radians());
    assert!((s.to_degrees() - 15.0).abs() < 1e-4, "{}", s.to_degrees());
    // Clockwise quarter turn on screen (y down) is +90°.
    let c = Vec2::ZERO;
    let q = drag_math::view_angle(c, Vec2::new(10.0, 0.0), Vec2::new(0.0, 10.0));
    assert!((q - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    // Rotating the X ring's start point by the angle moves it along the tangent.
    let x = Delta::Rotate(Vec3::Z, 0.3).apply(&Xform { translation: Vec3::X, ..Default::default() }, Vec3::ZERO, Quat::IDENTITY);
    assert!(x.translation.y > 0.0 && (x.translation.length() - 1.0).abs() < 1e-5);
}

#[test]
fn trackball_turns_the_front_towards_the_drag() {
    let v = view(ViewKind::Perspective);
    let cam = &v.camera;
    let front = -cam.forward();
    let (axis, a) = drag_math::trackball(cam.right(), cam.up(), Vec2::new(30.0, 0.0), 80.0);
    let moved = Quat::from_axis_angle(axis, a) * front;
    assert!(moved.dot(cam.right()) > 0.1, "dragging right turns the front to the right");
    let (axis, a) = drag_math::trackball(cam.right(), cam.up(), Vec2::new(0.0, 30.0), 80.0);
    let moved = Quat::from_axis_angle(axis, a) * front;
    assert!(moved.dot(cam.up()) < -0.1, "dragging down turns the front down");
}

#[test]
fn scale_factors_and_object_scaling() {
    assert!((drag_math::scale_factor(Vec2::ZERO, Vec2::new(40.0, 0.0), Vec2::X, 80.0) - 1.5).abs() < 1e-6);
    assert!((drag_math::uniform_factor(Vec2::ZERO, Vec2::new(0.0, -80.0), 80.0) - 2.0).abs() < 1e-6);
    assert!(drag_math::scale_factor(Vec2::ZERO, Vec2::new(-500.0, 0.0), Vec2::X, 80.0) > 0.0);
    let x = Xform { translation: Vec3::new(1.0, 2.0, 0.0), scale: Vec3::new(1.0, 2.0, 3.0), ..Default::default() };
    let u = Delta::Scale(Vec3::splat(2.0)).apply(&x, Vec3::ZERO, Quat::IDENTITY);
    assert!((u.scale - Vec3::new(2.0, 4.0, 6.0)).length() < 1e-5, "uniform keeps the ratios");
    assert!((u.translation - Vec3::new(2.0, 4.0, 0.0)).length() < 1e-5);
    // A local-X scale on an object turned 90° about Z stretches its own X.
    let r = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let t = Xform { rotation: r, ..Default::default() };
    let s = Delta::Scale(Vec3::new(3.0, 1.0, 1.0)).apply(&t, Vec3::ZERO, r);
    assert!((s.scale - Vec3::new(3.0, 1.0, 1.0)).length() < 1e-4, "{:?}", s.scale);
    // Frames: the delta matrix about the centre leaves the centre in place.
    let m = Delta::Scale(Vec3::splat(2.0)).matrix(Vec3::ONE, Quat::IDENTITY);
    assert!((m.transform_point3(Vec3::ONE) - Vec3::ONE).length() < 1e-6);
}
