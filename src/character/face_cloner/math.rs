//! Small dense solvers for the face fit: symmetric eigenvectors (Jacobi),
//! Horn's closed-form similarity transform, and Cholesky least squares.

use bevy::math::{DQuat, DVec3, Quat, Vec3};

/// Eigen-decomposition of a symmetric `n x n` matrix (row-major) by cyclic
/// Jacobi rotations: (eigenvalues, eigenvectors as columns, row-major).
pub fn jacobi(mut a: Vec<f64>, n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut v = vec![0.0; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    for _ in 0..64 {
        let off: f64 = (0..n).flat_map(|i| (0..n).filter(move |&j| j != i).map(move |j| (i, j))).map(|(i, j)| a[i * n + j].powi(2)).sum();
        if off < 1e-22 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let apq = a[p * n + q];
                if apq.abs() < 1e-30 {
                    continue;
                }
                let theta = (a[q * n + q] - a[p * n + p]) / (2.0 * apq);
                let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let (c, s) = (1.0 / (t * t + 1.0).sqrt(), t / (t * t + 1.0).sqrt());
                for k in 0..n {
                    let (akp, akq) = (a[k * n + p], a[k * n + q]);
                    a[k * n + p] = c * akp - s * akq;
                    a[k * n + q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p * n + k], a[q * n + k]);
                    a[p * n + k] = c * apk - s * aqk;
                    a[q * n + k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k * n + p], v[k * n + q]);
                    v[k * n + p] = c * vkp - s * vkq;
                    v[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..n).map(|i| a[i * n + i]).collect(), v)
}

/// A similarity transform `dst ~ scale * rot * src + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Similarity {
    pub scale: f32,
    pub rot: Quat,
    pub offset: Vec3,
}

impl Similarity {
    pub const IDENTITY: Self = Self { scale: 1.0, rot: Quat::IDENTITY, offset: Vec3::ZERO };

    pub fn apply(&self, p: Vec3) -> Vec3 {
        self.rot * p * self.scale + self.offset
    }
}

/// Horn's quaternion method: the weighted least-squares similarity taking
/// `src[i]` to `dst[i]` (`fixed_scale` keeps a known scale).
pub fn similarity(src: &[Vec3], dst: &[Vec3], w: &[f32], fixed_scale: Option<f32>) -> Similarity {
    let tw: f64 = w.iter().map(|&x| x as f64).sum::<f64>().max(1e-12);
    let mean = |pts: &[Vec3]| pts.iter().zip(w).map(|(p, &k)| p.as_dvec3() * k as f64).sum::<DVec3>() / tw;
    let (ms, md) = (mean(src), mean(dst));
    let mut m = [[0.0f64; 3]; 3];
    let mut ss = 0.0;
    for ((s, d), &k) in src.iter().zip(dst).zip(w) {
        let (a, b) = (s.as_dvec3() - ms, d.as_dvec3() - md);
        let (a3, b3) = (a.to_array(), b.to_array());
        for (i, row) in m.iter_mut().enumerate() {
            for (j, e) in row.iter_mut().enumerate() {
                *e += k as f64 * a3[i] * b3[j];
            }
        }
        ss += k as f64 * a.length_squared();
    }
    let [[sxx, sxy, sxz], [syx, syy, syz], [szx, szy, szz]] = m;
    #[rustfmt::skip]
    let n = vec![
        sxx + syy + szz, syz - szy, szx - sxz, sxy - syx,
        syz - szy, sxx - syy - szz, sxy + syx, szx + sxz,
        szx - sxz, sxy + syx, -sxx + syy - szz, syz + szy,
        sxy - syx, szx + sxz, syz + szy, -sxx - syy + szz,
    ];
    let (vals, vecs) = jacobi(n, 4);
    let best = (0..4).max_by(|&a, &b| vals[a].total_cmp(&vals[b])).unwrap_or(0);
    let q = DQuat::from_xyzw(vecs[4 + best], vecs[8 + best], vecs[12 + best], vecs[best]).normalize();
    let rot = q.as_quat();
    let scale = fixed_scale.unwrap_or_else(|| {
        let dot: f64 = src.iter().zip(dst).zip(w).map(|((s, d), &k)| k as f64 * (q * (s.as_dvec3() - ms)).dot(d.as_dvec3() - md)).sum();
        (dot / ss.max(1e-12)) as f32
    });
    let offset = (md - (q * ms) * scale as f64).as_vec3();
    Similarity { scale, rot, offset }
}

/// Solves the dense least-squares normal equations `(AtA + ridge I) x = Atb`
/// by Cholesky; `ata` is `n x n` row-major. Returns None if not positive definite.
pub fn solve_ridge(mut ata: Vec<f64>, mut atb: Vec<f64>, n: usize, ridge: f64) -> Option<Vec<f64>> {
    for i in 0..n {
        ata[i * n + i] += ridge;
    }
    // In-place Cholesky: lower triangle holds L.
    for j in 0..n {
        let mut d = ata[j * n + j];
        for k in 0..j {
            d -= ata[j * n + k].powi(2);
        }
        if d <= 0.0 {
            return None;
        }
        let d = d.sqrt();
        ata[j * n + j] = d;
        for i in j + 1..n {
            let mut s = ata[i * n + j];
            for k in 0..j {
                s -= ata[i * n + k] * ata[j * n + k];
            }
            ata[i * n + j] = s / d;
        }
    }
    for i in 0..n {
        let mut s = atb[i];
        for k in 0..i {
            s -= ata[i * n + k] * atb[k];
        }
        atb[i] = s / ata[i * n + i];
    }
    for i in (0..n).rev() {
        let mut s = atb[i];
        for k in i + 1..n {
            s -= ata[k * n + i] * atb[k];
        }
        atb[i] = s / ata[i * n + i];
    }
    Some(atb)
}
