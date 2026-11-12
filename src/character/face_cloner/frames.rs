//! Photos and exported video frames -> usable face shots: the face is
//! found, its head turn measured against MediaPipe's canonical face,
//! blurry and near-duplicate frames are dropped, and the sharpest few
//! frames of each view are kept.

use super::math::similarity;
use super::model::{self, canonical, Detected, MESH_POINTS};
use bevy::math::{EulerRot, Vec3};
use std::collections::BTreeMap;

/// Which way the head is turned (the subject's left / right).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum ViewBin {
    ProfileRight,
    ThreeQuarterRight,
    Front,
    ThreeQuarterLeft,
    ProfileLeft,
}

impl ViewBin {
    pub const ALL: [ViewBin; 5] = [Self::ProfileRight, Self::ThreeQuarterRight, Self::Front, Self::ThreeQuarterLeft, Self::ProfileLeft];

    /// From a head turn in degrees (+ = toward the subject's left).
    pub fn of(yaw: f32) -> Self {
        match yaw {
            y if y < -55.0 => Self::ProfileRight,
            y if y < -15.0 => Self::ThreeQuarterRight,
            y if y <= 15.0 => Self::Front,
            y if y <= 55.0 => Self::ThreeQuarterLeft,
            _ => Self::ProfileLeft,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ProfileRight => "Right Profile",
            Self::ThreeQuarterRight => "Right 3/4",
            Self::Front => "Front",
            Self::ThreeQuarterLeft => "Left 3/4",
            Self::ProfileLeft => "Left Profile",
        }
    }
}

/// One image with a face in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub name: String,
    pub size: (u32, u32),
    /// 478 points: image pixels (x right, y down) and depth.
    pub points: Vec<Vec3>,
    pub score: f32,
    /// Laplacian variance of the face, resampled to a fixed size.
    pub sharpness: f32,
    /// Head turn and tilt in degrees (+yaw = toward the subject's left).
    pub yaw: f32,
    pub pitch: f32,
    pub view: ViewBin,
    /// Mouth opening / mouth width (0 = closed).
    pub mouth_open: f32,
    /// 16x16 grey face thumbnail for duplicate tests.
    pub thumb: Vec<u8>,
    /// Manual pin corrections: point index -> image position.
    pub pins: BTreeMap<usize, [f32; 2]>,
    /// Picked for the fit.
    pub used: bool,
    /// Why it was not picked (empty when used).
    pub note: String,
}

impl Shot {
    /// Point `i` with any manual pin applied.
    pub fn point(&self, i: usize) -> Vec3 {
        let p = self.points[i];
        self.pins.get(&i).map_or(p, |q| Vec3::new(q[0], q[1], p.z))
    }

    /// Distance between the outer eye corners in pixels.
    pub fn eye_span(&self) -> f32 {
        self.points[33].truncate().distance(self.points[263].truncate())
    }
}

/// Image points as a right-handed y-up, z-toward-viewer cloud (the canonical frame).
pub fn to_canonical_frame(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.y, -p.z)
}

/// Head turn (yaw, pitch in degrees) of detected points, by fitting the canonical face.
pub fn head_angles(points: &[Vec3]) -> (f32, f32) {
    let can = &canonical()[..MESH_POINTS];
    let dst: Vec<Vec3> = points[..MESH_POINTS].iter().map(|&p| to_canonical_frame(p)).collect();
    let s = similarity(can, &dst, &vec![1.0; MESH_POINTS], None);
    let (yaw, pitch, _) = s.rot.to_euler(EulerRot::YXZ);
    // Turning toward its own left (+x) swings the nose toward +x: a +Y rotation.
    (yaw.to_degrees(), pitch.to_degrees())
}

