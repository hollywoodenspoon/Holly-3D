//! The smoke solver on the GPU: the same substeps as `solver.rs`, run as
//! wgpu compute kernels (`gpu.wgsl`) on the shared path-tracer device. The
//! source emission comes from [`SimState::emit_list`], so both solvers
//! inject exactly the same gas; after each frame the fields are read back
//! into a [`SimState`] for the bake to store.

use super::solver::SWIRL_GAIN;
use super::{SimParams, SimState};
use crate::render::gpu_pathtrace::{Gpu, gpu, read_floats};
use std::sync::OnceLock;

/// The compiled kernels, built once per run.
struct Kernels {
    layout: wgpu::BindGroupLayout,
    pipes: Vec<(&'static str, u32, wgpu::ComputePipeline)>,
}

/// Entry points and the FIELD each is built with.
const ENTRIES: [(&str, u32); 19] = [
    ("emit", 0),
    ("forces", 0),
    ("curl_mag", 0),
    ("swirl", 0),
    ("burn", 0),
    ("divergence", 0),
    ("jacobi_pq", 0),
    ("jacobi_qp", 0),
    ("gradient", 1),
    ("gradient", 2),
    ("adv_fwd", 0),
    ("adv_fwd", 1),
    ("adv_fwd", 2),
    ("adv_rev", 0),
    ("adv_fin", 0),
    ("adv_fin", 1),
    ("adv_fin", 2),
    ("adv_vel", 0),
    ("fade", 0),
];

fn kernels(g: &Gpu) -> &'static Kernels {
    static K: OnceLock<Kernels> = OnceLock::new();
    K.get_or_init(|| {
        let d = &g.device;
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("smoke solver"), source: wgpu::ShaderSource::Wgsl(include_str!("gpu.wgsl").into()) });
        let buf = |binding: u32, uniform: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: if uniform { wgpu::BufferBindingType::Uniform } else { wgpu::BufferBindingType::Storage { read_only: false } },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let entries: Vec<_> = (0..8).map(|b| buf(b, b == 0)).collect();
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("smoke solver"), entries: &entries });
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("smoke solver"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let uses_field = |e: &str| matches!(e, "gradient" | "adv_fwd" | "adv_fin");
        let pipes = ENTRIES
            .iter()
            .chain(&[("max_speed", 0)])
            .map(|&(e, f)| {
                let constants = [("FIELD", f64::from(f))];
                let c: &[(&str, f64)] = if uses_field(e) { &constants } else { &[] };
                let p = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(e),
                    layout: Some(&pl),
                    module: &module,
                    entry_point: Some(e),
                    compilation_options: wgpu::PipelineCompilationOptions { constants: c, ..Default::default() },
                    cache: None,
                });
                (e, f, p)
            })
            .collect();
        Kernels { layout, pipes }
    })
}

impl Kernels {
    fn get(&self, e: &str, f: u32) -> &wgpu::ComputePipeline {
        &self.pipes.iter().find(|p| p.0 == e && p.1 == f).expect("a smoke kernel").2
    }
}

/// A simulation running on the GPU; `state` mirrors it after each frame.
pub struct GpuSim {
    pub state: SimState,
    g: &'static Gpu,
    k: &'static Kernels,
    uniform: wgpu::Buffer,
    vel: [wgpu::Buffer; 2],
    sca: [wgpu::Buffer; 2],
    tmp: wgpu::Buffer,
    emits: wgpu::Buffer,
    stats: wgpu::Buffer,
    bind: wgpu::BindGroup,
    cap: u64,
}

const PARAM_BYTES: u64 = 80;

fn buffer(d: &wgpu::Device, label: &str, bytes: u64, extra: wgpu::BufferUsages) -> wgpu::Buffer {
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC | extra;
    d.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: bytes.max(4), usage, mapped_at_creation: false })
}

