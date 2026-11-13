//! LightMix on the GPU: with `PathSettings::lightmix` on, the kernel
//! (`FEAT_LIGHTMIX`) splits every path's light into the same groups as the
//! CPU tracer (sun & sky, environment, emissive, volume, one per scene
//! light) and sums them per pixel into one storage buffer, read back once
//! at the end into the Frame Window's [`Buffers`].

use super::Gpu;
use crate::render::job::Region;
use crate::render::lightmix::{Buffers, GROUPS};
use crate::render::pathtrace::progressive::Accum;
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use bevy::math::Vec3;

/// Groups this render keeps (0 = LightMix off).
pub fn groups(rs: &RenderScene, cfg: &RenderSettings) -> usize {
    if cfg.path.lightmix { rs.groups.len().min(GROUPS) } else { 0 }
}

/// Bytes of the group buffer for `n` pixels and `k` groups (one float per
/// channel; a single word when LightMix is off, as the binding needs one).
pub fn bytes(n: usize, k: usize) -> u64 {
    ((n * k * 3).max(1) * 4) as u64
}

/// The zeroed group buffer, or why it does not fit this GPU.
pub fn buffer(d: &wgpu::Device, n: usize, k: usize, limit: u64) -> Result<wgpu::Buffer, String> {
    let size = bytes(n, k);
    if size > limit {
        return Err(format!("LightMix buffers ({} MB) exceed this GPU's {} MB buffer limit", size >> 20, limit >> 20));
    }
    Ok(super::storage(d, "lightmix", &vec![0; size as usize], true))
}

/// Reads the group sums back and normalises them like the CPU tracer.
pub fn finish(g: &Gpu, buf: &wgpu::Buffer, rs: &RenderScene, cfg: &RenderSettings, region: Region, acc: &[Accum], k: usize) -> Result<Buffers, String> {
    let f = super::read_floats(g, buf, bytes(acc.len(), k))?;
    let data = f.as_chunks::<3>().0.iter().map(|c| Vec3::from_array(*c)).collect();
    // The GPU's image uses the bilateral denoiser, so no À-Trous guides.
    Ok(crate::render::pathtrace::extras::lightmix(rs, cfg, region, acc, data, k, Vec::new()))
}
