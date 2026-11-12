//! Easy Pose data: per-bone rotations in degrees (X = bend, Y = twist,
//! Z = side), measured in the rest-pose axes of the hm08 body (X towards
//! the character's left, Y up, Z forward) about the bone's head.

use bevy::math::{EulerRot, Quat};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pose {
    /// Bone name -> [bend, twist, side] degrees.
    pub joints: BTreeMap<String, [f32; 3]>,
}

/// The Easy Pose controls: (label, left / centre bone, right bone).
pub const CONTROLS: [(&str, &str, Option<&str>); 13] = [
    ("Whole Body", "root", None),
    ("Lower Spine", "spine04", None),
    ("Chest", "spine02", None),
    ("Neck", "neck01", None),
    ("Head", "head", None),
    ("Clavicle", "clavicle.L", Some("clavicle.R")),
    ("Upper Arm", "upperarm01.L", Some("upperarm01.R")),
    ("Elbow", "lowerarm01.L", Some("lowerarm01.R")),
    ("Wrist", "wrist.L", Some("wrist.R")),
    ("Thigh", "upperleg01.L", Some("upperleg01.R")),
    ("Knee", "lowerleg01.L", Some("lowerleg01.R")),
    ("Foot", "foot.L", Some("foot.R")),
    ("Jaw", "jaw", None),
];

/// Left <-> right twin of a bone name (`.L` / `.R`).
pub fn twin(bone: &str) -> Option<String> {
    if let Some(b) = bone.strip_suffix(".L") {
        Some(format!("{b}.R"))
    } else {
        bone.strip_suffix(".R").map(|b| format!("{b}.L"))
    }
}

/// Mirrors a rotation across the body's X = 0 plane (bend stays, twist
/// and side flip).
pub fn mirror([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, -y, -z]
}

impl Pose {
    pub fn is_rest(&self) -> bool {
        self.joints.values().all(|r| r.iter().all(|a| a.abs() < 1e-4))
    }

    pub fn get(&self, bone: &str) -> [f32; 3] {
        self.joints.get(bone).copied().unwrap_or_default()
    }

    pub fn rotation(&self, bone: &str) -> Option<Quat> {
        let r = self.joints.get(bone)?;
        if r.iter().all(|a| a.abs() < 1e-4) {
            return None;
        }
        Some(Quat::from_euler(EulerRot::ZYX, r[2].to_radians(), r[1].to_radians(), r[0].to_radians()))
    }

    /// Sets `bone`; a zero rotation removes the entry.
    pub fn set(&mut self, bone: &str, r: [f32; 3]) {
        if r.iter().all(|a| a.abs() < 1e-4) {
            self.joints.remove(bone);
        } else {
            self.joints.insert(bone.to_string(), r);
        }
    }

    /// Sets `bone` and the mirrored value on its twin.
    pub fn set_mirrored(&mut self, bone: &str, r: [f32; 3]) {
        self.set(bone, r);
        if let Some(t) = twin(bone) {
            self.set(&t, mirror(r));
        }
    }

    /// Copies one side onto the other (`left_to_right`, else right to left).
    pub fn copy_side(&mut self, left_to_right: bool) {
        let from = if left_to_right { ".L" } else { ".R" };
        let src: Vec<(String, [f32; 3])> = self.joints.iter().filter(|(k, _)| k.ends_with(from)).map(|(k, v)| (k.clone(), *v)).collect();
        for (k, v) in src {
            if let Some(t) = twin(&k) {
                self.set(&t, mirror(v));
            }
        }
    }

    /// Swaps the left and right sides (the mirror image of the pose).
    pub fn mirrored(&self) -> Pose {
        let mut out = Pose::default();
        for (k, v) in &self.joints {
            match twin(k) {
                Some(t) => out.set(&t, mirror(*v)),
                None => out.set(k, mirror(*v)),
            }
        }
        out
    }
}
