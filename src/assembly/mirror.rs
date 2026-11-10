//! Mirror Mech Limb: joint frames and values for a limb cloned across a
//! world plane. A reflection is not a rigid frame, so each mirrored mount
//! frame also flips its own X axis (keeping the face normal Z): hinge angles
//! about mount Y / Z and slider travel along X change sign, and stops swap.

use super::{Axis, Frame, Joint, JointKind, Limits};
use bevy::math::{Mat3, Mat4, Quat, Vec3};

/// Flip of the mount's own X axis.
fn flip_x() -> Mat4 {
    Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))
}

fn mat(f: Frame) -> Mat4 {
    Mat4::from_rotation_translation(f.rot, f.pos)
}

/// The rigid frame of a proper (determinant +1) rigid matrix.
fn frame(m: Mat4) -> Frame {
    Frame::new(m.w_axis.truncate(), Quat::from_mat3(&Mat3::from_mat4(m)).normalize())
}

/// World mount frame `w` reflected by `refl` (kept right-handed).
pub fn mirror_frame(w: Frame, refl: Mat4) -> Frame {
    frame(refl * mat(w) * flip_x())
}

fn negated(l: Limits) -> Limits {
    Limits { on: l.on, min: -l.max, max: -l.min }
}

/// The joint value seen through the mount X flip.
pub fn mirror_kind(k: &JointKind) -> JointKind {
    match k.clone() {
        JointKind::Revolute { axis, angle, limits } if axis != Axis::X => {
            JointKind::Revolute { axis, angle: -angle, limits: negated(limits) }
        }
        JointKind::Slider { axis: Axis::X, travel, limits } => {
            JointKind::Slider { axis: Axis::X, travel: -travel, limits: negated(limits) }
        }
        JointKind::Ball { rot, cone, twist } => JointKind::Ball { rot: Quat::from_xyzw(rot.x, -rot.y, -rot.z, rot.w), cone, twist },
        other => other,
    }
}

/// Joint `j` mirrored by `refl`. `old` holds the (child, parent) rigid poses
/// before mirroring, `new` the poses of the parts the new joint links.
pub fn mirror_joint(j: &Joint, refl: Mat4, old: (Frame, Frame), new: (Frame, Frame)) -> Joint {
    let wc = mirror_frame(old.0 * j.child_frame, refl);
    let wp = mirror_frame(old.1 * j.parent_frame, refl);
    Joint {
        child: j.child,
        parent: j.parent,
        kind: mirror_kind(&j.kind),
        child_frame: new.0.inverse() * wc,
        parent_frame: new.1.inverse() * wp,
        posed: None,
    }
}
