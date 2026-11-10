//! Rigid frames (rotation + translation) for joint mounts.

use crate::scene::Xform;
use bevy::math::{Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    pub pos: Vec3,
    pub rot: Quat,
}

impl Default for Frame {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl std::ops::Mul for Frame {
    type Output = Frame;
    fn mul(self, o: Frame) -> Frame {
        Frame { pos: self.pos + self.rot * o.pos, rot: (self.rot * o.rot).normalize() }
    }
}

impl Frame {
    pub const IDENTITY: Frame = Frame { pos: Vec3::ZERO, rot: Quat::IDENTITY };

    pub fn new(pos: Vec3, rot: Quat) -> Self {
        Self { pos, rot }
    }
    pub fn pos(pos: Vec3) -> Self {
        Self { pos, rot: Quat::IDENTITY }
    }
    pub fn rot(rot: Quat) -> Self {
        Self { pos: Vec3::ZERO, rot }
    }

    /// The rigid part of an object transform (its scale is ignored).
    pub fn of(x: &Xform) -> Self {
        Self { pos: x.translation, rot: x.rotation }
    }

    /// Writes this frame into `x`, keeping its scale.
    pub fn write(self, x: &mut Xform) {
        x.translation = self.pos;
        x.rotation = self.rot;
    }

    pub fn inverse(self) -> Self {
        let r = self.rot.inverse();
        Self { pos: -(r * self.pos), rot: r }
    }

    pub fn point(self, p: Vec3) -> Vec3 {
        self.pos + self.rot * p
    }

    pub fn approx(self, o: Frame, eps: f32) -> bool {
        // q and -q are the same turn; compare components (about half the
        // angle), which stays above f32 noise unlike `1 - dot`.
        let q = if self.rot.dot(o.rot) < 0.0 { -o.rot } else { o.rot };
        self.pos.distance(o.pos) <= eps && (self.rot - q).length() <= eps
    }

    /// Frame at `pos` whose Z axis is `normal` (any stable roll).
    pub fn from_normal(pos: Vec3, normal: Vec3) -> Self {
        Self { pos, rot: Quat::from_rotation_arc(Vec3::Z, normal.normalize_or(Vec3::Z)) }
    }
}

/// Signed rotation (radians) of `q` about the unit `axis` (swing-twist).
pub fn twist_angle(q: Quat, axis: Vec3) -> f32 {
    let p = Vec3::new(q.x, q.y, q.z).dot(axis);
    let a = 2.0 * p.atan2(q.w);
    // Wrap into (-π, π].
    if a > std::f32::consts::PI {
        a - std::f32::consts::TAU
    } else if a <= -std::f32::consts::PI {
        a + std::f32::consts::TAU
    } else {
        a
    }
}

/// Splits `q` into (swing, twist) about `axis`: `q = swing * twist`.
pub fn swing_twist(q: Quat, axis: Vec3) -> (Quat, Quat) {
    let t = Quat::from_axis_angle(axis, twist_angle(q, axis));
    ((q * t.inverse()).normalize(), t)
}

/// Rotation angle (radians, 0..=π) of a unit quaternion.
pub fn angle_of(q: Quat) -> f32 {
    2.0 * q.w.abs().clamp(0.0, 1.0).acos()
}
