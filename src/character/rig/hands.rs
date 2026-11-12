//! One-click hand shapes for Easy Pose: each finger segment curls about
//! the axis across its own direction and the palm normal (measured on the
//! rest hm08 body), stored as ordinary per-bone Euler angles so mirroring,
//! side copy and `.ilexpose` files work unchanged.

use super::pose::{mirror, Pose};
use super::skeleton;
use crate::character::makehuman::obj::base;
use bevy::math::{EulerRot, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandShape {
    Open,
    Relaxed,
    Grip,
    Fist,
    Point,
}

impl HandShape {
    pub const ALL: [HandShape; 5] = [HandShape::Open, HandShape::Relaxed, HandShape::Grip, HandShape::Fist, HandShape::Point];

    pub fn label(self) -> &'static str {
        match self {
            HandShape::Open => "Open",
            HandShape::Relaxed => "Relaxed",
            HandShape::Grip => "Grip",
            HandShape::Fist => "Fist",
            HandShape::Point => "Point",
        }
    }

    /// Curl in degrees of segments 1..=3 for finger 1 (thumb) ..= 5 (little).
    fn curls(self, finger: usize) -> [f32; 3] {
        let thumb = finger == 1;
        match self {
            HandShape::Open => [0.0; 3],
            HandShape::Relaxed if thumb => [5.0, 10.0, 10.0],
            HandShape::Relaxed => [12.0 + 3.0 * finger as f32, 20.0, 15.0],
            HandShape::Grip if thumb => [20.0, 30.0, 25.0],
            HandShape::Grip => [55.0, 60.0, 40.0],
            HandShape::Fist | HandShape::Point if thumb => [30.0, 40.0, 35.0],
            HandShape::Point if finger == 2 => [0.0; 3],
            HandShape::Fist | HandShape::Point => [80.0, 95.0, 65.0],
        }
    }
}

/// The left palm normal on the rest body, pointing out of the palm.
pub(crate) fn palm_normal(c: &[Vec3]) -> Vec3 {
    let s = skeleton();
    let at = |b: &str| s.head(s.index[b], c);
    let wrist = at("wrist.L");
    let n = (at("finger2-1.L") - wrist).cross(at("finger5-1.L") - wrist).normalize_or_zero();
    // At rest the hands hang at the sides with the palms facing the thighs.
    if n.x > 0.0 { -n } else { n }
}

/// Euler [bend, twist, side] degrees (the `Pose` convention) of `q`.
fn euler_deg(q: Quat) -> [f32; 3] {
    let (z, y, x) = q.to_euler(EulerRot::ZYX);
    [x.to_degrees(), y.to_degrees(), z.to_degrees()]
}

/// The left-hand finger rotations for `shape` (bone name, Euler degrees).
pub fn left_hand(shape: HandShape) -> Vec<(String, [f32; 3])> {
    let c = &base().coords;
    let s = skeleton();
    let n = palm_normal(c);
    let mut out = Vec::new();
    for finger in 1..=5 {
        for (seg, deg) in shape.curls(finger).into_iter().enumerate() {
            let bone = format!("finger{finger}-{}.L", seg + 1);
            let Some(&b) = s.index.get(&bone) else { continue };
            let (head, dir) = (s.head(b, c), (s.tail(b, c) - s.head(b, c)).normalize_or_zero());
            // The thumb folds across the palm toward the little finger.
            let toward = if finger == 1 { (s.head(s.index["finger5-1.L"], c) - head).normalize_or_zero() } else { n };
            let axis = dir.cross(toward).normalize_or_zero();
            let r = if deg == 0.0 || axis == Vec3::ZERO { [0.0; 3] } else { euler_deg(Quat::from_axis_angle(axis, deg.to_radians())) };
            out.push((bone, r));
        }
    }
    out
}

/// Shapes the left hand, the right hand, or both (mirrored).
pub fn set_hand(pose: &mut Pose, shape: HandShape, left: bool, right: bool) {
    for (bone, r) in left_hand(shape) {
        if left {
            pose.set(&bone, r);
        }
        if right {
            pose.set(&bone.replace(".L", ".R"), mirror(r));
        }
    }
}
