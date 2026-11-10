//! Built-in high-quality CPU raytracer: soft shadows, skylight occlusion,
//! glossy reflections, crystal refraction with absorption, opacity, fog and
//! stratified anti-aliasing. Produces linear radiance + a G-buffer per pixel.

use super::bvh::Hit;
use super::scene_build::{RShape, RenderScene};
use super::settings::RenderSettings;
use crate::materials::ShadingModel;
use crate::materials::shading::{LightIn, Surface, fresnel_dielectric, fresnel_schlick, lin3, refract, shade, surface, tonemap};
use bevy::math::{Vec2, Vec3, Vec4};

/// Small deterministic xorshift RNG (one per pixel).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Uniform point in the unit ball.
    pub fn ball(&mut self) -> Vec3 {
        loop {
            let p = Vec3::new(self.f(), self.f(), self.f()) * 2.0 - 1.0;
            if p.length_squared() <= 1.0 {
                return p;
            }
        }
    }
}

/// Cosine-weighted direction around `n`.
fn cosine_dir(n: Vec3, rng: &mut Rng) -> Vec3 {
    let (a, b) = n.any_orthonormal_pair();
    let (r, phi) = (rng.f().sqrt(), rng.f() * std::f32::consts::TAU);
    (a * r * phi.cos() + b * r * phi.sin() + n * (1.0 - r * r).max(0.0).sqrt()).normalize()
}

fn reflect(d: Vec3, n: Vec3) -> Vec3 {
    d - n * 2.0 * d.dot(n)
}

/// Per-pixel G-buffer sample used by the outline pass.
#[derive(Clone, Copy, Debug)]
pub struct GSample {
    /// Object index (`u32::MAX` = background).
    pub obj: u32,
    /// Material table index.
    pub mat: u32,
    pub depth: f32,
    pub normal: Vec3,
}

impl GSample {
    pub const EMPTY: GSample = GSample { obj: u32::MAX, mat: u32::MAX, depth: f32::INFINITY, normal: Vec3::ZERO };
}

/// One rendered pixel: display sRGB + alpha, linear HDR and G-buffer.
#[derive(Clone, Copy, Debug)]
pub struct Pixel {
    pub display: Vec4,
    pub hdr: Vec4,
    pub g: GSample,
}

pub struct Tracer<'a> {
    pub rs: &'a RenderScene,
    pub cfg: &'a RenderSettings,
    eps: f32,
}

struct Shading {
    p: Vec3,
    /// Shading normal facing the incoming ray.
    n: Vec3,
    /// Geometric normal facing the incoming ray.
    ng: Vec3,
    /// True when the ray enters the surface (hits its front face).
    entering: bool,
    s: Surface,
}

impl<'a> Tracer<'a> {
    pub fn new(rs: &'a RenderScene, cfg: &'a RenderSettings) -> Self {
        Self { rs, cfg, eps: (rs.radius * 2e-4).max(1e-4) }
    }

    fn interp(&self, h: &Hit, o: Vec3, d: Vec3) -> Shading {
        let t = &self.rs.bvh.tris[h.tri];
        let w = Vec3::new(1.0 - h.u - h.v, h.u, h.v);
        let bary3 = |a: [Vec3; 3]| a[0] * w.x + a[1] * w.y + a[2] * w.z;
        let p = o + d * h.t;
        let geo = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or(Vec3::Z);
        let entering = geo.dot(d) < 0.0;
        let ng = if entering { geo } else { -geo };
        let mut n = bary3(t.n).normalize_or(ng);
        if n.dot(ng) < 0.0 {
            n = -n;
        }
        let uv: Vec2 = t.uv[0] * w.x + t.uv[1] * w.y + t.uv[2] * w.z;
        let col: Vec4 = t.col[0] * w.x + t.col[1] * w.y + t.col[2] * w.z;
        let s = surface(&self.rs.materials[t.mat], uv, p, col);
        Shading { p, n, ng, entering, s }
    }

    /// Fraction of the light reaching `p` (0..1), with soft jitter.
    fn visibility(&self, p: Vec3, to_light: Vec3, dist: f32, soft: f32, rng: &mut Rng) -> f32 {
        let n = if soft > 0.0 { self.cfg.shadow_samples.max(1) } else { 1 };
        let mut lit = 0;
        for _ in 0..n {
            let (dir, len) = if soft <= 0.0 {
                (to_light, dist)
            } else if dist.is_finite() {
                let target = p + to_light * dist + rng.ball() * soft;
                let v = target - p;
                (v.normalize(), v.length())
            } else {
                ((to_light + rng.ball() * soft * 0.05).normalize(), dist)
            };
            if !self.rs.bvh.occluded(p, dir, (len - self.eps).min(1e30)) {
                lit += 1;
            }
        }
        lit as f32 / n as f32
    }

