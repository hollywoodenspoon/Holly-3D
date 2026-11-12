//! Which hm08 vertex each MediaPipe face-mesh point is. MediaPipe's
//! canonical face is placed on the base head (eye centres and nose tip
//! give a first similarity, then a few rounds of closest-point
//! refinement), and every canonical point takes its nearest head vertex.
//! Computed once from the bundled data, so no table needs maintaining.

use super::math::{similarity, Similarity};
use super::model::{canonical, MESH_POINTS};
use crate::character::anime::head_sphere;
use crate::character::body::body_mask;
use crate::character::makehuman::obj::{base, BaseMesh};
use bevy::math::Vec3;
use std::sync::OnceLock;

/// Canonical eye rings (subject's right, subject's left) and nose tip.
const EYE_R: [usize; 4] = [33, 133, 159, 145];
const EYE_L: [usize; 4] = [263, 362, 386, 374];
pub const NOSE_TIP: usize = 4;

/// The vertices used by the faces of group `name`.
pub fn group_verts(b: &BaseMesh, name: &str) -> Vec<u32> {
    let mut v: Vec<u32> = b.faces.iter().filter(|f| b.groups[f.group as usize] == name).flat_map(|f| f.verts().to_vec()).collect();
    v.sort_unstable();
    v.dedup();
    v
}

fn centroid(coords: &[Vec3], idx: &[u32]) -> Vec3 {
    idx.iter().map(|&i| coords[i as usize]).sum::<Vec3>() / idx.len().max(1) as f32
}

/// Body vertices of the front of the head (the candidates for face points).
pub fn face_candidates(coords: &[Vec3]) -> Vec<u32> {
    let (c, r) = head_sphere(coords, body_mask());
    (0..coords.len() as u32)
        .filter(|&i| {
            let p = coords[i as usize];
            body_mask()[i as usize] && p.distance(c) < r * 1.35 && p.z > c.z - r * 0.15 && p.y > c.y - r * 1.25
        })
        .collect()
}

fn nearest(coords: &[Vec3], cand: &[u32], p: Vec3) -> u32 {
    cand.iter().copied().min_by(|&a, &b| coords[a as usize].distance_squared(p).total_cmp(&coords[b as usize].distance_squared(p))).unwrap_or(0)
}

/// The canonical face placed on the hm08 head, and the 468 nearest vertices.
pub struct Correspondence {
    pub place: Similarity,
    pub verts: Vec<u32>,
    /// Distance (decimetres) from each placed canonical point to its vertex.
    pub gap: Vec<f32>,
}

pub fn compute(b: &BaseMesh) -> Option<Correspondence> {
    let can = canonical();
    if can.len() < MESH_POINTS {
        return None;
    }
    let coords = &b.coords;
    let cand = face_candidates(coords);
    let (eye_l, eye_r) = (centroid(coords, &group_verts(b, "helper-l-eye")), centroid(coords, &group_verts(b, "helper-r-eye")));
    let mid = (eye_l + eye_r) * 0.5;
    // Nose tip: the most forward head vertex near the midline below the eyes.
    let nose = cand.iter().map(|&i| coords[i as usize]).filter(|p| p.x.abs() < 0.05 && p.y < mid.y && p.y > mid.y - 0.8).max_by(|a, b| a.z.total_cmp(&b.z))?;
    let ring = |r: &[usize]| r.iter().map(|&i| can[i]).sum::<Vec3>() / r.len() as f32;
    // MakeHuman's eyeballs sit behind the lid line; pull the target onto it.
    let (src, dst) = ([ring(&EYE_L), ring(&EYE_R), can[NOSE_TIP]], [eye_l + Vec3::Z * 0.08, eye_r + Vec3::Z * 0.08, nose]);
    let mut place = similarity(&src, &dst, &[1.0, 1.0, 2.0], None);
    let scale = place.scale;
    let mut verts = vec![0; MESH_POINTS];
    for round in 0..6 {
        let placed: Vec<Vec3> = can[..MESH_POINTS].iter().map(|&p| place.apply(p)).collect();
        verts = placed.iter().map(|&p| nearest(coords, &cand, p)).collect();
        if round == 5 {
            break;
        }
        let targets: Vec<Vec3> = verts.iter().map(|&v| coords[v as usize]).collect();
        // Keep the anchors in the refinement so the fit cannot slide off the face.
        let (mut s, mut d, mut w) = (placed_src(can), targets, vec![1.0; MESH_POINTS]);
        s.extend(src);
        d.extend(dst);
        w.extend([40.0, 40.0, 80.0]);
        place = similarity(&s, &d, &w, Some(scale));
    }
    let gap = can[..MESH_POINTS].iter().zip(&verts).map(|(&p, &v)| place.apply(p).distance(coords[v as usize])).collect();
    Some(Correspondence { place, verts, gap })
}

fn placed_src(can: &[Vec3]) -> Vec<Vec3> {
    can[..MESH_POINTS].to_vec()
}

/// The correspondence for the bundled base mesh (cached).
pub fn get() -> Option<&'static Correspondence> {
    static CELL: OnceLock<Option<Correspondence>> = OnceLock::new();
    CELL.get_or_init(|| compute(base())).as_ref()
}
