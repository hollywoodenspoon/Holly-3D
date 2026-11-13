//! wgpu compute port of the SolidCraft GPU path tracer (MIT / Apache-2.0):
//! the scene's triangles, binary BVH, lights and emitter table go up as
//! storage buffers and a WGSL kernel (`wgsl/`, joined by [`source`]) traces progressive
//! passes into an accumulation buffer; the host reads it back after each
//! pass for the Frame Window and adaptive sampling. With no adapter (headless
//! CI, no driver) or for features only the CPU has, rendering falls back to
//! the CPU path tracer.

pub mod features;
pub mod pack;

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
        include_str!("wgsl/env.wgsl"),
        include_str!("wgsl/lights.wgsl"),
        include_str!("wgsl/volume.wgsl"),
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
    let dst = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut enc = g.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(src, 0, &dst, 0, size);
    g.queue.submit([enc.finish()]);
    let slice = dst.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| format!("GPU poll: {e}"))?;
    let bytes = slice.get_mapped_range().map_err(|e| format!("GPU map: {e:?}"))?;
    Ok(bytes.as_chunks::<16>().0.iter().map(|c| Vec4::from_array(std::array::from_fn(|i| f32::from_le_bytes([c[4 * i], c[4 * i + 1], c[4 * i + 2], c[4 * i + 3]])))).collect())
}

fn params(rs: &RenderScene, cfg: &RenderSettings, region: Region, spp: u32, seed: u32, p: &pack::Packed) -> Vec<u8> {
    let ps = &cfg.path;
    let inv = rs.camera.view_proj(cfg.width as f32 / cfg.height as f32).inverse();
    let mut b = pack::Buf::default();
    inv.to_cols_array().chunks(4).for_each(|c| b.v4(Vec4::from_slice(c)));
    let u4 = |b: &mut pack::Buf, v: [u32; 4]| v.iter().for_each(|x| b.0.extend_from_slice(&x.to_le_bytes()));
    u4(&mut b, [cfg.width, cfg.height, region[2] - region[0], region[3] - region[1]]);
    u4(&mut b, [region[0], region[1], spp, seed]);
    u4(&mut b, [ps.max_bounces.clamp(1, 8), ps.diffuse_bounces, ps.glossy_bounces, ps.transmission_bounces]);
    let flags = u32::from(rs.camera.is_ortho()) | u32::from(cfg.shadows) << 1 | u32::from(cfg.gi.softbox) << 2 | u32::from(cfg.gi.emitter_samples > 0) << 3;
    u4(&mut b, [p.n_lights, p.n_emitters, ps.rr_start, flags]);
    b.v3(rs.background, Tracer::new(rs, cfg).eps);
    b.v3(rs.sky, 0.0);
    b.v3(rs.ambient, 0.0);
    let (fc, fd) = rs.fog.unwrap_or_default();
    b.v3(fc, fd);
    b.0
}

/// Renders `region` on the GPU with progressive passes like the CPU tracer.
pub fn render(
    g: &Gpu,
    rs: &RenderScene,
    cfg: &RenderSettings,
    region: Region,
    cancel: &AtomicBool,
    mut on_pass: impl FnMut(PassInfo, &[Pixel]),
) -> Result<Vec<Pixel>, String> {
    let ps = &cfg.path;
    let [x0, y0, x1, y1] = region;
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let n = rw * rh;
    let p = pack::pack(rs);
    let d = &g.device;
    let size = (n * 16) as u64;
    let accum = storage(d, "accum", &vec![0; n * 16], true);
    let stats = storage(d, "stats", &vec![0; n * 16], true);
    let done_buf = storage(d, "done", &vec![0; n * 4], false);
    let uniform = d.create_buffer(&wgpu::BufferDescriptor { label: Some("params"), size: 64 + 4 * 16 + 4 * 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let bufs = [storage(d, "tris", &p.tris, false), storage(d, "nodes", &p.nodes, false), storage(d, "lights", &p.lights, false), storage(d, "emitters", &p.emitters, false)];
    let pipeline = g.pipeline(Features::of(rs));
    let layout = pipeline.get_bind_group_layout(0);
    let entries: Vec<_> = [&uniform, &bufs[0], &bufs[1], &bufs[2], &bufs[3], &accum, &stats, &done_buf]
        .iter()
        .enumerate()
        .map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() })
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
            g.queue.write_buffer(&uniform, 0, &params(rs, cfg, region, spp, seed, &p));
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
                return Ok(out);
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
    Ok(out)
}

/// Background and G-buffer of a pixel's center ray, traced on the CPU BVH
/// (the outline and denoiser passes need them; the GPU sends only color).
fn primary(t: &Tracer, cfg: &RenderSettings, x: u32, y: u32) -> Accum {
    let ndc = Vec2::new((x as f32 + 0.5) / cfg.width as f32 * 2.0 - 1.0, 1.0 - (y as f32 + 0.5) / cfg.height as f32 * 2.0);
    let ray = t.rs.camera.ray(ndc, cfg.width as f32 / cfg.height as f32);
    let g = t.rs.bvh.closest(ray.origin, ray.dir, f32::INFINITY).map_or(GSample::EMPTY, |h| {
        let tri = &t.rs.bvh.tris[h.tri];
        GSample { obj: tri.obj, mat: tri.mat as u32, depth: h.t, normal: t.interp(&h, ray.origin, ray.dir).n }
    });
    Accum { sum: Vec4::ZERO, back: t.backdrop(ray.dir), g, n: 0, stats: Welford::ZERO, ..Accum::EMPTY }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_features;
