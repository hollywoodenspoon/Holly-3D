//! The multi-view fit of MakeHuman's own head sliders. Each view gets a
//! similarity camera (Horn) from the current head to its detected points;
//! the slider weights then come from one clamped ridge least-squares
//! solve over every view, and the two steps alternate. Residuals are
//! measured in head space along each camera's image axes (depth counts
//! less: MediaPipe's z is a guess), weighted by how squarely the point
//! faces that camera.

use super::math::{similarity, solve_ridge, Similarity};
use super::model::MESH_POINTS;
use crate::character::makehuman::modifiers::{sliders, Slider};
use crate::character::makehuman::target::pack;
use bevy::math::Vec3;
use std::collections::HashMap;

/// Slider groups that shape the face (ears and neck are not seen well enough).
pub const GROUPS: [&str; 8] = ["head", "forehead", "eyebrows", "eyes", "nose", "mouth", "chin", "cheek"];
/// Weight of the depth axis against the two image axes.
const DEPTH_WEIGHT: f64 = 0.3;

/// The fitted sliders: face groups minus pure placement, one per mirrored pair.
pub fn fit_sliders() -> Vec<&'static Slider> {
    let all = sliders();
    all.iter()
        .filter(|s| GROUPS.contains(&s.group.as_str()))
        .filter(|s| !s.key.contains("head-trans") && !s.key.contains("head-angle"))
        .filter(|s| !(s.key.contains("/l-") && all.iter().any(|o| o.key == s.key.replacen("/l-", "/r-", 1))))
        .collect()
}

/// One unknown: a slider's positive or negative target (mirror twin
/// included), as its offset at each of the 468 face points.
pub struct Column {
    pub key: String,
    pub neg: bool,
    pub delta: Vec<Vec3>,
}

pub fn columns(verts: &[u32]) -> Vec<Column> {
    let mut at: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, &v) in verts.iter().enumerate() {
        at.entry(v).or_default().push(i);
    }
    let mut out = Vec::new();
    for s in fit_sliders() {
        for (name, neg) in [(Some(&s.pos), false), (s.neg.as_ref(), true)] {
            let Some(name) = name else { continue };
            let mut delta = vec![Vec3::ZERO; verts.len()];
            let twin = name.contains("/r-").then(|| name.replacen("/r-", "/l-", 1));
            for t in std::iter::once(name.clone()).chain(twin).filter_map(|n| pack().get(&n)) {
                for (i, d) in t.index.iter().zip(&t.delta) {
                    for &p in at.get(i).into_iter().flatten() {
                        delta[p] += *d;
                    }
                }
            }
            if delta.iter().any(|d| d.length_squared() > 1e-10) {
                out.push(Column { key: s.key.clone(), neg, delta });
            }
        }
    }
    out
}

/// One view: its 468 points in the canonical frame (x right, y up, z
/// toward the camera; any units), how much it counts, and the detector's
/// known bias on an unchanged head in this view (head space).
pub struct Obs {
    pub points: Vec<Vec3>,
    pub weight: f32,
    pub bias: Option<Vec<Vec3>>,
}

impl Obs {
    fn bias(&self, i: usize) -> Vec3 {
        self.bias.as_ref().map_or(Vec3::ZERO, |b| b[i])
    }
}

/// Inverse camera: canonical-frame point -> head space.
pub fn unproject(s: &Similarity, p: Vec3) -> Vec3 {
    s.rot.inverse() * (p - s.offset) / s.scale
}

/// The camera that best takes `model` (+ bias) onto the view's points.
pub fn camera(model: &[Vec3], o: &Obs, w: &[f32]) -> Similarity {
    let src: Vec<Vec3> = (0..MESH_POINTS).map(|i| model[i] + o.bias(i)).collect();
    similarity(&src, &o.points, w, None)
}

/// Image x, image y and depth axes in head space, with their weights.
pub fn axes(s: &Similarity) -> [(Vec3, f64); 3] {
    let inv = s.rot.inverse();
    [(inv * Vec3::X, 1.0), (inv * Vec3::Y, 1.0), (inv * Vec3::Z, DEPTH_WEIGHT)]
}

/// How squarely point `p` faces the camera (0 = edge-on or behind).
pub fn facing(p: Vec3, head: Vec3, s: &Similarity) -> f32 {
    let toward = s.rot.inverse() * Vec3::Z;
    ((p - head).normalize_or_zero().dot(toward) + 0.2).clamp(0.0, 1.0)
}

/// Each view's residual (head space) and weight at every point.
pub fn residuals(model: &[Vec3], views: &[(Similarity, &Obs)], w: &[f32], head: Vec3) -> Vec<Vec<(Vec3, f32)>> {
    views
        .iter()
        .map(|(s, o)| (0..MESH_POINTS).map(|i| (unproject(s, o.points[i]) - o.bias(i) - model[i], w[i] * o.weight * facing(model[i], head, s))).collect())
        .collect()
}

