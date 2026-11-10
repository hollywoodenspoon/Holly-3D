//! IK drag: when a jointed part is moved with `W` further than its own joint
//! allows, the joints above it are re-posed (cyclic coordinate descent) so
//! its pivot reaches the dragged position. The chain runs up to the root
//! part or the first Grounded part, and every joint keeps its stops.

use super::solve::{child_pose, fit, observed};
use super::{Frame, JointKind};
use crate::scene::{ObjId, Scene};
use bevy::math::{Quat, Vec3};

const ITER: usize = 24;
const REACHED: f32 = 1e-4;

/// `v` with its component along the unit `axis` removed.
fn reject(v: Vec3, axis: Vec3) -> Vec3 {
    v - axis * v.dot(axis)
}

/// One CCD step: turns or slides joint `kind` (mount frame `w` in world) so
/// the effector `e` moves toward `t`.
fn step(kind: &mut JointKind, w: Frame, e: Vec3, t: Vec3) {
    let cur = kind.motion();
    let want = match kind {
        JointKind::Rigid | JointKind::Piston => return,
        JointKind::Revolute { axis, .. } => {
            let a = w.rot * axis.vec();
            let (ve, vt) = (reject(e - w.pos, a), reject(t - w.pos, a));
            if ve.length() < 1e-5 || vt.length() < 1e-5 {
                return;
            }
            let ang = ve.angle_between(vt) * ve.cross(vt).dot(a).signum();
            Frame::rot(Quat::from_axis_angle(axis.vec(), ang) * cur.rot)
        }
        JointKind::Ball { .. } => {
            let (ve, vt) = (e - w.pos, t - w.pos);
            if ve.length() < 1e-5 || vt.length() < 1e-5 {
                return;
            }
            let q = Quat::from_rotation_arc(ve.normalize(), vt.normalize());
            Frame::rot((w.rot.inverse() * q * w.rot * cur.rot).normalize())
        }
        JointKind::Slider { axis, .. } => Frame::pos(cur.pos + axis.vec() * (t - e).dot(w.rot * axis.vec())),
    };
    fit(kind, want);
}

impl Scene {
    /// Joint indices from `child`'s own joint up to the root or a Grounded part.
    pub fn ik_chain(&self, child: ObjId) -> Vec<usize> {
        let a = &self.assembly;
        let (mut out, mut at) = (vec![], child);
        while !a.grounded.contains(&at) {
            let Some(i) = a.joint_of(at) else { break };
            if out.contains(&i) {
                break;
            }
            out.push(i);
            at = a.joints[i].parent;
        }
        out
    }

    /// Forward kinematics down `chain` from its top part: the parent pose
    /// of each chain joint, and the pose of the chain's end part.
    fn chain_poses(&self, chain: &[usize]) -> (Vec<Frame>, Frame) {
        let top = self.assembly.joints[chain[chain.len() - 1]].parent;
        let mut pose = Frame::of(&self.objects[top].transform);
        let mut parents = vec![Frame::IDENTITY; chain.len()];
        for k in (0..chain.len()).rev() {
            parents[k] = pose;
            pose = child_pose(&self.assembly.joints[chain[k]], pose);
        }
        (parents, pose)
    }

    /// Re-poses the joints above `child` so its pivot reaches `target`
    /// (or gets as close as the joints and their stops allow).
    pub fn ik_reach(&mut self, child: ObjId, target: Vec3) {
        let chain = self.ik_chain(child);
        if chain.is_empty() {
            return;
        }
        for _ in 0..ITER {
            for k in 0..chain.len() {
                let (parents, end) = self.chain_poses(&chain);
                if end.pos.distance(target) < REACHED {
                    return;
                }
                let j = &mut self.assembly.joints[chain[k]];
                let w = parents[k] * j.parent_frame;
                step(&mut j.kind, w, end.pos, target);
            }
        }
    }

    /// Before the solve: a child whose pivot was moved beyond what its own
    /// joint allows drives IK up its chain instead of snapping back.
    pub(super) fn ik_pass(&mut self) {
        if self.assembly.no_ik {
            return;
        }
        for i in 0..self.assembly.joints.len() {
            let j = &self.assembly.joints[i];
            let Some((pc, pp)) = j.posed else { continue };
            let c = Frame::of(&self.objects[j.child].transform);
            if !j.kind.is_kinematic() || pc.pos.distance(c.pos) < 1e-5 {
                continue;
            }
            let p = Frame::of(&self.objects[j.parent].transform);
            let mut own = j.clone();
            fit(&mut own.kind, observed(j, p, c));
            if child_pose(&own, p).pos.distance(c.pos) < REACHED {
                continue; // the part's own joint takes the move (a slider)
            }
            let child = own.child;
            self.assembly.joints[i].kind = own.kind;
            self.ik_reach(child, c.pos);
            // The solve below poses the chain from the new joint values.
            self.assembly.joints[i].posed = Some((c, pp));
        }
    }
}
