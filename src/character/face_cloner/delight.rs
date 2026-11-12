//! Takes the lighting out of one photo's skin samples. The shading of a
//! matte surface under any distant light is close to a second-order
//! spherical harmonic of its normal (Ramamoorthi & Hanrahan, 2001), so
//! the brightness of the skin samples is fitted by 9 SH coefficients
//! (assuming the skin itself is evenly coloured at that scale) and
//! divided out. Finer detail (pores, freckles, brows) and colour stay.

use bevy::math::Vec3;

/// One skin sample from one photo: linear colour, head-space normal, weight.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub color: Vec3,
    pub normal: Vec3,
    pub weight: f32,
}

pub fn luminance(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// The 9 real SH basis functions (constants folded into the coefficients).
fn basis(n: Vec3) -> [f64; 9] {
    let (x, y, z) = (n.x as f64, n.y as f64, n.z as f64);
    [1.0, y, z, x, x * y, y * z, 3.0 * z * z - 1.0, x * z, x * x - y * y]
}

/// Solves `a x = b` (Gaussian elimination with partial pivoting).
fn solve<const N: usize>(mut a: [[f64; N]; N], mut b: [f64; N]) -> Option<[f64; N]> {
    for c in 0..N {
        let p = (c..N).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[p][c].abs() < 1e-12 {
            return None;
        }
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..N {
            let f = a[r][c] / a[c][c];
            for k in c..N {
                a[r][k] -= f * a[c][k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = [0.0; N];
    for c in (0..N).rev() {
        x[c] = (b[c] - (c + 1..N).map(|k| a[c][k] * x[k]).sum::<f64>()) / a[c][c];
    }
    Some(x)
}

/// The fitted shading of a normal.
#[derive(Clone, Copy, Debug)]
pub struct Shading {
    coef: [f64; 9],
    /// Weighted mean shading over the samples.
    pub mean: f32,
}

impl Shading {
    pub fn at(&self, n: Vec3) -> f32 {
        basis(n).iter().zip(&self.coef).map(|(b, c)| b * c).sum::<f64>() as f32
    }
}

/// Fits the shading of `samples` (None when there are too few).
pub fn fit(samples: &[Sample]) -> Option<Shading> {
    let used: Vec<&Sample> = samples.iter().filter(|s| s.weight > 0.2).collect();
    if used.len() < 200 {
        return None;
    }
    let (mut a, mut b) = ([[0.0f64; 9]; 9], [0.0f64; 9]);
    for s in &used {
        let (y, w, l) = (basis(s.normal), s.weight as f64, luminance(s.color) as f64);
        for i in 0..9 {
            for j in 0..9 {
                a[i][j] += w * y[i] * y[j];
            }
            b[i] += w * y[i] * l;
        }
    }
    let ridge = (0..9).map(|i| a[i][i]).sum::<f64>() * 1e-4;
    (1..9).for_each(|i| a[i][i] += ridge);
    let coef = solve(a, b)?;
    let sh = Shading { coef, mean: 0.0 };
    let (sw, sl) = used.iter().fold((0.0, 0.0), |(w, l), s| (w + s.weight, l + s.weight * sh.at(s.normal)));
    (sl > 0.0).then(|| Shading { mean: sl / sw, ..sh })
}

/// Divides the fitted shading out of every sample (in place), then
/// scales the photo to mean luminance 1; returns its mean luminance
/// before (its exposure), or None when nothing could be fitted.
pub fn delight(samples: &mut [Sample]) -> Option<f32> {
    let total: f32 = samples.iter().map(|s| s.weight).sum();
    if total <= 0.0 {
        return None;
    }
    let raw = samples.iter().map(|s| luminance(s.color) * s.weight).sum::<f32>() / total;
    if let Some(sh) = fit(samples) {
        let floor = sh.mean * 0.2;
        for s in samples.iter_mut() {
            s.color *= sh.mean / sh.at(s.normal).max(floor);
        }
    }
    let now = samples.iter().map(|s| luminance(s.color) * s.weight).sum::<f32>() / total;
    if now > 1e-6 {
        samples.iter_mut().for_each(|s| s.color /= now);
    }
    Some(raw)
}

pub fn to_linear(c: u8) -> f32 {
    let v = c as f32 / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

pub fn to_srgb(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lit_even_surface_comes_out_even() {
        let light = Vec3::new(0.6, 0.5, 0.6).normalize();
        let albedo = Vec3::new(0.6, 0.4, 0.3);
        let mut s: Vec<Sample> = (0..2000)
            .map(|i| {
                let (a, b) = (i as f32 * 0.37, (i as f32 * 0.11).sin() * 1.2);
                let n = Vec3::new(a.sin() * b.cos(), b.sin(), a.cos().abs() * b.cos() + 0.2).normalize();
                Sample { color: albedo * (0.3 + 0.7 * n.dot(light).max(0.0)), normal: n, weight: 1.0 }
            })
            .collect();
        let spread = |s: &[Sample]| {
            let l: Vec<f32> = s.iter().map(|x| luminance(x.color)).collect();
            let m = l.iter().sum::<f32>() / l.len() as f32;
            (l.iter().map(|v| (v - m).powi(2)).sum::<f32>() / l.len() as f32).sqrt() / m
        };
        let before = spread(&s);
        delight(&mut s).unwrap();
        let after = spread(&s);
        assert!(after < before * 0.25, "shading should mostly go ({before} -> {after})");
        // Colour is kept: the ratio of channels stays the albedo's.
        let c = s[7].color;
        assert!((c.x / c.y - 1.5).abs() < 0.01 && (c.y / c.z - 4.0 / 3.0).abs() < 0.01, "{c}");
        assert_eq!(to_srgb(to_linear(200)), 200);
    }
}
