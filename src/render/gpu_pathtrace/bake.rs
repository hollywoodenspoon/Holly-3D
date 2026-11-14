//! Bakes a material's maps over a UV grid for the GPU path tracer: base
//! colour + alpha, emission + metallic, roughness + tangent-space normal,
//! twelve halves per texel (six `u32`). The kernel looks the grid up
//! bilinearly per hit, so every map the CPU tracer samples by UV renders
//! on the GPU. Maps that change with the 3D position (not the UVs) cannot
//! be baked this way and keep the scene on the CPU ([`why_not`]).

use crate::materials::Material;
use crate::materials::maps::MapNode;
use crate::materials::relief;
use crate::materials::shading::{base_surface, surface};
use bevy::math::{Vec2, Vec3, Vec4};
use std::sync::{Arc, Mutex};

/// `u32` words per texel.
pub const WORDS: usize = 6;
/// Grid size for procedural maps (bitmaps bake at their own size).
pub const PROC_RES: u32 = 1024;
pub const MAX_RES: u32 = 4096;
/// Most texture memory a scene may use on the GPU (bytes).
pub const CAP_BYTES: u64 = 1536 << 20;

/// Where a baked grid sits in UV space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    pub lo: Vec2,
    pub span: Vec2,
    /// The maps repeat every 1 UV: the lookup wraps instead of clamping.
    pub wrap: bool,
}

impl Space {
    pub const UNIT: Space = Space { lo: Vec2::ZERO, span: Vec2::ONE, wrap: true };
}

/// One material's baked grid.
pub struct Baked {
    pub w: u32,
    pub h: u32,
    pub space: Space,
    /// The material tilts normals (a normal / bump map or procedural relief).
    pub normal: bool,
    pub words: Arc<Vec<u32>>,
}

/// Everything the kernel reads at one UV: base, alpha, emission,
/// metallic, roughness, tangent normal.
fn sample(m: &Material, uv: Vec2, p: Vec3) -> ([f32; 12], bool) {
    let (s, t) = if m.projection.on() {
        (base_surface(m, uv, p, Vec4::ONE), relief::base_tangent_normal(m, uv, p, Vec3::X, Vec3::Y))
    } else {
        (surface(m, uv, p, Vec4::ONE), relief::tangent_normal(m, uv, p, Vec3::X, Vec3::Y))
    };
    let n = t.unwrap_or(Vec3::Z);
    let v = [s.base.x, s.base.y, s.base.z, s.alpha, s.emissive.x, s.emissive.y, s.emissive.z, s.metallic, s.roughness, n.x, n.y, n.z];
    (v, t.is_some())
}

fn close(a: &[f32; 12], b: &[f32; 12], tol: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol * (1.0 + x.abs().max(y.abs())))
}

/// Probe UVs: a fixed scatter across one repeat.
fn probes() -> impl Iterator<Item = Vec2> {
    (0..16).map(|i| {
        let f = i as f32;
        Vec2::new((f * 0.618_034 + 0.13).fract(), (f * 0.414_214 + 0.37).fract())
    })
}

/// The maps depend on the 3D position, not only on the UVs.
pub fn position_based(m: &Material) -> bool {
    let pts = [Vec3::ZERO, Vec3::new(3.7, -1.2, 5.1), Vec3::new(-8.3, 2.9, -0.7)];
    probes().any(|uv| {
        let a = sample(m, uv, pts[0]).0;
        pts[1..].iter().any(|p| !close(&a, &sample(m, uv, *p).0, 1e-4))
    })
}

/// The maps repeat every 1 UV in both directions.
pub fn periodic(m: &Material) -> bool {
    probes().all(|uv| {
        let a = sample(m, uv, Vec3::ZERO).0;
        [Vec2::X, Vec2::Y].iter().all(|d| close(&a, &sample(m, uv + *d, Vec3::ZERO).0, 1e-3))
    })
}