/// Weighted RMS image-plane error (head units) of the model against the views.
pub fn rms(model: &[Vec3], views: &[(Similarity, &Obs)], w: &[f32], head: Vec3) -> f32 {
    let (mut sum, mut tw) = (0.0f64, 0.0f64);
    for ((s, _), res) in views.iter().zip(residuals(model, views, w, head)) {
        let ax = axes(s);
        for (e, k) in res {
            sum += k as f64 * (ax[0].0.dot(e).powi(2) + ax[1].0.dot(e).powi(2)) as f64;
            tw += k as f64;
        }
    }
    (sum / tw.max(1e-12)).sqrt() as f32
}

/// The weights (each 0..1) of `cols` best explaining the residuals.
fn solve(cols: &[Column], model: &[Vec3], base: &[Vec3], views: &[(Similarity, &Obs)], w: &[f32], head: Vec3, ridge: f64) -> Vec<f64> {
    let n = cols.len();
    let nz: Vec<Vec<usize>> = (0..MESH_POINTS).map(|i| (0..n).filter(|&j| cols[j].delta[i].length_squared() > 1e-12).collect()).collect();
    let (mut ata, mut atb) = (vec![0.0; n * n], vec![0.0; n]);
    let res = residuals(model, views, w, head);
    for ((s, _), res) in views.iter().zip(res) {
        for (a, k) in axes(s) {
            for (i, &(e, wk)) in res.iter().enumerate() {
                if wk <= 0.0 {
                    continue;
                }
                // Residual against the base head (the solve is for absolute weights).
                let y = a.dot(e + model[i] - base[i]) as f64;
                let row: Vec<(usize, f64)> = nz[i].iter().map(|&j| (j, a.dot(cols[j].delta[i]) as f64)).collect();
                let wk = wk as f64 * k;
                for &(j, rj) in &row {
                    atb[j] += wk * rj * y;
                    for &(l, rl) in &row {
                        ata[j * n + l] += wk * rj * rl;
                    }
                }
            }
        }
    }
    // Active set: clamp any weight leaving 0..1 and solve the rest again.
    let mut fixed: Vec<Option<f64>> = vec![None; n];
    let mut x = vec![0.0; n];
    for _ in 0..12 {
        let free: Vec<usize> = (0..n).filter(|&j| fixed[j].is_none()).collect();
        let m = free.len();
        let mut a = vec![0.0; m * m];
        let mut b = vec![0.0; m];
        for (r, &j) in free.iter().enumerate() {
            b[r] = atb[j] - (0..n).filter_map(|l| fixed[l].map(|v| ata[j * n + l] * v)).sum::<f64>();
            for (c, &l) in free.iter().enumerate() {
                a[r * m + c] = ata[j * n + l];
            }
        }
        let Some(sol) = solve_ridge(a, b, m, ridge) else { break };
        let mut changed = false;
        for (r, &j) in free.iter().enumerate() {
            let v = sol[r];
            x[j] = v;
            if !(0.0..=1.0).contains(&v) {
                fixed[j] = Some(v.clamp(0.0, 1.0));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    (0..n).map(|j| fixed[j].unwrap_or(x[j]).clamp(0.0, 1.0)).collect()
}

/// The 468 face points with column weights `x` applied.
pub fn shaped(base: &[Vec3], cols: &[Column], x: &[f64]) -> Vec<Vec3> {
    (0..MESH_POINTS).map(|i| base[i] + cols.iter().zip(x).map(|(c, &v)| c.delta[i] * v as f32).sum::<Vec3>()).collect()
}

/// The fit's answer.
pub struct Fit {
    /// Slider key -> value (two-sided -1..1, one-sided 0..1).
    pub values: Vec<(String, f32)>,
    pub cams: Vec<Similarity>,
    /// The fitted 468 points (head space).
    pub model: Vec<Vec3>,
    /// RMS image-plane error before and after, head units.
    pub before: f32,
    pub after: f32,
}

/// Alternates camera and slider solves for `rounds` rounds.
pub fn fit(base: &[Vec3], cols: &[Column], obs: &[Obs], w: &[f32], head: Vec3, rounds: usize, ridge: f64) -> Fit {
    let cams_for = |model: &[Vec3]| -> Vec<Similarity> { obs.iter().map(|o| camera(model, o, w)).collect() };
    let pair = |cams: &[Similarity]| -> Vec<(Similarity, &Obs)> { cams.iter().copied().zip(obs).collect() };
    let mut cams = cams_for(base);
    let before = rms(base, &pair(&cams), w, head);
    let (mut model, mut x) = (base.to_vec(), vec![0.0; cols.len()]);
    for _ in 0..rounds {
        x = solve(cols, &model, base, &pair(&cams), w, head, ridge * obs.len() as f64);
        model = shaped(base, cols, &x);
        cams = cams_for(&model);
    }
    let after = rms(&model, &pair(&cams), w, head);
    let mut values: Vec<(String, f32)> = Vec::new();
    for (c, &v) in cols.iter().zip(&x) {
        let v = if c.neg { -v } else { v } as f32;
        match values.iter_mut().find(|(k, _)| *k == c.key) {
            Some(e) => e.1 += v,
            None => values.push((c.key.clone(), v)),
        }
    }
    for e in &mut values {
        e.1 = e.1.clamp(-1.0, 1.0);
    }
    Fit { values, cams, model, before, after }
}
