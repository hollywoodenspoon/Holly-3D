//! Progressive full-frame passes (1 -> 2 -> 4 -> ... -> target spp): every
//! pass adds samples to each unconverged pixel across worker threads, then
//! the whole region streams to the Rendered Frame Window; the last pass runs
//! the post-FX chain (denoiser, bloom, DOF, outlines).

use super::adaptive::{Welford, converged};
use super::integrator::PathTracer;
use crate::render::job::Region;
use crate::render::post::PixelLayers;
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::trace::{GSample, Pixel, Rng, Tracer};
use bevy::math::{Vec2, Vec3, Vec4};
use std::sync::atomic::{AtomicBool, Ordering};

/// Accumulated samples of one pixel.
#[derive(Clone, Copy)]
pub struct Accum {
    pub sum: Vec4,
    pub back: Vec3,
    pub g: GSample,
    pub n: u32,
    pub stats: Welford,
}

impl Accum {
    const EMPTY: Accum = Accum { sum: Vec4::ZERO, back: Vec3::ZERO, g: GSample::EMPTY, n: 0, stats: Welford::ZERO };
}

/// What a finished pass reports: its index, samples per pixel so far and
/// the share of pixels still sampling.
#[derive(Clone, Copy, Debug)]
pub struct PassInfo {
    pub pass: usize,
    pub spp: u32,
    pub active: f32,
}

/// SplitMix64-scrambled seed, so neighbouring pixels and successive passes
/// start uncorrelated xorshift streams.
fn seed(x: u32, y: u32, pass: usize) -> u64 {
    let mut z = (u64::from(y) << 40 ^ u64::from(x) << 16 ^ pass as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Renders `region` progressively; `on_pass` sees the whole region after
/// every pass. Returns the final pixels (post-FX applied unless cancelled).
pub fn render(
    rs: &RenderScene,
    cfg: &RenderSettings,
    region: Region,
    cancel: &AtomicBool,
    mut on_pass: impl FnMut(PassInfo, &[Pixel]),
) -> Vec<Pixel> {
    let ps = &cfg.path;
    let [x0, y0, x1, y1] = region;
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let pt = PathTracer::new(Tracer::new(rs, cfg), ps);
    let mut acc = vec![Accum::EMPTY; rw * rh];
    let mut done = vec![false; rw * rh];
    let mut out = vec![Pixel::EMPTY; rw * rh];
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, rh.max(1));
    let mut have = 0;
    for (pass, &target) in ps.schedule().iter().enumerate() {
        let add = target - have;
        have = target;
        let rows: Vec<_> = acc.chunks_mut(rw).zip(done.chunks(rw)).collect();
        let rows = std::sync::Mutex::new(rows.into_iter().enumerate().collect::<Vec<_>>());
        std::thread::scope(|s| {
            for _ in 0..workers {
                let (pt, rows) = (&pt, &rows);
                s.spawn(move || {
                    while let Some((r, (row, fin))) = rows.lock().ok().and_then(|mut v| v.pop()) {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let y = y0 + r as u32;
                        for (i, a) in row.iter_mut().enumerate() {
                            if !fin[i] {
                                sample_pixel(pt, a, x0 + i as u32, y, add, pass, cfg);
                            }
                        }
                    }
                });
            }
        });
        if cancel.load(Ordering::Relaxed) {
            return out;
        }
        if ps.adaptive {
            let stats: Vec<_> = acc.iter().map(|a| a.stats).collect();
            done = converged(&stats, rw, rh, ps.noise_threshold, ps.min_samples);
        }
        for (p, a) in out.iter_mut().zip(&acc) {
            *p = resolve(a, cfg, rs.hdri.is_none());
        }
        let active = done.iter().filter(|d| !**d).count() as f32 / done.len().max(1) as f32;
        on_pass(PassInfo { pass, spp: target, active }, &out);
        if active == 0.0 {
            break;
        }
    }
    crate::render::postfx::run(&mut out, rw, rh, rs, cfg);
    out
}

fn sample_pixel(pt: &PathTracer, a: &mut Accum, x: u32, y: u32, n: u32, pass: usize, cfg: &RenderSettings) {
    let (w, h) = (cfg.width, cfg.height);
    let aspect = w as f32 / h as f32;
    let mut rng = Rng::new(seed(x, y, pass));
    let cam = &pt.t.rs.camera;
    if a.n == 0 {
        let ndc = Vec2::new((x as f32 + 0.5) / w as f32 * 2.0 - 1.0, 1.0 - (y as f32 + 0.5) / h as f32 * 2.0);
        let ray = cam.ray(ndc, aspect);
        a.back = pt.t.backdrop(ray.dir);
        // The G-buffer (outlines, denoiser) comes from the pixel center.
        a.g = pt.radiance(ray.origin, ray.dir, &mut rng).g.unwrap_or(GSample::EMPTY);
    }
    for _ in 0..n {
        let ndc = Vec2::new((x as f32 + rng.f()) / w as f32 * 2.0 - 1.0, 1.0 - (y as f32 + rng.f()) / h as f32 * 2.0);
        let ray = cam.ray(ndc, aspect);
        let s = pt.radiance(ray.origin, ray.dir, &mut rng);
        let c = if s.radiance.is_finite() { s.radiance } else { Vec3::ZERO };
        a.sum += c.extend(s.alpha);
        a.n += 1;
        a.stats.push(c.dot(Vec3::new(0.2126, 0.7152, 0.0722)));
    }
}

/// Averages a pixel's samples into display / HDR values.
pub fn resolve(a: &Accum, cfg: &RenderSettings, plain_bg: bool) -> Pixel {
    let k = 1.0 / a.n.max(1) as f32;
    let mut px = Pixel { g: a.g, layers: PixelLayers { geom: a.sum * k, back: a.back, ..PixelLayers::ZERO }, ..Pixel::EMPTY };
    px.finish(&cfg.post.tone, plain_bg, cfg.transparent);
    px
}
