//! wgpu compute port of the SolidCraft GPU path tracer (MIT / Apache-2.0):
//! the scene's triangles, binary BVH, lights and emitter table go up as
//! storage buffers and a WGSL kernel (`wgsl/`, joined by [`source`]) traces progressive
//! passes into an accumulation buffer; the host reads it back after each
//! pass for the Frame Window and adaptive sampling. With no adapter (headless
//! CI, no driver) or for features only the CPU has, rendering falls back to
//! the CPU path tracer.

pub mod bake;
pub mod envmap;
pub mod features;
pub mod live;
pub mod mix;
pub mod pack;
pub mod textures;
pub mod volobj;

use crate::render::job::Region;
use crate::render::pathtrace::adaptive::{Welford, converged};
use crate::render::pathtrace::progressive::{Accum, PassInfo, resolve};
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::trace::{GSample, Pixel, Tracer};
use bevy::math::{Vec2, Vec4};
use features::Features;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};
use wgpu::util::DeviceExt;

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    module: wgpu::ShaderModule,
    /// One pipeline per feature mask, built on first use.
    pipelines: Mutex<HashMap<u32, wgpu::ComputePipeline>>,
    pub name: String,
}

/// The kernel's WGSL: every file in `wgsl/`, joined (WGSL takes any
/// declaration order).
pub fn source() -> String {
    [
        include_str!("wgsl/common.wgsl"),
        include_str!("wgsl/bvh.wgsl"),
        include_str!("wgsl/bsdf.wgsl"),
        include_str!("wgsl/principled.wgsl"),
        include_str!("wgsl/env.wgsl"),
        include_str!("wgsl/lights.wgsl"),
        include_str!("wgsl/area.wgsl"),
        include_str!("wgsl/volume.wgsl"),
        include_str!("wgsl/volobj.wgsl"),
        include_str!("wgsl/textures.wgsl"),
        include_str!("wgsl/live.wgsl"),
        include_str!("wgsl/opacity.wgsl"),
        include_str!("wgsl/toon.wgsl"),
        include_str!("wgsl/catcher.wgsl"),
        include_str!("wgsl/integrator.wgsl"),
    ]
    .join("\n")
}

impl Gpu {
    /// The pipeline for this feature mask (compiled once, then cached).
    pub fn pipeline(&self, f: Features) -> wgpu::ComputePipeline {
        let mut cache = self.pipelines.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        cache
            .entry(f.0)
            .or_insert_with(|| {
                let constants = f.constants();
                self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("path tracer"),
                    layout: None,
                    module: &self.module,
                    entry_point: Some("main"),
                    compilation_options: wgpu::PipelineCompilationOptions { constants: &constants, ..Default::default() },
                    cache: None,
                })
            })
            .clone()
    }
}

fn init() -> Result<Gpu, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let opts = wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() };
    let adapter = pollster::block_on(instance.request_adapter(&opts)).map_err(|e| format!("no GPU adapter: {e}"))?;
    let desc = wgpu::DeviceDescriptor { label: Some("path tracer"), required_limits: adapter.limits(), ..Default::default() };
    let (device, queue) = pollster::block_on(adapter.request_device(&desc)).map_err(|e| format!("GPU device: {e}"))?;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("path tracer"), source: wgpu::ShaderSource::Wgsl(source().into()) });
    let info = adapter.get_info();
    Ok(Gpu { device, queue, module, pipelines: Mutex::default(), name: device_label(&info.name, &format!("{:?}", info.backend), &info.driver) })
}

/// "GPU: card (backend, driver)", without the driver when it is unknown.
pub fn device_label(name: &str, backend: &str, driver: &str) -> String {
    match driver.trim() {
        "" => format!("GPU: {name} ({backend})"),
        d => format!("GPU: {name} ({backend}, {d})"),
    }
}

/// The shared GPU context, created on first use (`Err` = no usable adapter).
pub fn gpu() -> Result<&'static Gpu, String> {
    static GPU: OnceLock<Result<Gpu, String>> = OnceLock::new();
    GPU.get_or_init(init).as_ref().map_err(Clone::clone)
}

