//! Two-bone analytic IK for Easy Pose: moves a hand or foot to a target by
//! bending the elbow / knee to the right reach and swinging the whole limb
//! about the shoulder / hip. The result is stored as ordinary per-bone
//! angles on the upper and lower limb bones.

use super::hands::euler_deg;
use super::pose::Pose;
use super::{bone_matrices, skeleton, Skeleton};
use bevy::math::{Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limb {
    Arm,
    Leg,
}

impl Limb {
    /// (upper bone, lower bone, end bone) on the `.L` or `.R` side.
    pub fn chain(self, right: bool) -> [String; 3] {
        let s = if right { "R" } else { "L" };
        let names = match self {
            Limb::Arm => ["upperarm01", "lowerarm01", "wrist"],
            Limb::Leg => ["upperleg01", "lowerleg01", "foot"],
        };
        names.map(|n| format!("{n}.{s}"))
    }
}

fn rot(m: &Mat4) -> Quat {
    let (_, r, _) = m.to_scale_rotation_translation();
    r.normalize()
}

/// The posed joint positions (upper head, lower head, end head) on `coords`.
pub fn joints(skel: &Skeleton, coords: &[Vec3], pose: &Pose, chain: &[String; 3]) -> Option<[Vec3; 3]> {
    let mats = bone_matrices(skel, coords, pose);
    let mut out = [Vec3::ZERO; 3];
    for (o, name) in out.iter_mut().zip(chain) {
        let b = *skel.index.get(name)?;
        *o = mats[b].transform_point3(skel.head(b, coords));
    }
    Some(out)
}

/// Re-poses the limb so its end reaches `target` (clamped to the limb's
/// reach). Returns false when the bones are unknown or degenerate.
pub fn solve(pose: &mut Pose, coords: &[Vec3], limb: Limb, right: bool, target: Vec3) -> bool {
    let skel = skeleton();
    let chain = limb.chain(right);
    let Some([s, e, w]) = joints(skel, coords, pose, &chain) else { return false };
    let (a, b) = (s.distance(e), e.distance(w));
    if a < 1e-4 || b < 1e-4 {
        return false;
    }
    let d = s.distance(target).clamp((a - b).abs() + 1e-3, a + b - 1e-3);
    // 1. Bend the elbow / knee about the limb's plane normal until the end
    //    is `d` from the shoulder / hip (try both directions, keep the closer).
    let cross = (e - s).cross(w - e);
    // A straight limb has no plane: bend so the hand comes forward (elbow
    // back) or the foot goes back (knee forward).
    let pref = if limb == Limb::Arm { Vec3::Z } else { -Vec3::Z };
    let n = if cross.length() > 1e-3 * a * b { cross.normalize() } else { (e - s).cross(pref).normalize_or_zero() };
    let interior = (s - e).angle_between(w - e);
    let want = ((a * a + b * b - d * d) / (2.0 * a * b)).clamp(-1.0, 1.0).acos();
    let bend = [interior - want, want - interior]
        .into_iter()
        .map(|phi| Quat::from_axis_angle(n, phi))
        .min_by(|p, q| {
            let err = |r: &Quat| ((e + *r * (w - e)).distance(s) - d).abs();
            err(p).total_cmp(&err(q))
        })
        .unwrap_or(Quat::IDENTITY);
    let w1 = e + bend * (w - e);
    // 2. Swing the whole limb about the shoulder / hip onto the target.
    let swing = Quat::from_rotation_arc((w1 - s).normalize_or_zero(), (target - s).normalize_or_zero());
    // World deltas -> rest-frame local rotations (M = P * R about the head).
    let mats = bone_matrices(skel, coords, pose);
    let local = |name: &str, delta: Quat| -> Option<[f32; 3]> {
        let bi = *skel.index.get(name)?;
        let parent = skel.bones[bi].parent.map_or(Quat::IDENTITY, |p| rot(&mats[p]));
        let r = pose.rotation(name).unwrap_or(Quat::IDENTITY);
        Some(euler_deg((parent.inverse() * delta * parent * r).normalize()))
    };
    let (Some(upper), Some(lower)) = (local(&chain[0], swing), local(&chain[1], bend)) else { return false };
    pose.set(&chain[0], upper);
    pose.set(&chain[1], lower);
    true
}

/// Moves the hand / foot by `offset` (hm08 units: 1 = 10 cm) from where it is.
pub fn nudge(pose: &mut Pose, coords: &[Vec3], limb: Limb, right: bool, offset: Vec3) -> bool {
    let Some([_, _, end]) = joints(skeleton(), coords, pose, &limb.chain(right)) else { return false };
    solve(pose, coords, limb, right, end + offset)
}