fn grey(img: &image::RgbImage, x: f32, y: f32) -> f32 {
    let (xi, yi) = ((x as i64).clamp(0, img.width() as i64 - 1) as u32, (y as i64).clamp(0, img.height() as i64 - 1) as u32);
    let p = img.get_pixel(xi, yi);
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

/// The face box resampled to `n x n` grey values.
fn face_patch(img: &image::RgbImage, points: &[Vec3], n: usize) -> Vec<f32> {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &points[..MESH_POINTS] {
        (lo, hi) = (lo.min(*p), hi.max(*p));
    }
    let side = (hi.x - lo.x).max(hi.y - lo.y).max(1.0);
    let c = (lo + hi) * 0.5;
    (0..n * n).map(|i| grey(img, c.x + ((i % n) as f32 / n as f32 - 0.5) * side, c.y + ((i / n) as f32 / n as f32 - 0.5) * side)).collect()
}

fn laplacian_variance(g: &[f32], n: usize) -> f32 {
    let mut vals = Vec::with_capacity(n * n);
    for y in 1..n - 1 {
        for x in 1..n - 1 {
            let i = y * n + x;
            vals.push(g[i - 1] + g[i + 1] + g[i - n] + g[i + n] - 4.0 * g[i]);
        }
    }
    let mean = vals.iter().sum::<f32>() / vals.len() as f32;
    vals.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / vals.len() as f32
}

/// A shot from known points (no picture: sharpness 1, blank thumbnail).
pub fn from_points(name: &str, size: (u32, u32), points: Vec<Vec3>, score: f32) -> Shot {
    let (yaw, pitch) = head_angles(&points);
    let p = &points;
    let mouth_open = p[13].truncate().distance(p[14].truncate()) / p[61].truncate().distance(p[291].truncate()).max(1.0);
    Shot { name: name.to_string(), size, points, score, sharpness: 1.0, yaw, pitch, view: ViewBin::of(yaw), mouth_open, thumb: vec![0; 256], pins: BTreeMap::new(), used: true, note: String::new() }
}

/// Measures a detected face in its image.
pub fn shot(name: &str, img: &image::RgbImage, det: Detected) -> Shot {
    let sharpness = laplacian_variance(&face_patch(img, &det.points, 128), 128);
    let thumb = face_patch(img, &det.points, 16).iter().map(|&v| v as u8).collect();
    let mut s = from_points(name, img.dimensions(), det.points, det.score);
    (s.sharpness, s.thumb, s.used) = (sharpness, thumb, false);
    s
}

/// Finds and measures the face in one picture (downscaled to 1280 px for speed).
pub fn analyse(name: &str, img: &image::RgbImage) -> Option<Shot> {
    let long = img.width().max(img.height());
    let small;
    let img = if long > 1280 {
        let k = 1280.0 / long as f32;
        small = image::imageops::resize(img, (img.width() as f32 * k) as u32, (img.height() as f32 * k) as u32, image::imageops::FilterType::Triangle);
        &small
    } else {
        img
    };
    model::detect_image(img).map(|d| shot(name, img, d))
}

/// Picking rules for a batch of shots.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Picking {
    /// Frames kept per view.
    pub per_view: usize,
    /// Drop frames below this fraction of the sharpest frame's sharpness.
    pub min_sharpness: f32,
    /// Drop frames whose mouth is open wider than this (smiles, talking).
    pub max_mouth_open: f32,
}

impl Default for Picking {
    fn default() -> Self {
        Self { per_view: 3, min_sharpness: 0.35, max_mouth_open: 0.18 }
    }
}

fn thumb_diff(a: &[u8], b: &[u8]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (*x as f32 - *y as f32).abs()).sum::<f32>() / a.len().max(1) as f32
}

/// Marks the shots to fit (`used`) and notes why the rest were dropped.
pub fn pick(shots: &mut [Shot], rules: &Picking) {
    let best = shots.iter().map(|s| s.sharpness).fold(0.0, f32::max);
    let mut order: Vec<usize> = (0..shots.len()).collect();
    order.sort_by(|&a, &b| shots[b].sharpness.total_cmp(&shots[a].sharpness));
    let mut kept: Vec<usize> = Vec::new();
    for i in order {
        let s = &shots[i];
        let note = if s.sharpness < best * rules.min_sharpness {
            "blurry".to_string()
        } else if s.mouth_open > rules.max_mouth_open {
            "mouth open (not a neutral face)".to_string()
        } else if let Some(&k) = kept.iter().find(|&&k| (shots[k].yaw - s.yaw).abs() < 4.0 && thumb_diff(&shots[k].thumb, &s.thumb) < 6.0) {
            format!("near-duplicate of {}", shots[k].name)
        } else if kept.iter().filter(|&&k| shots[k].view == s.view).count() >= rules.per_view {
            format!("{} already has {} sharper frames", s.view.label(), rules.per_view)
        } else {
            kept.push(i);
            String::new()
        };
        shots[i].used = note.is_empty();
        shots[i].note = note;
    }
}