impl GpuSim {
    pub fn new(p: &SimParams) -> Result<Self, String> {
        let g = gpu()?;
        let state = SimState::new(p);
        let cells = state.density.d.len() as u64;
        let bytes = cells * 4 * 3;
        let limit = g.device.limits().max_storage_buffer_binding_size;
        if cells * 4 * 5 > limit {
            return Err(format!("the simulation grid ({} MB) exceeds this GPU's {} MB buffer limit", (cells * 20) >> 20, limit >> 20));
        }
        let d = &g.device;
        let k = kernels(g);
        let uniform = d.create_buffer(&wgpu::BufferDescriptor { label: Some("smoke params"), size: PARAM_BYTES, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let none = wgpu::BufferUsages::empty();
        let vel = [buffer(d, "velocity", bytes, none), buffer(d, "velocity next", bytes, none)];
        let sca = [buffer(d, "scalars", bytes, none), buffer(d, "scalars next", bytes, none)];
        let tmp = buffer(d, "scratch", cells * 4 * 5, none);
        let stats = buffer(d, "stats", 16, none);
        let (h, n) = (state.h, state.n);
        // The source sphere's bounding box bounds the emission list.
        let r = (p.source_radius / h).max(1.0) as u64 * 2 + 4;
        let cap = r * r * r.min(n[2] as u64 + 1) * 32;
        let emits = buffer(d, "emission", cap, none);
        let bind = bind_group(g, k, &uniform, &vel, &sca, &tmp, &emits, &stats);
        let sim = Self { state, g, k, uniform, vel, sca, tmp, emits, stats, bind, cap };
        g.queue.write_buffer(&sim.uniform, 0, &sim.params(p, 0.0, 0));
        Ok(sim)
    }

    /// The largest velocity component (m/s), from a GPU reduction.
    fn max_speed(&self) -> Result<f32, String> {
        let mut enc = self.g.device.create_command_encoder(&Default::default());
        enc.clear_buffer(&self.stats, 0, None);
        self.dispatch_linear(&mut enc, "max_speed", self.state.density.d.len() as u32 * 3);
        self.g.queue.submit([enc.finish()]);
        Ok(read_floats(self.g, &self.stats, 4)?[0])
    }

    /// One frame, as [`SimState::step_frame`], then reads the fields back.
    pub fn step_frame(&mut self, p: &SimParams) -> Result<(), String> {
        let dt = 1.0 / p.fps.clamp(1.0, 240.0);
        let speed = [self.max_speed()?, p.source_speed.abs(), p.wind[0].abs(), p.wind[1].abs()].into_iter().fold(0.0f32, f32::max);
        let steps = ((speed * dt * 3f32.sqrt() / (self.state.h * 1.5)).ceil() as u32).clamp(1, 6);
        for _ in 0..steps {
            self.substep(p, dt / steps as f32)?;
        }
        self.state.frame += 1;
        self.read_back()
    }

    fn params(&self, p: &SimParams, dt: f32, emits: u32) -> Vec<u8> {
        let s = &self.state;
        let n = s.n.map(|v| v as u32);
        let u = [n[0], n[1], n[2], s.density.d.len() as u32];
        let f = [
            s.h,
            dt,
            p.buoyancy,
            p.weight,
            p.wind[0],
            p.wind[1],
            (0.5 * dt).min(1.0),
            p.swirl.max(0.0) * SWIRL_GAIN * s.h,
            p.burn_rate.max(0.0) * dt,
            p.ignition,
            p.heat_release,
            p.soot,
            (-p.dissipation.max(0.0) * dt).exp(),
            (-p.cooling.max(0.0) * dt).exp(),
        ];
        let mut out: Vec<u8> = u.iter().flat_map(|v| v.to_le_bytes()).collect();
        out.extend(f.iter().flat_map(|v| v.to_le_bytes()));
        out.extend(emits.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        debug_assert_eq!(out.len() as u64, PARAM_BYTES);
        out
    }

    fn substep(&mut self, p: &SimParams, dt: f32) -> Result<(), String> {
        let list = self.state.emit_list(p, dt);
        let words: Vec<u8> = list.iter().flat_map(|e| e.words()).flat_map(f32::to_le_bytes).collect();
        if words.len() as u64 > self.cap {
            (self.cap, self.emits) = (words.len() as u64, buffer(&self.g.device, "emission", words.len() as u64, wgpu::BufferUsages::empty()));
            self.bind = bind_group(self.g, self.k, &self.uniform, &self.vel, &self.sca, &self.tmp, &self.emits, &self.stats);
        }
        let q = &self.g.queue;
        q.write_buffer(&self.uniform, 0, &self.params(p, dt, list.len() as u32));
        if !words.is_empty() {
            q.write_buffer(&self.emits, 0, &words);
        }
        let cells = self.state.density.d.len() as u64 * 4;
        let mut enc = self.g.device.create_command_encoder(&Default::default());
        self.dispatch_linear(&mut enc, "emit", list.len() as u32);
        self.dispatch(&mut enc, &[("forces", 0)]);
        if p.swirl > 0.0 {
            self.dispatch(&mut enc, &[("curl_mag", 0), ("swirl", 0)]);
            self.copy(&mut enc, &self.vel);
        }
        self.dispatch(&mut enc, &[("burn", 0), ("divergence", 0)]);
        enc.clear_buffer(&self.tmp, cells, Some(cells * 2));
        let iters = p.pressure_iters.clamp(1, 200);
        let jacobi: Vec<_> = (0..iters).map(|i| if i % 2 == 0 { ("jacobi_pq", 0) } else { ("jacobi_qp", 0) }).collect();
        self.dispatch(&mut enc, &jacobi);
        // An odd count leaves the newest pressure in slot 2.
        self.dispatch(&mut enc, &[("gradient", if iters % 2 == 1 { 2 } else { 1 })]);
        self.copy(&mut enc, &self.vel);
        let adv: Vec<_> = (0..3).flat_map(|f| [("adv_fwd", f), ("adv_rev", 0), ("adv_fin", f)]).chain([("adv_vel", 0)]).collect();
        self.dispatch(&mut enc, &adv);
        self.copy(&mut enc, &self.sca);
        self.copy(&mut enc, &self.vel);
        self.dispatch(&mut enc, &[("fade", 0)]);
        q.submit([enc.finish()]);
        self.state.time += dt;
        Ok(())
    }

    /// Runs grid kernels in order (each sees the last one's writes).
    fn dispatch(&self, enc: &mut wgpu::CommandEncoder, list: &[(&str, u32)]) {
        let n = self.state.n.map(|v| (v as u32).div_ceil(4));
        let mut cp = enc.begin_compute_pass(&Default::default());
        cp.set_bind_group(0, &self.bind, &[]);
        for &(e, f) in list {
            cp.set_pipeline(self.k.get(e, f));
            cp.dispatch_workgroups(n[0], n[1], n[2]);
        }
    }

    fn dispatch_linear(&self, enc: &mut wgpu::CommandEncoder, e: &str, count: u32) {
        if count == 0 {
            return;
        }
        let groups = count.div_ceil(64);
        let (x, y) = (groups.min(65_535), groups.div_ceil(65_535));
        let mut cp = enc.begin_compute_pass(&Default::default());
        cp.set_bind_group(0, &self.bind, &[]);
        cp.set_pipeline(self.k.get(e, 0));
        cp.dispatch_workgroups(x, y, 1);
    }

    /// Next values become current.
    fn copy(&self, enc: &mut wgpu::CommandEncoder, pair: &[wgpu::Buffer; 2]) {
        enc.copy_buffer_to_buffer(&pair[1], 0, &pair[0], 0, None);
    }

    fn read_back(&mut self) -> Result<(), String> {
        let c = self.state.density.d.len();
        let bytes = c as u64 * 12;
        let (s, v) = (read_floats(self.g, &self.sca[0], bytes)?, read_floats(self.g, &self.vel[0], bytes)?);
        let st = &mut self.state;
        for (k, f) in [&mut st.density, &mut st.temperature, &mut st.fuel].into_iter().enumerate() {
            f.d.copy_from_slice(&s[k * c..(k + 1) * c]);
        }
        for (k, f) in st.vel.iter_mut().enumerate() {
            f.d.copy_from_slice(&v[k * c..(k + 1) * c]);
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn bind_group(g: &Gpu, k: &Kernels, u: &wgpu::Buffer, vel: &[wgpu::Buffer; 2], sca: &[wgpu::Buffer; 2], tmp: &wgpu::Buffer, emits: &wgpu::Buffer, stats: &wgpu::Buffer) -> wgpu::BindGroup {
    let bufs = [u, &vel[0], &vel[1], &sca[0], &sca[1], tmp, emits, stats];
    let entries: Vec<_> = bufs.iter().enumerate().map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() }).collect();
    g.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("smoke solver"), layout: &k.layout, entries: &entries })
}
