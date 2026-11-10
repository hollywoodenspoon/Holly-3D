//! Post passes on a finished region, in order: edge-preserving spatial
//! denoiser (G-buffer aware bilateral), depth of field (thin-lens circle of
//! confusion, gather blur), the universal ink outline, bloom / glare, then
//! the compositor that produces display + HDR from each pixel's layers.

use super::outline;
use super::sample::Pixel;
use super::scene_build::RenderScene;
use super::settings::RenderSettings;
use bevy::math::{Vec3, Vec4};

/// Runs every enabled pass over `px` (`w` x `h`, row-major).
pub fn run(px: &mut [Pixel], w: usize, h: usize, rs: &RenderScene, cfg: &RenderSettings) {
    if w == 0 || h == 0 {
        return;
    }
    let noisy = cfg.gi.gi || cfg.gi.ao || !rs.emitters.is_empty();
    if cfg.gi.denoise && noisy {
        denoise(px, w, h);
    }
    let p = &cfg.post;
    if p.dof && !rs.camera.is_ortho() {
        dof(px, w, h, coc_scale(rs, cfg), p.focus_distance.max(1e-3));
    }
    if cfg.outlines {
        let g: Vec<_> = px.iter().map(|p| p.g).collect();
        let mut ink = vec![Vec4::ZERO; px.len()];
        outline::apply(&g, w, h, &rs.materials, &mut ink);
        px.iter_mut().zip(ink).for_each(|(p, k)| p.layers.ink = k);
    }
    if p.bloom {
        bloom(px, w, h, cfg);
    }
    let plain = rs.hdri.is_none();
    px.iter_mut().for_each(|p| p.finish(&cfg.post.tone, plain, cfg.transparent));
}

/// Bilateral weights: same object, similar normal, depth and color.
fn denoise(px: &mut [Pixel], w: usize, h: usize) {
    const R: i32 = 3;
    let src: Vec<Pixel> = px.to_vec();
    for y in 0..h {
        for x in 0..w {
            let c = &src[y * w + x];
            if c.g.obj == u32::MAX {
                continue;
            }
            let cc = c.layers.geom;
            let (mut sum, mut wsum) = (Vec4::ZERO, 0.0);
            for dy in -R..=R {
                for dx in -R..=R {
                    let (sx, sy) = (x as i32 + dx, y as i32 + dy);
                    if sx < 0 || sy < 0 || sx >= w as i32 || sy >= h as i32 {
                        continue;
                    }
                    let s = &src[sy as usize * w + sx as usize];
                    if s.g.obj != c.g.obj {
                        continue;
                    }
                    let nd = s.g.normal.dot(c.g.normal);
                    if nd < 0.9 || (s.g.depth - c.g.depth).abs() > c.g.depth * 0.02 {
                        continue;
                    }
                    let spatial = (-((dx * dx + dy * dy) as f32) / (2.0 * 2.0 * 2.0)).exp();
                    let dc = (s.layers.geom - cc).truncate().length() / (cc.truncate().length() + 0.05);
                    let wgt = spatial * (-dc * dc * 2.0).exp() * ((nd - 0.9) * 10.0);
                    sum += s.layers.geom * wgt;
                    wsum += wgt;
                }
            }
            if wsum > 0.0 {
                px[y * w + x].layers.geom = sum / wsum;
            }
        }
    }
}

/// Pixels of blur per unit of |1/focus - 1/depth| (thin lens, 36 mm back,
/// 1 scene unit = 1 m).
fn coc_scale(rs: &RenderScene, cfg: &RenderSettings) -> f32 {
    let aspect = cfg.width as f32 / cfg.height.max(1) as f32;
    let fov_x = 2.0 * ((rs.camera.fov_y(aspect) * 0.5).tan() * aspect).atan();
    let f = 0.018 / (fov_x * 0.5).tan(); // focal length in metres
    let aperture = f / cfg.post.f_stop.max(0.5);
    // CoC on the sensor = A * f * |1/s - 1/d| (for d, s >> f), in pixels.
    aperture * f / 0.036 * cfg.width as f32
}

