//! What a finished CPU path-traced render hands over besides the image:
//! the AOV passes, the À-Trous denoiser guides and the LightMix buffers.

use super::progressive::Accum;
use crate::render::aov::{AovBuffers, AovPixel};
use crate::render::denoise_atrous::Aux;
use crate::render::job::Region;
use crate::render::lightmix::{Buffers, Fader};
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use bevy::math::Vec3;

/// AOV passes and LightMix buffers of a finished render.
#[derive(Default)]
pub struct Extras {
    pub lightmix: Option<Buffers>,
    pub aovs: Option<AovBuffers>,
}

fn inv(a: &Accum) -> f32 {
    1.0 / a.n.max(1) as f32
}

/// Denoiser guides: mean albedo / roughness and the variance of the mean.
pub fn aux(acc: &[Accum]) -> Vec<Aux> {
    acc.iter()
        .map(|a| Aux { albedo: a.aov.albedo * inv(a), rough: a.rough * inv(a), var: (a.stats.variance() / f64::from(a.n.max(1))) as f32 })
        .collect()
}

/// Mean AOV passes of every pixel.
pub fn aovs(region: Region, acc: &[Accum]) -> AovBuffers {
    let px = acc
        .iter()
        .map(|a| {
            let (k, v) = (inv(a), a.aov);
            AovPixel {
                direct: v.direct * k,
                indirect: v.indirect * k,
                volume: v.volume * k,
                specular: v.specular * k,
                sss: v.sss * k,
                albedo: v.albedo * k,
                normal: a.g.normal,
                depth: a.g.depth,
                ao: if a.g.depth.is_finite() { v.ao * k } else { 1.0 },
                samples: a.n,
            }
        })
        .collect();
    AovBuffers { region, px }
}

/// Normalises the accumulated group sums into LightMix buffers.
pub fn lightmix(rs: &RenderScene, cfg: &RenderSettings, region: Region, acc: &[Accum], mut data: Vec<Vec3>, k: usize, aux: Vec<Aux>) -> Buffers {
    for (a, g) in acc.iter().zip(data.chunks_mut(k)) {
        let s = inv(a);
        g.iter_mut().for_each(|c| *c *= s);
    }
    let ctx = crate::render::postfx::PostCtx::of(rs, cfg);
    Buffers {
        region,
        groups: rs.groups[..k].to_vec(),
        data,
        alpha: acc.iter().map(|a| a.sum.w * inv(a)).collect(),
        back: acc.iter().map(|a| a.back).collect(),
        g: acc.iter().map(|a| a.g).collect(),
        faders: vec![Fader::default(); k],
        cfg: cfg.clone(),
        materials: rs.materials.clone(),
        noisy: ctx.noisy,
        coc: ctx.coc,
        plain_bg: ctx.plain_bg,
        aux,
    }
}
