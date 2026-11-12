//! Skeleton landmarks of a morphed body: the centroid of every hm08
//! `joint-*` helper cube (they are morphed with the body, which is how
//! MakeHuman places its rig). Used to fit hair, outfits and weapons, and
//! by Phase 10B to place the bones.

use super::makehuman::obj::base;
use bevy::math::Vec3;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Vertex indices of each joint group (cached).
fn groups() -> &'static HashMap<String, Vec<u32>> {
    static CELL: OnceLock<HashMap<String, Vec<u32>>> = OnceLock::new();
    CELL.get_or_init(|| {
        let b = base();
        let mut out: HashMap<String, Vec<u32>> = HashMap::new();
        for f in &b.faces {
            let g = &b.groups[f.group as usize];
            if let Some(name) = g.strip_prefix("joint-") {
                let e = out.entry(name.to_string()).or_default();
                e.extend_from_slice(f.verts());
            }
        }
        for v in out.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        out
    })
}

/// Joint centre in the coordinates of `coords` (hm08 space).
pub fn joint(coords: &[Vec3], name: &str) -> Option<Vec3> {
    let idx = groups().get(name)?;
    Some(idx.iter().map(|&i| coords[i as usize]).sum::<Vec3>() / idx.len().max(1) as f32)
}

pub fn joint_names() -> Vec<&'static str> {
    let mut v: Vec<&str> = groups().keys().map(String::as_str).collect();
    v.sort_unstable();
    v
}

/// The landmarks the Character Studio fits pieces to (scene space).
#[derive(Clone, Copy, Debug, Default)]
pub struct Landmarks {
    pub head: Vec3,
    pub head_r: f32,
    pub neck: Vec3,
    pub chest: Vec3,
    pub pelvis: Vec3,
    pub shoulder: [Vec3; 2],
    pub elbow: [Vec3; 2],
    pub hand: [Vec3; 2],
    pub knee: [Vec3; 2],
    pub ankle: [Vec3; 2],
    pub height: f32,
}

impl Landmarks {
    /// `to_scene` maps hm08 coordinates to the final scene placement.
    pub fn of(coords: &[Vec3], head: (Vec3, f32), height: f32, to_scene: impl Fn(Vec3) -> Vec3) -> Self {
        let j = |n: &str| to_scene(joint(coords, n).unwrap_or(Vec3::ZERO));
        let lr = |n: &str| [j(&format!("l-{n}")), j(&format!("r-{n}"))];
        Self {
            head: to_scene(head.0),
            head_r: head.1 * 0.1,
            neck: j("neck"),
            chest: j("spine-2"),
            pelvis: j("pelvis"),
            shoulder: lr("shoulder"),
            elbow: lr("elbow"),
            hand: lr("hand"),
            knee: lr("knee"),
            ankle: lr("ankle"),
            height,
        }
    }
}