/// Circle-of-confusion radius (pixels) of a pixel at `depth`.
pub fn coc(scale: f32, focus: f32, depth: f32) -> f32 {
    let inv = if depth.is_finite() { 1.0 / depth.max(1e-3) } else { 0.0 };
    (scale * (1.0 / focus - inv).abs()).min(24.0)
}

fn dof(px: &mut [Pixel], w: usize, h: usize, scale: f32, focus: f32) {
    let radius: Vec<f32> = px.iter().map(|p| coc(scale, focus, p.g.depth)).collect();
    let src: Vec<Pixel> = px.to_vec();
    const TAPS: usize = 48;
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    for y in 0..h {
        for x in 0..w {
            let r = radius[y * w + x];
            if r < 0.5 {
                continue;
            }
            let (mut geom, mut back, mut wsum) = (Vec4::ZERO, Vec3::ZERO, 0.0);
            for i in 0..TAPS {
                let rr = r * ((i as f32 + 0.5) / TAPS as f32).sqrt();
                let a = i as f32 * golden;
                let (sx, sy) = ((x as f32 + rr * a.cos()).round() as i32, (y as f32 + rr * a.sin()).round() as i32);
                let (sx, sy) = (sx.clamp(0, w as i32 - 1) as usize, sy.clamp(0, h as i32 - 1) as usize);
                let j = sy * w + sx;
                // A sharp, nearer neighbour does not bleed into a blurred one.
                if radius[j] + 0.5 < rr && src[j].g.depth < src[y * w + x].g.depth {
                    continue;
                }
                geom += src[j].layers.geom;
                back += src[j].layers.back;
                wsum += 1.0;
            }
            if wsum > 0.0 {
                let l = &mut px[y * w + x].layers;
                (l.geom, l.back) = (geom / wsum, back / wsum);
            }
        }
    }
}

fn blur_1d(img: &[Vec3], w: usize, h: usize, sigma: f32, horizontal: bool) -> Vec<Vec3> {
    let r = (sigma * 3.0).ceil() as i32;
    let k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let ksum: f32 = k.iter().sum();
    let mut out = vec![Vec3::ZERO; img.len()];
    for y in 0..h {
        for x in 0..w {
            let mut s = Vec3::ZERO;
            for (i, kv) in k.iter().enumerate() {
                let o = i as i32 - r;
                let (sx, sy) = if horizontal {
                    ((x as i32 + o).clamp(0, w as i32 - 1), y as i32)
                } else {
                    (x as i32, (y as i32 + o).clamp(0, h as i32 - 1))
                };
                s += img[sy as usize * w + sx as usize] * *kv;
            }
            out[y * w + x] = s / ksum;
        }
    }
    out
}

/// Bright parts (luminance above the threshold) blurred into the glow layer.
fn bloom(px: &mut [Pixel], w: usize, h: usize, cfg: &RenderSettings) {
    let p = &cfg.post;
    let bright: Vec<Vec3> = px
        .iter()
        .map(|q| {
            let c = q.layers.hdr(false).truncate();
            let l = c.dot(Vec3::new(0.2126, 0.7152, 0.0722));
            if l > p.bloom_threshold { c * ((l - p.bloom_threshold) / l.max(1e-6)) } else { Vec3::ZERO }
        })
        .collect();
    if bright.iter().all(|c| *c == Vec3::ZERO) {
        return;
    }
    let sigma = (p.bloom_radius * w as f32 / 800.0).clamp(0.5, 64.0) / 2.0;
    let blurred = blur_1d(&blur_1d(&bright, w, h, sigma, true), w, h, sigma, false);
    for (q, b) in px.iter_mut().zip(blurred) {
        q.layers.glow = b * p.bloom_intensity.max(0.0);
    }
}
