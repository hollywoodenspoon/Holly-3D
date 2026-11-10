//! Bone-free mech assembly: parts are linked by mechanical joints instead
//! of a skeleton. A joint pins a child part to a parent part at a mount
//! frame and allows only its degrees of freedom (Rigid, Revolute, Ball,
//! Slider); a Piston joint aims a housing and a rod at each other.
//!
//! Object transforms stay in world space. Every frame `solve` re-poses each
//! child from its parent and the joint value; when the user moved or turned
//! a child directly, the new pose is first projected onto the joint's free
//! axes and clamped at its stops (forward kinematics); a `W` move beyond
//! what the joint allows re-poses the chain above it (IK, see `ik`).

pub mod frame;
pub mod ik;
pub mod mirror;
pub mod mount;
pub mod solve;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_ik;

use crate::scene::ObjId;
use bevy::math::{Quat, Vec3};
pub use frame::Frame;

/// Local joint axis (in the mount frame; Z is the mount face normal).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Axis {
    X,
    Y,
    #[default]
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn vec(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
        }
    }
}

/// Min / max stops of a 1-DOF joint (degrees or scene units).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Limits {
    pub on: bool,
    pub min: f32,
    pub max: f32,
}

impl Limits {
    pub fn clamp(&self, v: f32) -> f32 {
        if self.on { v.clamp(self.min.min(self.max), self.max.max(self.min)) } else { v }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum JointKind {
    /// Bolted 1:1 to the parent.
    Rigid,
    /// Hinge: spins about one axis (`angle` in degrees).
    Revolute { axis: Axis, angle: f32, limits: Limits },
    /// Socket: any rotation within a swing cone about Z and a twist range.
    Ball { rot: Quat, cone: f32, twist: f32 },
    /// Rail: slides along one axis (`travel` in scene units).
    Slider { axis: Axis, travel: f32, limits: Limits },
    /// Telescoping piston: the child (housing) and parent (rod) turn about
    /// their mounts so their Z axes point at each other's mount.
    Piston,
}

/// Joint types offered by the J tool and the Assembly rollout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JointType {
    Rigid,
    #[default]
    Revolute,
    Ball,
    Slider,
    Piston,
}

impl JointType {
    pub const ALL: [JointType; 5] = [JointType::Rigid, JointType::Revolute, JointType::Ball, JointType::Slider, JointType::Piston];

    pub fn label(self) -> &'static str {
        match self {
            JointType::Rigid => "Rigid (Welded)",
            JointType::Revolute => "Revolute / Hinge",
            JointType::Ball => "Ball Socket",
            JointType::Slider => "Slider / Rail",
            JointType::Piston => "Hydraulic Piston",
        }
    }

    /// The joint at rest (zero angle / travel, generous stops).
    pub fn rest(self) -> JointKind {
        let deg = Limits { on: false, min: -90.0, max: 90.0 };
        match self {
            JointType::Rigid => JointKind::Rigid,
            JointType::Revolute => JointKind::Revolute { axis: Axis::Z, angle: 0.0, limits: deg },
            JointType::Ball => JointKind::Ball { rot: Quat::IDENTITY, cone: 45.0, twist: 30.0 },
            JointType::Slider => JointKind::Slider { axis: Axis::Z, travel: 0.0, limits: Limits { on: false, min: 0.0, max: 1.0 } },
            JointType::Piston => JointKind::Piston,
        }
    }
}

impl JointKind {
    pub fn kind(&self) -> JointType {
        match self {
            JointKind::Rigid => JointType::Rigid,
            JointKind::Revolute { .. } => JointType::Revolute,
            JointKind::Ball { .. } => JointType::Ball,
            JointKind::Slider { .. } => JointType::Slider,
            JointKind::Piston => JointType::Piston,
        }
    }

    /// Pistons aim; every other joint places its child.
    pub fn is_kinematic(&self) -> bool {
        !matches!(self, JointKind::Piston)
    }

    /// The joint's motion from the parent mount to the child mount.
    pub fn motion(&self) -> Frame {
        match self {
            JointKind::Rigid | JointKind::Piston => Frame::IDENTITY,
            JointKind::Revolute { axis, angle, .. } => Frame::rot(Quat::from_axis_angle(axis.vec(), angle.to_radians())),
            JointKind::Ball { rot, .. } => Frame::rot(*rot),
            JointKind::Slider { axis, travel, .. } => Frame::pos(axis.vec() * *travel),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Joint {
    pub child: ObjId,
    pub parent: ObjId,
    pub kind: JointKind,
    /// Mount frame in the parent's pivot space (scale ignored).
    pub parent_frame: Frame,
    /// Mount frame in the child's pivot space.
    pub child_frame: Frame,
    /// Child and parent poses the solver last wrote (detects direct edits).
    #[serde(skip)]
    pub posed: Option<(Frame, Frame)>,
}

/// All joints of a scene (saved with it).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assembly {
    pub joints: Vec<Joint>,
    /// Grounded parts: IK chains stop here (the part never moves for IK).
    #[serde(default)]
    pub grounded: Vec<ObjId>,
    /// IK Drag off: a `W` move of a jointed part is only projected onto
    /// its own joint.
    #[serde(default)]
    pub no_ik: bool,
}

impl Assembly {
    /// The joint that places `child` (its link to its parent).
    pub fn joint_of(&self, child: ObjId) -> Option<usize> {
        self.joints.iter().position(|j| j.child == child && j.kind.is_kinematic())
    }

    pub fn parent_of(&self, child: ObjId) -> Option<ObjId> {
        self.joint_of(child).map(|i| self.joints[i].parent)
    }

    /// True when `a` is `b` or one of `b`'s ancestors.
    pub fn is_ancestor(&self, a: ObjId, b: ObjId) -> bool {
        let mut at = Some(b);
        for _ in 0..=self.joints.len() {
            match at {
                Some(x) if x == a => return true,
                Some(x) => at = self.parent_of(x),
                None => return false,
            }
        }
        false
    }

    /// Parts placed by `id`'s joints, recursively (children first-level order).
    pub fn descendants(&self, id: ObjId) -> Vec<ObjId> {
        let mut out = vec![];
        let mut stack = vec![id];
        while let Some(p) = stack.pop() {
            for j in self.joints.iter().filter(|j| j.parent == p && j.kind.is_kinematic()) {
                if !out.contains(&j.child) && j.child != id {
                    out.push(j.child);
                    stack.push(j.child);
                }
            }
        }
        out
    }
}
