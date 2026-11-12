//! Native progressive path tracer (Phase 11.5A), replacing the old Cycles /
//! Blender bridge: multi-bounce paths with MIS and Russian Roulette on the
//! CPU, plus a wgpu compute port of the SolidCraft GPU path tracer.

pub mod adaptive;
pub mod bsdf;
pub mod integrator;
pub mod progressive;
pub mod settings;

pub use settings::{ComputeDevice, Integrator, PathSettings};

use crate::render::job::Region;
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::trace::Pixel;
use std::sync::atomic::AtomicBool;

/// Renders with the chosen Compute Device; returns the pixels and the name
/// of the device that ran (GPU falls back to the CPU without an adapter).
pub fn run(
    rs: &RenderScene,
    cfg: &RenderSettings,
    region: Region,
    cancel: &AtomicBool,
    on_pass: impl FnMut(progressive::PassInfo, &[Pixel]),
) -> (Vec<Pixel>, String) {
    let mut on_pass = on_pass;
    let gpu = match cfg.path.device {
        ComputeDevice::Cpu => Err("CPU selected".to_string()),
        _ => match crate::render::gpu_pathtrace::pack::unsupported(rs) {
            Some(why) => Err(format!("{why} need the CPU tracer")),
            None => crate::render::gpu_pathtrace::gpu(),
        },
    };
    let fallback = match gpu {
        Ok(g) => match crate::render::gpu_pathtrace::render(g, rs, cfg, region, cancel, &mut on_pass) {
            Ok(px) => return (px, g.name.clone()),
            Err(e) => e,
        },
        Err(e) => e,
    };
    let label = if cfg.path.device == ComputeDevice::Cpu { "CPU".to_string() } else { format!("CPU ({fallback})") };
    (progressive::render(rs, cfg, region, cancel, on_pass), label)
}
#[cfg(test)]
mod tests;