fn storage(d: &wgpu::Device, label: &str, bytes: &[u8], rw: bool) -> wgpu::Buffer {
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | if rw { wgpu::BufferUsages::COPY_SRC } else { wgpu::BufferUsages::empty() };
    d.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents: bytes, usage })
}

fn read_back(g: &Gpu, src: &wgpu::Buffer, size: u64) -> Result<Vec<Vec4>, String> {
    let f = read_floats(g, src, size)?;
    Ok(f.as_chunks::<4>().0.iter().map(|c| Vec4::from_array(*c)).collect())
}

/// `size` bytes of `src` as floats.
fn read_floats(g: &Gpu, src: &wgpu::Buffer, size: u64) -> Result<Vec<f32>, String> {
    let dst = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut enc = g.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(src, 0, &dst, 0, size);
    g.queue.submit([enc.finish()]);
    let slice = dst.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| format!("GPU poll: {e}"))?;
    let bytes = slice.get_mapped_range().map_err(|e| format!("GPU map: {e:?}"))?;
    Ok(bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

fn params(rs: &RenderScene, cfg: &RenderSettings, region: Region, spp: u32, seed: u32, data: (&pack::Packed, &envmap::EnvMap, &textures::Store)) -> Vec<u8> {
    let (p, env, tex) = data;
    let ps = &cfg.path;
    let inv = rs.camera.view_proj(cfg.width as f32 / cfg.height as f32).inverse();
    let mut b = pack::Buf::default();
    inv.to_cols_array().chunks(4).for_each(|c| b.v4(Vec4::from_slice(c)));
    let u4 = |b: &mut pack::Buf, v: [u32; 4]| v.iter().for_each(|x| b.0.extend_from_slice(&x.to_le_bytes()));
    u4(&mut b, [cfg.width, cfg.height, region[2] - region[0], region[3] - region[1]]);
    u4(&mut b, [region[0], region[1], spp, seed]);
    u4(&mut b, [ps.max_bounces.clamp(1, 8), ps.diffuse_bounces, ps.glossy_bounces, ps.transmission_bounces]);
    let flags = u32::from(rs.camera.is_ortho()) | u32::from(cfg.shadows) << 1 | u32::from(cfg.gi.softbox) << 2 | u32::from(cfg.gi.emitter_samples > 0) << 3 | u32::from(rs.procedural.is_some()) << 4;
    u4(&mut b, [p.n_lights, p.n_emitters, ps.rr_start, flags]);
    b.v3(rs.background, Tracer::new(rs, cfg).eps);
    // The catcher's contact AO: strength in sky.w, reach in ambient.w.
    let gi = &cfg.gi;
    b.v3(rs.sky, gi.ao_strength.clamp(0.0, 1.0) * f32::from(u8::from(gi.ao)));
    b.v3(rs.ambient, gi.ao_radius.max(1e-3));
    let (fc, fd) = rs.fog.unwrap_or_default();
    b.v3(fc, fd);
    b.v4(Vec4::new(env.w as f32, env.h as f32, env.turn, env.strength));
    u4(&mut b, [tex.table, tex.count, tex.records, tex.rec_count]);
    volume(&mut b, rs, mix::groups(rs, cfg), cfg.gi.emitter_samples);
    b.0
}

/// See-through surfaces a shadow ray passes before it counts as blocked
/// (`Tracer::transmittance`). A uniform, not a WGSL constant, so drivers
/// keep the loop instead of unrolling a BVH walk per hit at every call site.
const SHADOW_HITS: f32 = 8.0;

/// Bytes of the kernel's `Params` uniform.
const PARAMS_BYTES: u64 = 64 + 4 * 16 + 5 * 16 + 16 + 5 * 16;

/// The volume block of `Params` (`vol0..vol4`, see `wgsl/volume.wgsl`);
/// zeros when the scene has no mist. `vol4.y` carries the LightMix group
/// count, `vol4.z` the glowing-triangle picks toon materials take and
/// `vol4.w` the see-through hits a shadow ray passes ([`SHADOW_HITS`]).
fn volume(b: &mut pack::Buf, rs: &RenderScene, groups: usize, picks: u32) {
    let Some(m) = &rs.volume else {
        (0..4).for_each(|_| b.v4(Vec4::ZERO));
        b.v4(Vec4::new(0.0, groups as f32, picks as f32, SHADOW_HITS));
        return;
    };
    b.v3(m.albedo, m.density);
    b.v4(Vec4::new(m.base, m.falloff, m.wisps, m.inv_scale));
    b.v4(Vec4::new(m.wind.x, m.wind.y, m.g, m.intensity));
    b.v3(rs.camera.eye(), rs.radius * 6.0);
    b.v4(Vec4::new(m.steps as f32, groups as f32, picks as f32, SHADOW_HITS));
}

/// Renders `region` on the GPU with progressive passes like the CPU tracer.
pub fn render(
    g: &Gpu,
    rs: &RenderScene,
    cfg: &RenderSettings,
    region: Region,
    cancel: &AtomicBool,
    mut on_pass: impl FnMut(PassInfo, &[Pixel]),
) -> Result<(Vec<Pixel>, Option<crate::render::lightmix::Buffers>), String> {
    let ps = &cfg.path;
    let [x0, y0, x1, y1] = region;
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let n = rw * rh;
    let p = pack::pack(rs, cfg.shadow_samples);
    let env = envmap::env_map(rs, cfg.path.sky_bake).unwrap_or_else(envmap::EnvMap::none);
    let tex = textures::build(rs);
    let tex_bytes = tex.bytes();
    let limit = g.device.limits().max_storage_buffer_binding_size;
    if tex_bytes.len() as u64 > limit {
        return Err(format!("baked textures ({} MB) exceed this GPU's {} MB buffer limit", tex_bytes.len() >> 20, limit >> 20));
    }
    let d = &g.device;
    let size = (n * 16) as u64;
    let accum = storage(d, "accum", &vec![0; n * 16], true);
    let stats = storage(d, "stats", &vec![0; n * 16], true);
    let done_buf = storage(d, "done", &vec![0; n * 4], false);
    let k = mix::groups(rs, cfg);
    let group_buf = mix::buffer(d, n, k, limit)?;
    let vol_buf = storage(d, "volume objects", &volobj::pack(rs, cfg), false);
    let uniform = d.create_buffer(&wgpu::BufferDescriptor { label: Some("params"), size: PARAMS_BYTES, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let bufs = [storage(d, "tris", &p.tris, false), storage(d, "nodes", &p.nodes, false), storage(d, "lights", &p.lights, false), storage(d, "emitters", &p.emitters, false), storage(d, "envmap", &env.px, false), storage(d, "textures", &tex_bytes, false)];
    let pipeline = g.pipeline(Features(Features::of(rs).0 | if k > 0 { Features::LIGHTMIX } else { 0 }));
    let layout = pipeline.get_bind_group_layout(0);
    let entries: Vec<_> = [&uniform, &bufs[0], &bufs[1], &bufs[2], &bufs[3], &accum, &stats, &done_buf, &bufs[4], &bufs[5]]
        .iter()
        .enumerate()
        .map(|(i, b)| (i as u32, *b))
        .chain([(12, &group_buf), (13, &vol_buf)])
        .map(|(i, b)| wgpu::BindGroupEntry { binding: i, resource: b.as_entire_binding() })
        .collect();
    let bind = d.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("path tracer"), layout: &layout, entries: &entries });
    let (gx, gy) = (rw.div_ceil(8) as u32, rh.div_ceil(8) as u32);
    let tracer = Tracer::new(rs, cfg);
    let mut acc: Vec<Accum> = (0..n).map(|i| primary(&tracer, cfg, x0 + (i % rw) as u32, y0 + (i / rw) as u32)).collect();
    let mut out = vec![Pixel::EMPTY; n];
    let (mut have, mut seed, mut batch) = (0, 0u32, features::SPP_RANGE.0);
    for (pass, &target) in ps.schedule().iter().enumerate() {
        while have < target {
            let spp = (target - have).min(batch);
            let t0 = Instant::now();
            g.queue.write_buffer(&uniform, 0, &params(rs, cfg, region, spp, seed, (&p, &env, &tex)));
            let mut enc = d.create_command_encoder(&Default::default());
            {
                let mut cp = enc.begin_compute_pass(&Default::default());
                cp.set_pipeline(&pipeline);
                cp.set_bind_group(0, &bind, &[]);
                cp.dispatch_workgroups(gx, gy, 1);
            }
            g.queue.submit([enc.finish()]);
            // Wait for this batch so the next one is sized from its real time.
            d.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| format!("GPU poll: {e}"))?;
            if spp == batch {
                batch = features::next_spp(spp, t0.elapsed().as_secs_f64() * 1e3, features::TARGET_MS);
            }
            (have, seed) = (have + spp, seed + 1);
            if cancel.load(Ordering::Relaxed) {
                return Ok((out, None));
            }
        }
        let (sums, st) = (read_back(g, &accum, size)?, read_back(g, &stats, size)?);
        for ((a, s), t) in acc.iter_mut().zip(&sums).zip(&st) {
            (a.sum, a.n, a.stats) = (*s, t.z as u32, Welford::from_sums(t.z as u32, f64::from(t.x), f64::from(t.y)));
        }
        let done = if ps.adaptive { converged(&acc.iter().map(|a| a.stats).collect::<Vec<_>>(), rw, rh, ps.noise_threshold, ps.min_samples) } else { vec![false; n] };
        let mask: Vec<u8> = done.iter().flat_map(|&b| u32::from(b).to_le_bytes()).collect();
        g.queue.write_buffer(&done_buf, 0, &mask);
        for (o, a) in out.iter_mut().zip(&acc) {
            *o = resolve(a, cfg, rs.hdri.is_none());
        }
        let active = done.iter().filter(|d| !**d).count() as f32 / n.max(1) as f32;
        on_pass(PassInfo { pass, spp: target, active }, &out);
        if active == 0.0 {
            break;
        }
    }
    crate::render::postfx::run(&mut out, rw, rh, rs, cfg);
    let lightmix = if k > 0 { Some(mix::finish(g, &group_buf, rs, cfg, region, &acc, k)?) } else { None };
    Ok((out, lightmix))
}