    fn direct(&self, sh: &Shading, rng: &mut Rng) -> Vec<LightIn> {
        let o = sh.p + sh.ng * self.eps;
        let mut out = Vec::with_capacity(self.rs.lights.len());
        for l in &self.rs.lights {
            let (dir, dist, mut k) = match l.shape {
                RShape::Directional { dir } => (-dir, f32::INFINITY, 1.0),
                RShape::Point | RShape::Spot { .. } => {
                    let v = l.pos - sh.p;
                    let d = v.length().max(1e-4);
                    let window = (1.0 - (d / l.range.max(1e-3)).powi(4)).clamp(0.0, 1.0).powi(2);
                    (v / d, d, window * l.atten.factor(d) / (d * d))
                }
            };
            if let RShape::Spot { dir: axis, cos_in, cos_out } = l.shape {
                let c = (-dir).dot(axis);
                let t = ((c - cos_out) / (cos_in - cos_out).max(1e-4)).clamp(0.0, 1.0);
                k *= t * t * (3.0 - 2.0 * t);
            }
            if k <= 0.0 || sh.n.dot(dir) <= 0.0 {
                continue;
            }
            let mut radiance = l.radiance * k;
            if self.cfg.shadows && l.shadows {
                let vis = self.visibility(o, dir, dist, l.softness, rng);
                radiance *= l.shadow_tint + (Vec3::ONE - l.shadow_tint) * vis;
            }
            out.push(LightIn { dir, radiance });
        }
        out
    }

    /// Skylight with hemisphere occlusion.
    fn sky(&self, sh: &Shading, rng: &mut Rng) -> Vec3 {
        if self.rs.sky.max_element() <= 0.0 {
            return Vec3::ZERO;
        }
        let n = self.cfg.sky_samples.max(1);
        let o = sh.p + sh.ng * self.eps;
        let open = (0..n).filter(|_| !self.rs.bvh.occluded(o, cosine_dir(sh.n, rng), self.rs.radius)).count();
        self.rs.sky * (open as f32 / n as f32)
    }

    /// Radiance + coverage along a ray.
    pub fn trace(&self, o: Vec3, d: Vec3, depth: u32, rng: &mut Rng) -> (Vec3, f32, Option<GSample>) {
        let Some(h) = self.rs.bvh.closest(o, d, f32::INFINITY) else {
            return (if depth == 0 { Vec3::ZERO } else { self.environment(d) }, 0.0, None);
        };
        let tri = &self.rs.bvh.tris[h.tri];
        let m = &self.rs.materials[tri.mat];
        let sh = self.interp(&h, o, d);
        let g = GSample { obj: tri.obj, mat: tri.mat as u32, depth: h.t, normal: sh.n };
        let v = -d;
        let deeper = depth < self.cfg.max_depth;
        // Opacity: blend with whatever is behind.
        if sh.s.alpha < 0.999 && deeper {
            let (behind, ba, _) = self.trace(sh.p + d * self.eps, d, depth + 1, rng);
            // Premultiplied: a miss behind a see-through surface adds nothing.
            let behind = if ba > 0.0 { behind } else { Vec3::ZERO };
            let front = self.surface_color(m, &sh, v, d, depth, rng);
            let a = sh.s.alpha;
            return (self.fog(front * a + behind * (1.0 - a), h.t), a + (1.0 - a) * ba, Some(g));
        }
        let mut c = self.surface_color(m, &sh, v, d, depth, rng);
        // Leaving a crystal: Beer-Lambert absorption over the inside path.
        if !sh.entering && m.model == ShadingModel::Crystal {
            let att = lin3(m.attenuation_color).max(Vec3::splat(1e-3));
            let k = h.t / m.thickness.max(1e-3);
            c *= Vec3::new(att.x.powf(k), att.y.powf(k), att.z.powf(k));
        }
        (self.fog(c, h.t), 1.0, Some(g))
    }

    /// What a reflected / refracted ray leaving the scene sees: the background
    /// plus a soft studio dome (brighter overhead). Camera rays that miss
    /// return zero coverage and show the plain background.
    fn environment(&self, d: Vec3) -> Vec3 {
        if let Some(c) = self.rs.hdri.as_ref().and_then(|h| h.radiance(d)) {
            return c + self.rs.sky;
        }
        let up = (0.5 + 0.5 * d.z).clamp(0.0, 1.0);
        self.rs.background + self.rs.sky + self.rs.ambient * (1.0 + 5.0 * up * up)
    }

    /// What a camera ray that misses shows: the HDRI, else the plain color.
    fn backdrop(&self, d: Vec3) -> Vec3 {
        self.rs.hdri.as_ref().and_then(|h| h.radiance(d)).unwrap_or(self.rs.background)
    }

    fn fog(&self, c: Vec3, t: f32) -> Vec3 {
        match self.rs.fog {
            Some((col, density)) => {
                let f = (-density * t).exp();
                c * f + col * (1.0 - f)
            }
            None => c,
        }
    }

