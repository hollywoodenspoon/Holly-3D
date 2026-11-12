//! Phase 10B: the MakeHuman default skeleton (163 bones) and its skin
//! weights, bound to the morphed hm08 body. Bone heads are the centroids
//! of the skeleton's joint vertex lists on the *morphed* mesh, so every
//! body shape gets its own fitted rig.
//!
//! Posing is forward kinematics with linear blend skinning: each bone
//! rotates about its rest head, its children inherit the motion, and every
//! vertex is moved by the weighted blend of its bones' transforms.

pub mod pose;
pub mod pose_file;
pub mod presets;

#[cfg(test)]
mod tests;

use super::makehuman::assets;
use bevy::math::{Mat4, Vec3};
use std::collections::HashMap;
use std::sync::OnceLock;

pub use pose::Pose;

pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    /// Vertex indices whose centroid is the head (the pivot).
    pub head: Vec<u32>,
    pub tail: Vec<u32>,
}

/// (name, parent, head joint, tail joint) as read from the file.
type RawBone = (String, Option<String>, Vec<u32>, Vec<u32>);

/// Bones in parent-before-child order.
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub index: HashMap<String, usize>,
}

impl Skeleton {
    pub fn parse(json: &str) -> Self {
        let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
        let joints = &v["joints"];
        let ids = |name: &str| -> Vec<u32> {
            joints[name].as_array().into_iter().flatten().filter_map(|x| x.as_u64()).map(|x| x as u32).collect()
        };
        let raw: Vec<RawBone> = v["bones"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(n, b)| {
                let parent = b["parent"].as_str().map(str::to_string);
                (n.clone(), parent, ids(b["head"].as_str().unwrap_or("")), ids(b["tail"].as_str().unwrap_or("")))
            })
            .collect();
        // Topological order: repeatedly take bones whose parent is placed.
        let mut bones: Vec<Bone> = Vec::with_capacity(raw.len());
        let mut index = HashMap::new();
        let mut left = raw;
        while !left.is_empty() {
            let before = left.len();
            let mut rest = Vec::new();
            for (name, parent, head, tail) in left {
                let p = match &parent {
                    None => Some(None),
                    Some(p) => index.get(p).map(|&i| Some(i)),
                };
                match p {
                    Some(parent) => {
                        index.insert(name.clone(), bones.len());
                        bones.push(Bone { name, parent, head, tail });
                    }
                    None => rest.push((name, parent, head, tail)),
                }
            }
            left = rest;
            if left.len() == before {
                break; // orphaned bones (none in the bundled rig)
            }
        }
        Self { bones, index }
    }

    pub fn head(&self, b: usize, coords: &[Vec3]) -> Vec3 {
        centroid(&self.bones[b].head, coords)
    }

    pub fn tail(&self, b: usize, coords: &[Vec3]) -> Vec3 {
        centroid(&self.bones[b].tail, coords)
    }
}

fn centroid(ids: &[u32], coords: &[Vec3]) -> Vec3 {
    let pts: Vec<Vec3> = ids.iter().filter_map(|&i| coords.get(i as usize).copied()).collect();
    pts.iter().copied().sum::<Vec3>() / pts.len().max(1) as f32
}

pub fn skeleton() -> &'static Skeleton {
    static CELL: OnceLock<Skeleton> = OnceLock::new();
    CELL.get_or_init(|| Skeleton::parse(assets::text(assets::skeleton_json())))
}

/// Per-vertex (bone, weight) lists, normalised to sum to 1.
pub fn parse_weights(json: &str, skel: &Skeleton, n_verts: usize) -> Vec<Vec<(u16, f32)>> {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let mut out = vec![Vec::new(); n_verts];
    for (bone, list) in v["weights"].as_object().into_iter().flatten() {
        let Some(&b) = skel.index.get(bone) else { continue };
        for pair in list.as_array().into_iter().flatten() {
            let (Some(i), Some(w)) = (pair[0].as_u64(), pair[1].as_f64()) else { continue };
            if let Some(slot) = out.get_mut(i as usize) {
                slot.push((b as u16, w as f32));
            }
        }
    }
    for slot in &mut out {
        let s: f32 = slot.iter().map(|x| x.1).sum();
        if s > 0.0 {
            slot.iter_mut().for_each(|x| x.1 /= s);
        }
    }
    out
}

pub fn weights() -> &'static [Vec<(u16, f32)>] {
    static CELL: OnceLock<Vec<Vec<(u16, f32)>>> = OnceLock::new();
    CELL.get_or_init(|| parse_weights(assets::text(assets::weights_json()), skeleton(), super::makehuman::obj::base().coords.len()))
}

/// World skinning matrix of every bone for `pose` on the rest `coords`.
pub fn bone_matrices(skel: &Skeleton, coords: &[Vec3], pose: &Pose) -> Vec<Mat4> {
    let mut m = vec![Mat4::IDENTITY; skel.bones.len()];
    for (i, b) in skel.bones.iter().enumerate() {
        let parent = b.parent.map_or(Mat4::IDENTITY, |p| m[p]);
        m[i] = match pose.rotation(&b.name) {
            Some(r) => {
                let h = skel.head(i, coords);
                parent * Mat4::from_translation(h) * Mat4::from_quat(r) * Mat4::from_translation(-h)
            }
            None => parent,
        };
    }
    m
}

/// The skinning matrix of one bone (identity if it is unknown).
pub fn bone_matrix(coords: &[Vec3], pose: &Pose, bone: &str) -> Mat4 {
    let skel = skeleton();
    skel.index.get(bone).map_or(Mat4::IDENTITY, |&b| bone_matrices(skel, coords, pose)[b])
}

/// Poses the hm08 coordinates in place (no-op for the rest pose).
pub fn apply(pose: &Pose, coords: &mut [Vec3]) {
    if pose.is_rest() {
        return;
    }
    let skel = skeleton();
    let mats = bone_matrices(skel, coords, pose);
    for (c, w) in coords.iter_mut().zip(weights()) {
        if w.is_empty() {
            continue;
        }
        let p = *c;
        *c = w.iter().map(|&(b, wt)| mats[b as usize].transform_point3(p) * wt).sum();
    }
}
