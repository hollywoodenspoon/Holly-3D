//! Progressive full-frame passes (1 -> 2 -> 4 -> ... -> target spp): every
//! pass adds samples to each unconverged pixel across worker threads, then
//! the whole region streams to the Rendered Frame Window; the last pass runs
//! the post-FX chain (denoiser, bloom, DOF, outlines).

use super::adaptive::{Welford, converged};
use super::integrator::PathTracer;
use crate::render::job::Region;
pub use super::extras::Extras;
use crate::render::aov::AovPixel;
use crate::render::lightmix::{GROUPS, VOLUME};
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
    /// AOV sums (radiance passes, albedo, AO); `aov.samples` unused.
    pub aov: AovPixel,
    pub rough: f32,
}

impl Accum {
    pub const EMPTY: Accum = Accum {
        sum: Vec4::ZERO,
        back: Vec3::ZERO,
        g: GSample::EMPTY,
        n: 0,
        stats: Welford::ZERO,
        aov: AovPixel { direct: Vec3::ZERO, indirect: Vec3::ZERO, volume: Vec3::ZERO, specular: Vec3::ZERO, sss: Vec3::ZERO, albedo: Vec3::ZERO, normal: Vec3::ZERO, depth: 0.0, ao: 0.0, samples: 0 },
        rough: 0.0,
    };
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
pub fn render(rs: &RenderScene, cfg: &RenderSettings, region: Region, cancel: &AtomicBool, on_pass: impl FnMut(PassInfo, &[Pixel])) -> Vec<Pixel> {
    render_mix(rs, cfg, region, cancel, on_pass).0
}

/// [`render`] that also returns the AOV passes and, when
/// `PathSettings::lightmix` is on, the per-light-group LightMix buffers
/// (both empty if cancelled).
pub fn render_mix(
    rs: &RenderScene,
    cfg: &RenderSettings,
    region: Region,
    cancel: &AtomicBool,
    mut on_pass: impl FnMut(PassInfo, &[Pixel]),
) -> (Vec<Pixel>, Extras) {
    let ps = &cfg.path;
    let [x0, y0, x1, y1] = region;
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let pt = PathTracer::new(Tracer::new(rs, cfg), ps);
    let k = if ps.lightmix { rs.groups.len().min(GROUPS) } else { 0 };
    let mut gsum = vec![Vec3::ZERO; rw * rh * k];
    let mut acc = vec![Accum::EMPTY; rw * rh];
    let mut done = vec![false; rw * rh];
    let mut out = vec![Pixel::EMPTY; rw * rh];
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, rh.max(1));
    let mut have = 0;
    for (pass, &target) in ps.schedule().iter().enumerate() {
        let add = target - have;
        have = target;
        let mut gs: Vec<&mut [Vec3]> = if k > 0 { gsum.chunks_mut(rw * k).collect() } else { (0..rh).map(|_| &mut [][..]).collect() };
        let rows: Vec<_> = acc.chunks_mut(rw).zip(done.chunks(rw)).zip(gs.drain(..)).collect();
        let rows = std::sync::Mutex::new(rows.into_iter().enumerate().collect::<Vec<_>>());
        std::thread::scope(|s| {
            for _ in 0..workers {
                let (pt, rows) = (&pt, &rows);
                s.spawn(move || {
                    while let Some((r, ((row, fin), gr))) = rows.lock().ok().and_then(|mut v| v.pop()) {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let y = y0 + r as u32;
                        for (i, a) in row.iter_mut().enumerate() {
                            if !fin[i] {
                                let g = if k > 0 { &mut gr[i * k..(i + 1) * k] } else { &mut [][..] };
                                sample_pixel(pt, a, g, x0 + i as u32, y, add, pass, cfg);
                            }
                        }
                    }
                });
            }
        });
        if cancel.load(Ordering::Relaxed) {
            return (out, Extras::default());
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
    let aux = super::extras::aux(&acc);
    let ctx = crate::render::postfx::PostCtx { aux: Some(&aux), ..crate::render::postfx::PostCtx::of(rs, cfg) };
    crate::render::postfx::run_ctx(&mut out, rw, rh, &ctx, cfg);
    let lightmix = (k > 0).then(|| super::extras::lightmix(rs, cfg, region, &acc, gsum, k, aux));
    (out, Extras { lightmix, aovs: Some(super::extras::aovs(region, &acc)) })
}

#[allow(clippy::too_many_arguments)]
fn sample_pixel(pt: &PathTracer, a: &mut Accum, groups: &mut [Vec3], x: u32, y: u32, n: u32, pass: usize, cfg: &RenderSettings) {
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
        let ok = s.radiance.is_finite();
        let c = if ok { s.radiance } else { Vec3::ZERO };
        if ok {
            groups.iter_mut().zip(&s.groups).for_each(|(g, v)| *g += *v);
            let v = &mut a.aov;
            let ind = s.radiance - s.direct;
            (v.direct, v.indirect, v.volume) = (v.direct + s.direct, v.indirect + ind, v.volume + s.groups[VOLUME as usize]);
            match s.lobe {
                2 => v.specular += ind,
                3 => v.sss += ind,
                _ => {}
            }
        }
        (a.aov.albedo, a.aov.ao, a.rough) = (a.aov.albedo + s.albedo, a.aov.ao + s.ao, a.rough + s.rough);
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
