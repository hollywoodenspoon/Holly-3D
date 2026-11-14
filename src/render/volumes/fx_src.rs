//! Clouds PR 3: trail media. A trail renders as up to two volumes, its
//! smoke and its flame, each a soft union of puffs worn by erosion.

use super::density::{Blob, Source};
use super::erode::Cloud;
use crate::generators::clouds::{CloudLook, RenderAs};
use crate::generators::trails::TrailParams;
use crate::materials::volume::VolumeMat;
use bevy::math::Vec3;

/// Flames keep a smooth, barely eroded body.
fn flame_look() -> CloudLook {
    CloudLook { erosion: 0.15, wispiness: 0.0, edge_softness: 0.8, ..CloudLook::default() }
}

/// The trail's media: smoke (its own Volume material replaces the type's)
/// and flame, each scaled by the trail's Density.
pub fn media(p: &TrailParams, own: Option<VolumeMat>) -> Vec<(bool, VolumeMat)> {
    if p.look.render_as == RenderAs::Mesh {
        return Vec::new();
    }
    let k = p.look.density.max(0.0);
    let mut out = Vec::new();
    if let Some(m) = own.or_else(|| p.smoke_mat()).filter(|_| p.smoke_mat().is_some()) {
        out.push((false, m));
    }
    if let Some(m) = p.flame_mat() {
        out.push((true, m));
    }
    for (_, m) in &mut out {
        m.density *= k;
    }
    out.retain(|(_, m)| m.density > 0.0);
    out
}

/// The density source of the smoke (`flame` false) or flame puffs.
pub fn source(p: &TrailParams, flame: bool) -> Option<(Source, Vec3, Vec3)> {
    let b: Vec<Blob> =
        p.puffs().iter().filter(|q| q.flame == flame).map(|q| Blob { c: q.c, r: q.r, sq: 1.0, low: 1.0, w: q.w }).collect();
    if b.is_empty() {
        return None;
    }
    let (lo, hi) = b.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), q| (lo.min(q.c - q.half()), hi.max(q.c + q.half())));
    // Erosion features about a third of a typical puff.
    let feature = b.iter().map(|q| q.r).sum::<f32>() / b.len() as f32 * 0.6;
    let look = if flame { flame_look() } else { p.look.clone() };
    let mut c = Cloud::new(b, look, feature, p.seed);
    (c.z0, c.z1) = (lo.z, hi.z);
    Some((Source::Puffs(c), lo, hi))
}
