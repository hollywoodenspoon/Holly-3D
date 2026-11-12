//! Unbiased multi-bounce path tracing on the CPU: next-event estimation on
//! punctual lights and emissive polygons, BSDF sampling, both combined with
//! the power heuristic (no firefly clamps), per-lobe bounce limits, Russian
//! Roulette, stochastic opacity, Beer-Lambert crystal absorption and fog.

use super::bsdf::{Bsdf, Lobe};
use super::settings::PathSettings;
use crate::materials::ShadingModel;
use crate::materials::shading::{camera_view, lin3, shade};
use crate::render::emitters::point_on;
use crate::render::lightmix::{EMISSIVE, ENV, Groups, SUN_SKY, VOLUME};
use crate::render::mis::{area_to_solid_angle, power_heuristic};
use crate::render::trace::{GSample, Rng, Shading, Tracer};
use bevy::math::Vec3;

/// Most see-through (alpha < 1) surfaces a path may pass before it ends.
const MAX_PASSES: u32 = 32;

/// One camera path: premultiplied radiance (also split by LightMix group),
/// coverage and the primary hit.
pub struct PathSample {
    pub radiance: Vec3,
    pub groups: Groups,
    pub alpha: f32,
    pub g: Option<GSample>,
    /// AOV split: light gathered at the camera hit (emission + direct).
    pub direct: Vec3,
    /// First bounce: 0 none, 1 diffuse, 2 glossy, 3 transmission / SSS.
    pub lobe: u8,
    /// Camera-hit albedo / roughness (denoiser demodulation, Albedo AOV).
    pub albedo: Vec3,
    pub rough: f32,
    /// One cavity / AO ray from the camera hit (1 = open).
    pub ao: f32,
}

#[derive(Default)]
struct Depths {
    total: u32,
    diffuse: u32,
    glossy: u32,
    transmission: u32,
}

impl Depths {
    /// Counts a bounce; false once any limit is exceeded.
    fn bump(&mut self, lobe: Lobe, ps: &PathSettings) -> bool {
        self.total += 1;
        let (n, max) = match lobe {
            Lobe::Diffuse => (&mut self.diffuse, ps.diffuse_bounces),
            Lobe::Glossy => (&mut self.glossy, ps.glossy_bounces),
            Lobe::Transmission => (&mut self.transmission, ps.transmission_bounces),
        };
        *n += 1;
        self.total <= ps.max_bounces.clamp(1, 8) && *n <= max.max(1)
    }
}

pub struct PathTracer<'a> {
    pub t: Tracer<'a>,
    pub ps: &'a PathSettings,
}

impl<'a> PathTracer<'a> {
    pub fn new(t: Tracer<'a>, ps: &'a PathSettings) -> Self {
        Self { t, ps }
    }

    fn offset(&self, sh: &Shading, dir: Vec3) -> Vec3 {
        sh.p + sh.ng * if dir.dot(sh.ng) >= 0.0 { self.t.eps } else { -self.t.eps }
    }

    /// Traces one path from the camera.
    pub fn radiance(&self, o: Vec3, d: Vec3, rng: &mut Rng) -> PathSample {
        self.radiance_from(o, d, false, rng)
    }

