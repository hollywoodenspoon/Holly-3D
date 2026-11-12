//! The fine pass after the slider fit: what the sliders could not reach
//! is measured at each face point (one 3D offset agreeing with every
//! view), then spread smoothly over the head with a Gaussian RBF that
//! dies out away from the face and fades to nothing down the neck.

use super::fit::{axes, residuals, Obs};
use super::math::{solve_ridge, Similarity};
use super::model::MESH_POINTS;
use bevy::math::{Mat3, Vec3};
use std::collections::BTreeMap;

/// RBF radius (decimetres): a little under an eye opening.
pub const SIGMA: f32 = 0.22;
/// Largest offset trusted at one point (decimetres).
pub const MAX_OFFSET: f32 = 0.15;
/// Damping of the per-point solve, relative to the total view weight.
pub const DAMPING: f32 = 0.5;

/// Per point: the offset (head space) that best explains every view.
pub fn point_offsets(model: &[Vec3], views: &[(Similarity, &Obs)], w: &[f32], head: Vec3) -> Vec<Option<Vec3>> {
    let res = residuals(model, views, w, head);
    (0..MESH_POINTS)
        .map(|i| {
            let (mut m, mut b, mut tw) = (Mat3::ZERO, Vec3::ZERO, 0.0);
            for ((s, _), r) in views.iter().zip(&res) {
                let (e, k) = r[i];
                for (a, ak) in axes(s) {
                    let k = k * ak as f32;
                    m += Mat3::from_cols(a * a.x, a * a.y, a * a.z) * k;
                    b += a * a.dot(e) * k;
                    tw += k;
                }
            }
            if tw < 0.3 {
                return None;
            }
            // Damping keeps unseen directions (often depth) at zero and
            // shrinks what only one view claims.
            let d = (m + Mat3::IDENTITY * (DAMPING * tw)).inverse() * b;
            d.is_finite().then(|| d.clamp_length_max(MAX_OFFSET))
        })
        .collect()
}

/// 1 on the head, 0 below the neck, smooth between.
pub fn neck_fade(p: Vec3, head: (Vec3, f32)) -> f32 {
    let (c, r) = head;
    let t = ((p.y - (c.y - r * 1.7)) / (r * 0.4)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn kernel(a: Vec3, b: Vec3) -> f64 {
    (-(a.distance_squared(b) / (SIGMA * SIGMA)) as f64).exp()
}

/// Spreads `offsets` at face vertex `verts` over `coords`: returns the
/// non-zero per-vertex offsets (vertex index, decimetres).
pub fn spread(coords: &[Vec3], verts: &[u32], offsets: &[Option<Vec3>], head: (Vec3, f32)) -> Vec<(u32, [f32; 3])> {
    // One control per vertex (several face points can share a vertex).
    let mut ctrl: BTreeMap<u32, (Vec3, f32)> = BTreeMap::new();
    for (&v, o) in verts.iter().zip(offsets) {
        if let Some(o) = o {
            let e = ctrl.entry(v).or_insert((Vec3::ZERO, 0.0));
            (e.0, e.1) = (e.0 + *o, e.1 + 1.0);
        }
    }
    let ctrl: Vec<(Vec3, Vec3)> = ctrl.into_iter().map(|(v, (s, n))| (coords[v as usize], s / n)).collect();
    let n = ctrl.len();
    if n == 0 {
        return Vec::new();
    }
    let mut k = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            k[i * n + j] = kernel(ctrl[i].0, ctrl[j].0);
        }
    }
    let weights: Vec<Vec<f64>> = (0..3).filter_map(|axis| solve_ridge(k.clone(), ctrl.iter().map(|c| c.1[axis] as f64).collect(), n, 0.15)).collect();
    if weights.len() < 3 {
        return Vec::new();
    }
    let (c, r) = head;
    let mut out = Vec::new();
    for (i, &p) in coords.iter().enumerate() {
        if p.distance(c) > r * 2.2 {
            continue;
        }
        let fade = neck_fade(p, head);
        if fade <= 0.0 {
            continue;
        }
        let mut d = [0.0f64; 3];
        for (j, (q, _)) in ctrl.iter().enumerate() {
            let kk = kernel(p, *q);
            if kk < 1e-5 {
                continue;
            }
            for (a, w) in d.iter_mut().zip(&weights) {
                *a += w[j] * kk;
            }
        }
        let d = Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32) * fade;
        if d.length_squared() > 1e-8 {
            out.push((i as u32, d.to_array()));
        }
    }
    out
}
