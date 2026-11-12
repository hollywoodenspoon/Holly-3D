//! Albedo-demodulated, edge-avoiding À-Trous wavelet denoiser (after the
//! SVGF filter in Strolle, MIT): divide the noisy radiance by the surface
//! albedo, run 5 widening B3-spline passes (1, 2, 4, 8, 16 px) whose taps
//! are weighted by world normal, linear depth, roughness, object / material
//! ID and variance-scaled luminance, then multiply the crisp albedo back.
//! Bricks, stamps and painted detail stay sharp at 4..8 spp.

use super::sample::Pixel;
use bevy::math::Vec3;

pub const ITERATIONS: usize = 5;
const B3: [f32; 5] = [1.0 / 16.0, 0.25, 3.0 / 8.0, 0.25, 1.0 / 16.0];
/// Luminance edge-stopping strength, in standard deviations.
const SIGMA_L: f32 = 4.0;

/// Per-pixel guides the path tracer keeps for the denoiser.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Aux {
    pub albedo: Vec3,
    pub rough: f32,
    /// Variance of the pixel's mean luminance (sample variance / n).
    pub var: f32,
}

fn lum(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// Albedo that radiance is divided by (never ~0, so black stays black).
fn demod(a: Vec3) -> Vec3 {
    a.max(Vec3::splat(0.02))
}

/// Denoises `px` in place (`w` x `h`); background pixels are left alone.
pub fn denoise(px: &mut [Pixel], aux: &[Aux], w: usize, h: usize) {
    if px.len() != w * h || aux.len() != px.len() {
        return;
    }
    let surf: Vec<bool> = px.iter().map(|p| p.g.obj != u32::MAX).collect();
    let mut irr: Vec<Vec3> = px.iter().zip(aux).map(|(p, a)| p.layers.geom.truncate() / demod(a.albedo)).collect();
    let mut var: Vec<f32> = aux.iter().map(|a| a.var / lum(demod(a.albedo)).powi(2)).collect();
    for it in 0..ITERATIONS {
        let step = 1i32 << it;
        let (src, vsrc) = (irr.clone(), var.clone());
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !surf[i] {
                    continue;
                }
                let (c, gc, ac) = (src[i], px[i].g, aux[i]);
                let (lc, sd) = (lum(c), vsrc[i].max(0.0).sqrt());
                let (mut sum, mut vsum, mut wsum) = (Vec3::ZERO, 0.0, 0.0);
                for (ky, by) in B3.iter().enumerate() {
                    for (kx, bx) in B3.iter().enumerate() {
                        let (sx, sy) = (x as i32 + (kx as i32 - 2) * step, y as i32 + (ky as i32 - 2) * step);
                        if sx < 0 || sy < 0 || sx >= w as i32 || sy >= h as i32 {
                            continue;
                        }
                        let j = sy as usize * w + sx as usize;
                        let gs = px[j].g;
                        if !surf[j] || gs.obj != gc.obj || gs.mat != gc.mat {
                            continue;
                        }
                        let wn = gs.normal.dot(gc.normal).max(0.0).powi(64);
                        let wz = (-(gs.depth - gc.depth).abs() / (gc.depth * 0.01 * step as f32 + 1e-4)).exp();
                        let wr = (-(aux[j].rough - ac.rough).abs() * 10.0).exp();
                        let wl = (-(lum(src[j]) - lc).abs() / (SIGMA_L * sd + 1e-4)).exp();
                        let k = by * bx * wn * wz * wr * wl;
                        sum += src[j] * k;
                        vsum += vsrc[j] * k * k;
                        wsum += k;
                    }
                }
                if wsum > 1e-8 {
                    irr[i] = sum / wsum;
                    var[i] = vsum / (wsum * wsum);
                }
            }
        }
    }
    for ((p, a), (c, s)) in px.iter_mut().zip(aux).zip(irr.into_iter().zip(&surf)) {
        if *s {
            let rgb = c * demod(a.albedo);
            p.layers.geom = rgb.extend(p.layers.geom.w);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::trace::GSample;
    use bevy::math::Vec4;

    /// A flat wall with a sharp 4 px checker albedo under uniform light,
    /// rendered with heavy per-pixel noise.
    fn wall(w: usize, h: usize) -> (Vec<Pixel>, Vec<Aux>, Vec<Vec3>) {
        let mut rng = crate::render::trace::Rng::new(7);
        let mut px = vec![];
        let (mut aux, mut clean) = (vec![], vec![]);
        for y in 0..h {
            for x in 0..w {
                let a = if (x / 4 + y / 4) % 2 == 0 { Vec3::new(0.9, 0.2, 0.1) } else { Vec3::new(0.1, 0.3, 0.9) };
                let noise = 1.0 + (rng.f() - 0.5) * 1.6;
                let g = GSample { obj: 0, mat: 0, depth: 5.0, normal: Vec3::Z };
                let mut p = Pixel { g, ..Pixel::EMPTY };
                p.layers.geom = (a * noise).extend(1.0);
                px.push(p);
                aux.push(Aux { albedo: a, rough: 0.5, var: 0.2 });
                clean.push(a);
            }
        }
        (px, aux, clean)
    }

    fn err(px: &[Pixel], clean: &[Vec3]) -> f32 {
        px.iter().zip(clean).map(|(p, c)| (p.layers.geom.truncate() - *c).length_squared()).sum::<f32>() / px.len() as f32
    }

    #[test]
    fn demodulated_atrous_keeps_texture_and_removes_noise() {
        let (w, h) = (32, 32);
        let (mut px, aux, clean) = wall(w, h);
        let before = err(&px, &clean);
        denoise(&mut px, &aux, w, h);
        let after = err(&px, &clean);
        assert!(after < before * 0.1, "noise mostly removed: {before} -> {after}");
        // Checker edges stay razor sharp: neighbours across an edge keep
        // their own albedo.
        for i in [4 * w + 3, 4 * w + 4, 3 * w + 4] {
            let (got, want) = (px[i].layers.geom.truncate(), clean[i]);
            assert!((got - want).abs().max_element() < 0.12, "edge pixel {i} keeps its albedo: {got} vs {want}");
        }
        assert!(px.iter().all(|p| p.layers.geom.w == 1.0), "coverage untouched");
    }

    #[test]
    fn objects_and_background_do_not_bleed() {
        let (w, h) = (16, 8);
        let (mut px, aux, _) = wall(w, h);
        for (i, p) in px.iter_mut().enumerate() {
            if i % w >= 8 {
                p.g.obj = 1;
                p.layers.geom = Vec4::new(5.0, 5.0, 5.0, 1.0);
            }
            if i < w {
                *p = Pixel::EMPTY;
                p.layers.geom = Vec4::new(0.0, 0.0, 0.0, 0.0);
            }
        }
        denoise(&mut px, &aux, w, h);
        assert!(px[3 * w + 7].layers.geom.x < 2.0, "dark object stays dark next to a bright one");
        assert_eq!(px[3].layers.geom, Vec4::ZERO, "background untouched");
    }
}