    /// Traces a camera-like path; `nested` for a shadow catcher's
    /// reflection ray (catchers it meets then skip their own reflection).
    pub(super) fn radiance_from(&self, mut o: Vec3, mut d: Vec3, nested: bool, rng: &mut Rng) -> PathSample {
        let (rs, ps) = (self.t.rs, self.ps);
        let (mut l, mut beta): (Groups, Vec3) = (Default::default(), Vec3::ONE);
        let sky = if rs.procedural.is_some() { SUN_SKY } else { ENV };
        let (mut alpha, mut g, mut camera) = (0.0, None, true);
        let (mut prev_pdf, mut prev_delta, mut passes) = (0.0f32, true, 0);
        let mut depth = Depths::default();
        // Dispersion: the color channel this path follows once split.
        let mut channel = None;
        let (mut direct, mut lobe, mut albedo, mut rough, mut ao) = (None, 0u8, Vec3::ZERO, 0.0, 1.0);
        loop {
            let hit = rs.bvh.closest(o, d, f32::INFINITY);
            if let Some(m) = &rs.volume {
                let v = self.t.march(m, o, d, hit.as_ref().map_or(f32::INFINITY, |h| h.t), rng);
                l[VOLUME as usize] += beta * v.inscatter;
                beta *= v.transmittance;
            }
            if camera && let Some((le, gi)) = self.t.area_seen(o, d, hit.as_ref().map_or(f32::INFINITY, |h| h.t)) {
                (l[gi as usize], alpha) = (l[gi as usize] + beta * le, 1.0);
                break;
            }
            let Some(h) = hit else {
                if !camera {
                    l[sky as usize] += beta * self.t.environment(d);
                }
                break;
            };
            let tri = &rs.bvh.tris[h.tri];
            let m = &rs.materials[tri.mat];
            let mut sh = self.t.interp(&h, o, d);
            // Stochastic opacity: pass straight through with probability 1 - alpha.
            if sh.s.alpha < 0.999 && rng.f() >= sh.s.alpha && passes < MAX_PASSES {
                passes += 1;
                o = sh.p + d * self.t.eps;
                continue;
            }
            if camera {
                (alpha, g) = (1.0, Some(GSample { obj: tri.obj, mat: tri.mat as u32, depth: h.t, normal: sh.n }));
                sh.s = camera_view(m, &sh.s);
                (albedo, rough) = (sh.s.base, sh.s.roughness);
                let reach = self.t.cfg.gi.ao_radius.max(1e-3);
                ao = if rs.bvh.occluded(sh.p + sh.ng * self.t.eps, crate::render::trace::cosine_dir(sh.n, rng), reach) { 0.0 } else { 1.0 };
                if m.shadow_catcher {
                    let c = self.catch(&sh, d, ao, nested, rng);
                    l.iter_mut().zip(c.groups).for_each(|(a, b)| *a += b);
                    alpha = c.alpha;
                    break;
                }
            }
            if let Some((col, density)) = rs.fog {
                let f = (-density * h.t).exp();
                l[ENV as usize] += beta * col * (1.0 - f);
                beta *= f;
            }
            if !sh.entering && m.model == ShadingModel::Crystal {
                let att = lin3(m.attenuation_color).max(Vec3::splat(1e-3));
                let k = h.t / m.thickness.max(1e-3);
                beta *= Vec3::new(att.x.powf(k), att.y.powf(k), att.z.powf(k));
            }
            if !sh.entering && sh.s.transmission > 0.0 {
                beta *= super::subsurface::absorption(&m.principled, h.t);
            }
            let v = -d;
            if sh.s.emissive.max_element() > 0.0 {
                l[EMISSIVE as usize] += beta * sh.s.emissive * self.emission_weight(&sh, h.t, d, prev_pdf, prev_delta);
            }
            match m.model {
                ShadingModel::Unlit => {
                    l[EMISSIVE as usize] += beta * sh.s.base;
                    break;
                }
                ShadingModel::Toon => {
                    let mut s = sh.s;
                    s.emissive = Vec3::ZERO;
                    l[ENV as usize] += beta * shade(m, &s, sh.n, v, &self.t.direct(&sh, rng), rs.ambient);
                    break;
                }
                _ => {}
            }
            camera = false;
            let mut bsdf = Bsdf::new(&sh.s, sh.n, sh.entering).principled(&m.principled, sh.s.roughness);
            bsdf.channel = channel;
            self.next_event(&sh, &bsdf, v, beta, &mut l, rng);
            let first = direct.is_none();
            if first {
                direct = Some(l.iter().sum::<Vec3>());
            }
            let Some(mut bs) = bsdf.sample(v, rng) else { break };
            if first {
                lobe = if bs.sss { 3 } else { 1 + bs.lobe as u8 };
            }
            if !depth.bump(bs.lobe, ps) {
                break;
            }
            beta *= bs.weight;
            channel = channel.or(bs.channel);
            if bs.sss {
                // Random walk inside, then a white Lambert exit with NEE.
                let Some((ex, w)) = self.random_walk(m, &sh, rng) else { break };
                beta *= w;
                let exit = Bsdf::lambert(ex.n);
                self.next_event(&ex, &exit, ex.n, beta, &mut l, rng);
                let Some(out) = exit.sample(ex.n, rng) else { break };
                (sh, bs) = (ex, out);
            }
            if depth.total > ps.rr_start {
                let survive = beta.max_element().clamp(0.0, 0.95);
                if rng.f() >= survive {
                    break;
                }
                beta /= survive;
            }
            if beta.max_element() <= 0.0 {
                break;
            }
            (prev_pdf, prev_delta) = (bs.pdf, bs.delta);
            o = self.offset(&sh, bs.dir);
            d = bs.dir;
        }
        let radiance = l.iter().sum();
        PathSample { radiance, groups: l, alpha, g, direct: direct.unwrap_or(radiance), lobe, albedo, rough, ao }
    }

    /// MIS weight of emission found by BSDF sampling.
    fn emission_weight(&self, sh: &Shading, dist: f32, d: Vec3, prev_pdf: f32, prev_delta: bool) -> f32 {
        let em = &self.t.rs.emitters;
        if prev_delta || em.is_empty() || self.t.cfg.gi.emitter_samples == 0 {
            return 1.0;
        }
        let Some((e, p)) = em.lookup(sh.tri) else { return 1.0 };
        let light_pdf = area_to_solid_angle(p / e.area.max(1e-12), dist * dist, sh.ng.dot(d));
        power_heuristic(prev_pdf, light_pdf)
    }

    /// Direct light into `l` (each light's own group), weighted by `beta`:
    /// punctual / area lights (no MIS) plus one MIS-weighted sample on an
    /// emissive polygon.
    fn next_event(&self, sh: &Shading, bsdf: &Bsdf, v: Vec3, beta: Vec3, l: &mut Groups, rng: &mut Rng) {
        self.t.punctual_each(sh, rng, |g, li| l[g as usize] += beta * bsdf.eval(v, li.dir).0 * li.radiance);
        let em = &self.t.rs.emitters;
        if self.t.cfg.gi.emitter_samples == 0 {
            return;
        }
        let Some((e, prob)) = em.pick(rng) else { return };
        if e.tri == sh.tri {
            return;
        }
        let t = &self.t.rs.bvh.tris[e.tri];
        let q = point_on(&t.p, rng);
        let o = sh.p + sh.ng * self.t.eps;
        let to = q - o;
        let (d2, dir) = (to.length_squared(), to.normalize_or_zero());
        let nl = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
        let light_pdf = area_to_solid_angle(prob / e.area.max(1e-12), d2, nl.dot(dir));
        let (f, bsdf_pdf) = bsdf.eval(v, dir);
        if !light_pdf.is_finite() || f.max_element() <= 0.0 {
            return;
        }
        let vis = if self.t.cfg.shadows { self.t.transmittance(o, dir, d2.sqrt() - self.t.eps * 4.0) } else { 1.0 };
        if vis > 0.0 {
            l[EMISSIVE as usize] += beta * f * e.radiance * vis * power_heuristic(light_pdf, bsdf_pdf) / light_pdf;
        }
    }
}