/// Background and G-buffer of a pixel's center ray, traced on the CPU BVH
/// (the outline and denoiser passes need them; the GPU sends only color).
fn primary(t: &Tracer, cfg: &RenderSettings, x: u32, y: u32) -> Accum {
    let ndc = Vec2::new((x as f32 + 0.5) / cfg.width as f32 * 2.0 - 1.0, 1.0 - (y as f32 + 0.5) / cfg.height as f32 * 2.0);
    let ray = t.rs.camera.ray(ndc, cfg.width as f32 / cfg.height as f32);
    // As the CPU tracer's G-buffer: Cut-out holes are skipped and an area
    // light the camera sees in front of the surface leaves it empty.
    let (o, hit) = t.past_holes(ray.origin, ray.dir);
    let seen = t.area_seen(o, ray.dir, hit.as_ref().map_or(f32::INFINITY, |h| h.t)).is_some();
    let g = hit.filter(|_| !seen).map_or(GSample::EMPTY, |h| {
        let tri = &t.rs.bvh.tris[h.tri];
        GSample { obj: tri.obj, mat: tri.mat as u32, depth: o.distance(ray.origin) + h.t, normal: t.interp(&h, o, ray.dir).n }
    });
    Accum { sum: Vec4::ZERO, back: t.backdrop(ray.dir), g, n: 0, stats: Welford::ZERO, ..Accum::EMPTY }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_features;
#[cfg(test)]
mod tests_env;
#[cfg(test)]
mod tests_principled;
#[cfg(test)]
mod tests_lobes;
#[cfg(test)]
mod tests_tex;
#[cfg(test)]
mod tests_volume;
#[cfg(test)]
mod tests_area;
#[cfg(test)]
mod tests_live;
#[cfg(test)]
mod tests_mix;
#[cfg(test)]
mod tests_opacity;
#[cfg(test)]
mod tests_toon;
#[cfg(test)]
mod tests_catcher;
#[cfg(test)]
mod tests_volobj;