/// Why this (mapped) material cannot be baked for the GPU, if it cannot.
pub fn why_not(m: &Material) -> Option<&'static str> {
    if m.projection.on() && m.paint.is_some() {
        return Some("paint layers on World / Triplanar materials");
    }
    if position_based(m) {
        return Some("3D (position-based) procedural maps");
    }
    if m.projection.on() && !periodic(m) {
        return Some("non-repeating maps on World / Triplanar materials");
    }
    None
}

/// Grid size: the largest bitmap's own size, or [`PROC_RES`] when any
/// map is procedural (paint layers, patterns, node graphs, non-bitmap maps).
pub fn resolution(m: &Material) -> (u32, u32) {
    let mut proc = m.proc.pattern != crate::materials::proc_engine::Pattern::None || m.paint.is_some() || !m.slate_nodes.is_empty() || m.brush.enabled;
    let (mut w, mut h) = (0, 0);
    for s in m.maps.iter().filter(|s| s.enabled) {
        match &s.node {
            Some(MapNode::Bitmap { tex: Some(t), .. }) => (w, h) = (w.max(t.tex.width), h.max(t.tex.height)),
            Some(MapNode::Bitmap { tex: None, .. }) | None => {}
            Some(_) => proc = true,
        }
    }
    if proc {
        (w, h) = (w.max(PROC_RES), h.max(PROC_RES));
    }
    (w.clamp(16, MAX_RES), h.clamp(16, MAX_RES))
}

/// Bytes [`bake`] uses for this material.
pub fn bytes(m: &Material) -> u64 {
    let (w, h) = resolution(m);
    u64::from(w) * u64::from(h) * WORDS as u64 * 4
}

fn half2(a: f32, b: f32) -> u32 {
    u32::from(half::f16::from_f32(a).to_bits()) | u32::from(half::f16::from_f32(b).to_bits()) << 16
}

type Entry = (Material, Space, (u32, u32), Arc<Vec<u32>>, bool);
static CACHE: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
const CACHE_LEN: usize = 24;

/// Bakes `m` over `space` (cached: the same material and space bake once).
pub fn bake(m: &Material, space: Space) -> Baked {
    let (w, h) = resolution(m);
    if let Some(e) = CACHE.lock().unwrap().iter().find(|e| e.1 == space && e.2 == (w, h) && e.0 == *m) {
        return Baked { w, h, space, normal: e.4, words: e.3.clone() };
    }
    let mut words = vec![0u32; w as usize * h as usize * WORDS];
    let row = w as usize * WORDS;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let rows_per = (h as usize).div_ceil(threads).max(1);
    let normal = std::thread::scope(|sc| {
        let jobs: Vec<_> = words
            .chunks_mut(row * rows_per)
            .enumerate()
            .map(|(c, chunk)| {
                sc.spawn(move || {
                    let mut tilt = false;
                    for (r, line) in chunk.chunks_mut(row).enumerate() {
                        let y = c * rows_per + r;
                        let v = space.lo.y + (y as f32 + 0.5) / h as f32 * space.span.y;
                        for (x, px) in line.chunks_mut(WORDS).enumerate() {
                            let u = space.lo.x + (x as f32 + 0.5) / w as f32 * space.span.x;
                            let (s, t) = sample(m, Vec2::new(u, v), Vec3::ZERO);
                            tilt |= t;
                            for (k, o) in px.iter_mut().enumerate() {
                                *o = half2(s[2 * k], s[2 * k + 1]);
                            }
                        }
                    }
                    tilt
                })
            })
            .collect();
        jobs.into_iter().fold(false, |a, j| a | j.join().unwrap_or(false))
    });
    let words = Arc::new(words);
    let mut c = CACHE.lock().unwrap();
    if c.len() >= CACHE_LEN {
        c.remove(0);
    }
    c.push((m.clone(), space, (w, h), words.clone(), normal));
    Baked { w, h, space, normal, words }
}