    fn surface_color(&self, m: &crate::materials::Material, sh: &Shading, v: Vec3, d: Vec3, depth: u32, rng: &mut Rng) -> Vec3 {
        let s = &sh.s;
        if m.model == ShadingModel::Unlit {
            return s.base + s.emissive;
        }
        let lights = self.direct(sh, rng);
        let ambient = self.rs.ambient + self.sky(sh, rng);
        let mut c = shade(m, s, sh.n, v, &lights, ambient);
        if m.model == ShadingModel::Toon {
            return c;
        }
        let deeper = depth < self.cfg.max_depth;
        let cos = sh.n.dot(v).clamp(0.0, 1.0);
        let gloss = (1.0 - s.roughness).powi(2);
        // Crystal: Fresnel-weighted reflection + refraction.
        if s.transmission > 0.0 {
            let eta = if sh.entering { 1.0 / s.ior } else { s.ior };
            let refr_dir = refract(d, sh.n, eta);
            let fr = if refr_dir.is_some() { fresnel_dielectric(cos, eta) } else { 1.0 };
            let mut glass = Vec3::ZERO;
            if deeper {
                let r = (reflect(d, sh.n) + rng.ball() * s.roughness * s.roughness).normalize();
                glass += self.trace(sh.p + sh.ng * self.eps, r, depth + 1, rng).0 * fr;
                if let Some(t) = refr_dir {
                    let t = (t + rng.ball() * s.roughness * s.roughness * 0.5).normalize();
                    let tint = if sh.entering { s.base } else { Vec3::ONE };
                    glass += self.trace(sh.p - sh.ng * self.eps, t, depth + 1, rng).0 * (1.0 - fr) * tint;
                }
            } else {
                glass = self.backdrop(d) * (1.0 - fr) + self.rs.ambient * fr;
            }
            return c * (1.0 - s.transmission) + glass * s.transmission;
        }
        // Glossy reflection of the scene / background.
        let f0 = Vec3::splat(0.04).lerp(s.base, s.metallic);
        let f = fresnel_schlick(cos, f0) * gloss;
        c += f0 * (1.0 - gloss) * ambient;
        if deeper && f.max_element() > 0.02 {
            let r = (reflect(d, sh.n) + rng.ball() * s.roughness * s.roughness).normalize();
            let r = if r.dot(sh.ng) < 0.0 { reflect(d, sh.n) } else { r };
            c += self.trace(sh.p + sh.ng * self.eps, r, depth + 1, rng).0 * f;
        }
        c
    }

    /// Renders pixel (x, y) of a `w` x `h` image.
    pub fn pixel(&self, x: u32, y: u32, w: u32, h: u32) -> Pixel {
        let n = self.cfg.aa.max(1);
        let aspect = w as f32 / h as f32;
        let mut rng = Rng::new(((y as u64) << 32) ^ x as u64 ^ 0xA5A5_5A5A);
        let (mut display, mut hdr) = (Vec4::ZERO, Vec4::ZERO);
        let mut g = GSample::EMPTY;
        let bg = tonemap_bg(self.rs.background);
        for sy in 0..n {
            for sx in 0..n {
                let jx = (sx as f32 + if n > 1 { rng.f() } else { 0.5 }) / n as f32;
                let jy = (sy as f32 + if n > 1 { rng.f() } else { 0.5 }) / n as f32;
                let ndc = Vec2::new((x as f32 + jx) / w as f32 * 2.0 - 1.0, 1.0 - (y as f32 + jy) / h as f32 * 2.0);
                let ray = self.rs.camera.ray(ndc, aspect);
                let (c, a, gs) = self.trace(ray.origin, ray.dir, 0, &mut rng);
                let back = self.backdrop(ray.dir);
                let bg = if self.rs.hdri.is_some() { tonemap(back) } else { bg };
                if sx == n / 2 && sy == n / 2 {
                    g = gs.unwrap_or(GSample::EMPTY);
                }
                // `c` is premultiplied by the coverage `a`.
                let shaded = if a > 0.0 { tonemap(c / a) * a } else { Vec3::ZERO };
                if self.cfg.transparent {
                    display += shaded.extend(a);
                    hdr += c.extend(a);
                } else {
                    display += (shaded + bg * (1.0 - a)).extend(1.0);
                    hdr += (c + back * (1.0 - a)).extend(1.0);
                }
            }
        }
        let k = 1.0 / (n * n) as f32;
        let mut display = display * k;
        if self.cfg.transparent && display.w > 1e-4 {
            // Straight (un-premultiplied) alpha for PNG/EXR output.
            display = (display.truncate() / display.w).extend(display.w);
        }
        Pixel { display, hdr: hdr * k, g }
    }
}

/// The background shows exactly the chosen Environment color.
fn tonemap_bg(c: Vec3) -> Vec3 {
    use crate::materials::shading::linear_to_srgb;
    Vec3::new(linear_to_srgb(c.x), linear_to_srgb(c.y), linear_to_srgb(c.z))
}
