//! The detector's own habits: MediaPipe's points on an unchanged hm08
//! head do not sit exactly on the corresponding vertices (the canonical
//! face and MakeHuman's differ a little in where they put an eyelid or a
//! jaw line). Rendering the base head in each view and measuring that
//! offset lets the fit subtract it, so the default head clones to itself.
//! The cloner then repeats this on its own result (render the fitted head
//! at each photo's angle, detect, refit): the detector's habits cancel on
//! both sides, which also undoes its pull toward an average face.

use super::correspond;
use super::fit::{camera, unproject, Obs};
use super::frames::to_canonical_frame;
use super::model::{self, MESH_POINTS};
use super::render::head_view;
use crate::character::body::morph;
use crate::character::params::CharacterParams;
use bevy::math::Vec3;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// The detector's bias on the head `coords` (hm08 order) rendered at
/// `yaw` degrees: detected point minus true point, head space.
pub fn measure_on(coords: &[Vec3], yaw: f32) -> Option<Vec<Vec3>> {
    let cor = correspond::get()?;
    let (_, img) = head_view(coords, yaw.to_radians());
    let det = model::detect_image(&img)?;
    let pts: Vec<Vec3> = det.points[..MESH_POINTS].iter().map(|&p| to_canonical_frame(p)).collect();
    let verts: Vec<Vec3> = cor.verts.iter().map(|&v| coords[v as usize]).collect();
    let o = Obs { points: pts, weight: 1.0, bias: None };
    let cam = camera(&verts, &o, &vec![1.0; MESH_POINTS]);
    Some((0..MESH_POINTS).map(|i| unproject(&cam, o.points[i]) - verts[i]).collect())
}

/// The bias on the default character at `yaw` degrees.
pub fn measure(yaw: f32) -> Option<Vec<Vec3>> {
    measure_on(&morph(&CharacterParams::default()).0, yaw)
}

/// The cached bias at head turn `yaw` degrees (5 degree steps; None if
/// the detector misses that render).
pub fn bias(yaw: f32) -> Option<Vec<Vec3>> {
    static CELL: OnceLock<Mutex<HashMap<i32, Option<Vec<Vec3>>>>> = OnceLock::new();
    let step = (yaw.clamp(-85.0, 85.0) / 5.0).round() as i32;
    let cell = CELL.get_or_init(Default::default);
    if let Some(b) = cell.lock().ok()?.get(&step) {
        return b.clone();
    }
    let b = measure(step as f32 * 5.0);
    cell.lock().ok()?.insert(step, b.clone());
    b
}
