//! Face Cloner, start to finish: the picked shots -> MakeHuman head
//! sliders + a fine warp, written onto a copy of the character.

use super::fit::{self, fit, Obs, GROUPS};
use super::frames::{to_canonical_frame, Shot};
use super::model::MESH_POINTS;
use super::warp;
use super::{calibrate, correspond, FaceClone};
use crate::character::anime::head_sphere;
use crate::character::body::{body_mask, morph};
use crate::character::makehuman::modifiers::sliders;
use crate::character::params::CharacterParams;
use bevy::math::Vec3;

/// Fit settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// Subtract the detector's measured habits on the base head.
    pub calibrate: bool,
    /// Add the fine RBF warp on top of the sliders.
    pub fine_warp: bool,
    /// Pull toward the unchanged head (bigger = more cautious).
    pub ridge: f64,
    pub rounds: usize,
    /// Render-detect-refit passes after the first fit (0 = none).
    pub refine: usize,
    /// Bake the photos into the skin texture.
    pub texture: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { calibrate: true, fine_warp: true, ridge: 0.002, rounds: 5, refine: 2, texture: true }
    }
}

/// What the fit did, for the UI.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub views: usize,
    /// RMS image-plane error, millimetres on the model head: before, after the sliders, after the warp.
    pub before_mm: f32,
    pub sliders_mm: f32,
    pub warp_mm: f32,
    /// Sliders the fit moved (key, value).
    pub moved: Vec<(String, f32)>,
    /// The skin bake, when it ran.
    pub skin: Option<super::skin::SkinReport>,
}

/// Every micro slider key the cloner owns (both mirror halves).
pub fn owned_keys() -> Vec<String> {
    sliders().iter().filter(|s| GROUPS.contains(&s.group.as_str())).map(|s| s.key.clone()).collect()
}

/// `p` with its cloned face removed (face sliders zeroed, warp cleared).
pub fn reset(p: &CharacterParams) -> CharacterParams {
    let mut q = p.clone();
    for k in owned_keys() {
        q.micro.remove(&k);
    }
    q.face = FaceClone::default();
    q
}

/// How much a view counts: steep views (past 35 degrees) are less
/// reliable (MediaPipe guesses the hidden half of the face).
pub fn view_weight(yaw: f32) -> f32 {
    1.0 - 0.75 * ((yaw.abs() - 35.0) / 35.0).clamp(0.0, 1.0)
}

/// The observation of one shot (pins applied).
pub fn observe(s: &Shot, calibrate: bool) -> Obs {
    let points = (0..MESH_POINTS).map(|i| to_canonical_frame(s.point(i))).collect();
    let bias = if calibrate { calibrate::bias(s.yaw) } else { None };
    Obs { points, weight: view_weight(s.yaw), bias }
}

/// Fits `p`'s head to the `used` shots; returns the new character and a report.
pub fn clone_face(p: &CharacterParams, shots: &[Shot], opt: &Options) -> Result<(CharacterParams, Report), String> {
    let used: Vec<&Shot> = shots.iter().filter(|s| s.used).collect();
    if used.is_empty() {
        return Err("No usable face shots: load photos or frames with a clear face first.".into());
    }
    let cor = correspond::get().ok_or("The bundled face model could not be read.")?;
    let mut q = reset(p);
    let (pose, anime) = (std::mem::take(&mut q.pose), q.anime);
    q.anime = 0.0;
    let (coords, _) = morph(&q);
    let head = head_sphere(&coords, body_mask());
    let base: Vec<Vec3> = cor.verts.iter().map(|&v| coords[v as usize]).collect();
    // Points far from their vertex (lip line, inner eye) count less.
    let w: Vec<f32> = cor.gap.iter().map(|g| 1.0 / (1.0 + (g / 0.03).powi(2))).collect();
    let mut obs: Vec<Obs> = used.iter().map(|s| observe(s, opt.calibrate)).collect();
    let cols = fit::columns(&cor.verts);
    let mut f = fit(&base, &cols, &obs, &w, head.0, opt.rounds, opt.ridge);
    let before = f.before;
    let unfitted = q.clone();
    let set = |f: &fit::Fit| {
        let mut q = unfitted.clone();
        for (k, v) in f.values.iter().filter(|e| e.1.abs() > 0.01) {
            q.set_micro(k, *v);
        }
        q
    };
    q = set(&f);
    for _ in 0..if opt.calibrate { opt.refine } else { 0 } {
        let (now, _) = morph(&q);
        for (o, s) in obs.iter_mut().zip(&used) {
            if let Some(b) = calibrate::measure_on(&now, s.yaw) {
                o.bias = Some(b);
            }
        }
        f = fit(&base, &cols, &obs, &w, head.0, opt.rounds, opt.ridge);
        q = set(&f);
    }
    let mm = |dm: f32| dm * 100.0;
    let moved = f.values.iter().filter(|e| e.1.abs() > 0.01).cloned().collect();
    let mut report = Report { views: obs.len(), before_mm: mm(before), sliders_mm: mm(f.after), warp_mm: mm(f.after), moved, skin: None };
    if opt.fine_warp {
        let views: Vec<_> = f.cams.iter().copied().zip(&obs).collect();
        let offsets = warp::point_offsets(&f.model, &views, &w, head.0);
        let (fitted, _) = morph(&q);
        q.face = FaceClone { deltas: warp::spread(&fitted, &cor.verts, &offsets, head), views: obs.len(), ..FaceClone::default() };
    } else {
        q.face.views = obs.len();
    }
    // The cameras of the final head (also what the skin is projected with).
    let (fin, _) = morph(&q);
    let model: Vec<Vec3> = cor.verts.iter().map(|&v| fin[v as usize]).collect();
    let cams: Vec<_> = obs.iter().map(|o| fit::camera(&model, o, &w)).collect();
    if opt.fine_warp {
        report.warp_mm = mm(fit::rms(&model, &cams.iter().copied().zip(&obs).collect::<Vec<_>>(), &w, head.0));
    }
    if opt.texture
        && let Some((tex, rep)) = super::skin::bake_shots(&q, &fin, head, &used, &cams)
    {
        q.face.texture = Some(tex);
        report.skin = Some(rep);
    }
    (q.pose, q.anime) = (pose, anime);
    Ok((q, report))
}
