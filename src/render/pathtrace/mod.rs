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
    (progressive::render(rs, cfg, region, cancel, on_pass), "CPU".into())
}
#[cfg(test)]
mod tests;
