//! What a gizmo drag does: cursor travel to a move offset, an angle or a
//! scale factor, and that change applied to objects and frames. Pure math,
//! covered by headless tests.

use crate::scene::Xform;
use crate::viewport::camera::Ray;
use bevy::math::{Mat3, Mat4, Quat, Vec2, Vec3};

/// The change a drag makes, about the gizmo centre and in its axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Delta {
    Move(Vec3),
    /// Axis (world, unit) and angle in radians.
    Rotate(Vec3, f32),
    /// Factors along the gizmo's own X, Y, Z axes.
    Scale(Vec3),
}

impl Delta {
    /// World matrix of the change about `center`, with gizmo rotation `rot`.
    pub fn matrix(&self, center: Vec3, rot: Quat) -> Mat4 {
        let about = |m: Mat4| Mat4::from_translation(center) * m * Mat4::from_translation(-center);
        match *self {
            Delta::Move(d) => Mat4::from_translation(d),
            Delta::Rotate(axis, a) => about(Mat4::from_quat(Quat::from_axis_angle(axis, a))),
            Delta::Scale(s) => about(Mat4::from_quat(rot) * Mat4::from_scale(s) * Mat4::from_quat(rot.inverse())),
        }
    }

    /// The change applied to one object transform.
    pub fn apply(&self, x: &Xform, center: Vec3, rot: Quat) -> Xform {
        match *self {
            Delta::Move(d) => Xform { translation: x.translation + d, ..*x },
            Delta::Rotate(axis, a) => {
                let q = Quat::from_axis_angle(axis, a);
                Xform { translation: center + q * (x.translation - center), rotation: (q * x.rotation).normalize(), scale: x.scale }
            }
            Delta::Scale(s) => {
                let m = Mat3::from_quat(rot) * Mat3::from_diagonal(s) * Mat3::from_quat(rot.inverse());
                // How much each of the object's own axes stretches.
                let f = Vec3::from_array(std::array::from_fn(|i| (m * (x.rotation * Vec3::AXES[i])).length()));
                Xform { translation: center + m * (x.translation - center), rotation: x.rotation, scale: x.scale * f }
            }
        }
    }
}

/// Distance along `axis` (from `c`) of the point closest to the ray.
pub fn axis_param(ray: &Ray, c: Vec3, axis: Vec3) -> Option<f32> {
    let w0 = c - ray.origin;
    let (b, d, e) = (axis.dot(ray.dir), axis.dot(w0), ray.dir.dot(w0));
    let denom = 1.0 - b * b;
    (denom > 1e-4).then(|| (b * e - d) / denom)
}

/// Where the ray meets the plane through `c` with normal `n`.
pub fn plane_hit(ray: &Ray, c: Vec3, n: Vec3) -> Option<Vec3> {
    let d = ray.dir.dot(n);
    if d.abs() < 1e-5 {
        return None;
    }
    let t = (c - ray.origin).dot(n) / d;
    t.is_finite().then(|| ray.at(t))
}

/// Rounds to the nearest multiple of `step` (no-op for step <= 0).
pub fn snap(v: f32, step: f32) -> f32 {
    if step > 0.0 { (v / step).round() * step } else { v }
}

/// Snaps a move offset along each gizmo axis.
pub fn snap_move(d: Vec3, rot: Quat, step: f32) -> Vec3 {
    let l = rot.inverse() * d;
    rot * Vec3::new(snap(l.x, step), snap(l.y, step), snap(l.z, step))
}

/// Axis ring: cursor travel along the ring's screen tangent, one radian
/// per ring radius.
pub fn ring_angle(grab: Vec2, cur: Vec2, tangent: Vec2, radius_px: f32) -> f32 {
    (cur - grab).dot(tangent.normalize_or_zero()) / radius_px.max(1.0)
}

/// View ring: signed screen angle swept around the centre. Screen y
/// points down, so a clockwise sweep is positive, which is a positive
/// turn about the camera's forward axis.
pub fn view_angle(c: Vec2, grab: Vec2, cur: Vec2) -> f32 {
    let (a, b) = (grab - c, cur - c);
    a.perp_dot(b).atan2(a.dot(b))
}

/// Trackball: horizontal travel turns about the camera's up axis, vertical
/// travel about its right axis. Returns the rotation as axis + angle.
pub fn trackball(right: Vec3, up: Vec3, delta: Vec2, radius_px: f32) -> (Vec3, f32) {
    let r = radius_px.max(1.0);
    let q = Quat::from_axis_angle(right, delta.y / r) * Quat::from_axis_angle(up, delta.x / r);
    let (axis, a) = q.to_axis_angle();
    if a.abs() < 1e-6 { (up, 0.0) } else { (axis, a) }
}

/// Scale factor from travel along a screen direction (one axis length of
/// travel doubles the size). Never reaches zero.
pub fn scale_factor(grab: Vec2, cur: Vec2, dir: Vec2, len_px: f32) -> f32 {
    (1.0 + (cur - grab).dot(dir.normalize_or_zero()) / len_px.max(1.0)).max(0.01)
}

/// Uniform scale: dragging up grows, down shrinks.
pub fn uniform_factor(grab: Vec2, cur: Vec2, len_px: f32) -> f32 {
    (1.0 + (grab.y - cur.y) / len_px.max(1.0)).max(0.01)
}
