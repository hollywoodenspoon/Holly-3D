//! Per-frame assembly solve: joints are processed parents-first; each child
//! is re-posed from its parent, after projecting any direct edit of the
//! child onto the joint's free axes (clamped at its stops). Pistons aim last.

use super::frame::{angle_of, swing_twist, twist_angle};
use super::{Frame, Joint, JointKind};
use crate::scene::{ObjId, Scene};
use bevy::math::{Quat, Vec3};

const EPS: f32 = 1e-5;

/// Projects the observed mount-to-mount motion `rel` onto the joint's DOF.
pub fn fit(kind: &mut JointKind, rel: Frame) {
    match kind {
        JointKind::Rigid | JointKind::Piston => {}
        JointKind::Revolute { axis, angle, limits } => {
            let mut a = twist_angle(rel.rot, axis.vec()).to_degrees();
            // Stay on the turn closest to the previous angle (no 360° jumps).
            a += 360.0 * ((*angle - a) / 360.0).round();
            *angle = limits.clamp(a);
        }
        JointKind::Ball { rot, cone, twist } => {
            let (mut swing, tw) = swing_twist(rel.rot, Vec3::Z);
            let s = angle_of(swing).to_degrees();
            if *cone < 180.0 && s > *cone {
                swing = Quat::IDENTITY.slerp(swing, *cone / s);
            }
            let t = twist_angle(tw, Vec3::Z).to_degrees();
            let t = if *twist < 180.0 { t.clamp(-*twist, *twist) } else { t };
            *rot = (swing * Quat::from_axis_angle(Vec3::Z, t.to_radians())).normalize();
        }
        JointKind::Slider { axis, travel, limits } => *travel = limits.clamp(rel.pos.dot(axis.vec())),
    }
}

/// Where the joint puts its child, given the parent's rigid pose.
pub fn child_pose(j: &Joint, parent: Frame) -> Frame {
    parent * j.parent_frame * j.kind.motion() * j.child_frame.inverse()
}

/// Observed motion between the mounts for the given poses.
pub fn observed(j: &Joint, parent: Frame, child: Frame) -> Frame {
    (parent * j.parent_frame).inverse() * child * j.child_frame
}

/// Turns `x` about its mount `m` (local frame) so the mount's Z axis points
/// at the world point `target`.
fn aim(x: Frame, m: Frame, target: Vec3) -> Frame {
    let w = x * m;
    let (cur, want) = (w.rot * Vec3::Z, (target - w.pos).normalize_or_zero());
    if want == Vec3::ZERO || cur.dot(want) > 1.0 - 1e-7 {
        return x;
    }
    let q = Quat::from_rotation_arc(cur, want);
    Frame::new(w.pos + q * (x.pos - w.pos), (q * x.rot).normalize())
}

/// Joint indices, parents before children (depth along the joint tree).
fn order(scene: &Scene) -> Vec<usize> {
    let a = &scene.assembly;
    let depth = |j: &Joint| {
        let mut d = 0;
        let mut at = j.parent;
        while let Some(p) = a.parent_of(at) {
            d += 1;
            at = p;
            if d > a.joints.len() {
                break;
            }
        }
        d
    };
    let mut idx: Vec<usize> = (0..a.joints.len()).filter(|&i| a.joints[i].kind.is_kinematic()).collect();
    idx.sort_by_key(|&i| depth(&a.joints[i]));
    idx
}

impl Scene {
    fn rigid(&self, id: ObjId) -> Frame {
        Frame::of(&self.objects[id].transform)
    }

    fn place(&mut self, id: ObjId, f: Frame) -> bool {
        let x = &mut self.objects[id].transform;
        if Frame::of(x).approx(f, EPS) {
            return false;
        }
        f.write(x);
        true
    }

    /// Enforces every joint; returns true when a part was moved.
    pub fn solve_assembly(&mut self) -> bool {
        let objs = &self.objects;
        self.assembly.joints.retain(|j| j.child != j.parent && objs.contains_key(j.child) && objs.contains_key(j.parent));
        self.assembly.grounded.retain(|&g| objs.contains_key(g));
        self.ik_pass();
        let mut moved = false;
        for i in order(self) {
            let (child, parent) = (self.assembly.joints[i].child, self.assembly.joints[i].parent);
            let (p, c) = (self.rigid(parent), self.rigid(child));
            let j = &mut self.assembly.joints[i];
            // A child that is not where the solver left it was edited
            // directly: read its new joint value from the current pose.
            if j.posed.is_none_or(|(pc, _)| !pc.approx(c, EPS)) {
                let rel = observed(j, p, c);
                fit(&mut j.kind, rel);
            }
            let target = child_pose(j, p);
            j.posed = Some((target, p));
            moved |= self.place(child, target);
        }
        for i in 0..self.assembly.joints.len() {
            let j = self.assembly.joints[i].clone();
            if j.kind.is_kinematic() {
                continue;
            }
            let (h, r) = (self.rigid(j.child), self.rigid(j.parent));
            let (hm, rm) = ((h * j.child_frame).pos, (r * j.parent_frame).pos);
            moved |= self.place(j.child, aim(h, j.child_frame, rm));
            moved |= self.place(j.parent, aim(r, j.parent_frame, hm));
        }
        moved
    }

    /// Adds (or replaces) the joint placing `j.child`, refusing loops.
    pub fn add_joint(&mut self, mut j: Joint) -> Result<(), String> {
        if j.child == j.parent {
            return Err("A part cannot be jointed to itself".into());
        }
        let a = &mut self.assembly;
        if j.kind.is_kinematic() {
            if a.is_ancestor(j.child, j.parent) {
                return Err("That would make a loop: the parent is already carried by this part".into());
            }
            if let Some(i) = a.joint_of(j.child) {
                a.joints.remove(i);
            }
        } else {
            a.joints.retain(|o| o.kind.is_kinematic() || (o.child, o.parent) != (j.child, j.parent));
        }
        j.posed = None;
        a.joints.push(j);
        self.solve_assembly();
        Ok(())
    }
}
