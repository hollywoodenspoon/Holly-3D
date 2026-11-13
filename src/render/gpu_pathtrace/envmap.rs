//! The environment image the GPU kernel looks up when a path leaves the
//! scene: an HDRI as loaded, or the procedural sky baked into an equirect
//! image once per sky setting. The mapping matches `HdrImage::sample`
//! (Z-up, middle row = horizon, centre column = +X), so the kernel's
//! bilinear lookup returns what the CPU tracer sees.

use crate::render::hdri::HdrImage;
use crate::render::scene_build::RenderScene;
use crate::render::sky::SkySettings;
use bevy::math::Vec3;
use std::sync::{Arc, Mutex};

/// Equirect radiance (rgba32f rows) plus how the kernel turns and scales it.
#[derive(Clone)]
pub struct EnvMap {
    pub w: u32,
    pub h: u32,
    pub px: Arc<Vec<u8>>,
    /// Image turn about Z as a fraction of a full turn.
    pub turn: f32,
    pub strength: f32,
}

impl EnvMap {
    /// A one-texel black map, bound when the scene has no image.
    pub fn none() -> Self {
        Self { w: 1, h: 1, px: Arc::new(vec![0; 16]), turn: 0.0, strength: 0.0 }
    }
}

fn bytes(px: &[Vec3]) -> Vec<u8> {
    px.iter().flat_map(|c| c.extend(0.0).to_array()).flat_map(f32::to_le_bytes).collect()
}

/// World direction through the centre of texel (`x`, `y`) of a `w` x `h` map.
pub fn texel_dir(x: u32, y: u32, w: u32, h: u32) -> Vec3 {
    let (u, v) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
    let phi = (0.5 - u) * std::f32::consts::TAU;
    let theta = v * std::f32::consts::PI;
    Vec3::new(theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos())
}

/// Bakes `sky` into a `w` x `w/2` image on all cores.
pub fn bake(sky: &SkySettings, w: u32) -> HdrImage {
    let (w, h) = (w.max(8), (w / 2).max(4));
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()) as u32;
    let rows_per = h.div_ceil(threads);
    let px = std::thread::scope(|s| {
        let parts: Vec<_> = (0..h)
            .step_by(rows_per as usize)
            .map(|y0| s.spawn(move || (y0..(y0 + rows_per).min(h)).flat_map(|y| (0..w).map(move |x| sky.radiance(texel_dir(x, y, w, h)))).collect::<Vec<_>>()))
            .collect();
        parts.into_iter().flat_map(|p| p.join().expect("sky bake thread")).collect()
    });
    HdrImage { w, h, px }
}

/// The scene's environment image for the GPU (`None` = flat colours only).
/// Bakes and conversions are cached, so a re-render with the same sky or
/// HDRI uploads straight away.
pub fn env_map(rs: &RenderScene, bake_w: u32) -> Option<EnvMap> {
    type SkyKey = (SkySettings, u32);
    type HdriEntry = (Arc<HdrImage>, Arc<Vec<u8>>);
    static SKY: Mutex<Option<(SkyKey, Arc<Vec<u8>>)>> = Mutex::new(None);
    static HDRI: Mutex<Option<HdriEntry>> = Mutex::new(None);
    if let Some(sky) = &rs.procedural {
        let (w, h) = (bake_w.max(8), (bake_w / 2).max(4));
        let mut c = SKY.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let key = (sky.clone(), w);
        let px = match c.as_ref().filter(|(k, _)| *k == key) {
            Some((_, px)) => px.clone(),
            None => {
                let px = Arc::new(bytes(&bake(sky, w).px));
                *c = Some((key, px.clone()));
                px
            }
        };
        return Some(EnvMap { w, h, px, turn: 0.0, strength: 1.0 });
    }
    let hdri = rs.hdri.as_ref()?;
    let img = hdri.image.as_ref()?;
    // Keyed by the image itself (held here, so its address cannot be reused).
    let mut c = HDRI.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let px = match c.as_ref().filter(|(k, _)| Arc::ptr_eq(k, img)) {
        Some((_, px)) => px.clone(),
        None => {
            let px = Arc::new(bytes(&img.px));
            *c = Some((img.clone(), px.clone()));
            px
        }
    };
    Some(EnvMap { w: img.w, h: img.h, px, turn: hdri.rotation / 360.0, strength: hdri.strength })
}
