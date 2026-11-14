//! Clouds PR 3 / 4: media of trails, explosions and smoke plumes. Each renders as a few
//! volumes (smoke, flame, dust, vapour, sparks), each a soft union of
//! puffs worn by erosion; glowing media keep a smooth, barely eroded body.

use super::density::{Blob, Source};
use super::erode::Cloud;
use crate::generators::WorldGen;
use crate::generators::clouds::{CloudLook, RenderAs};
use crate::generators::pyro::{Medium, PyroParams};
use crate::generators::smoke::SmokeParams;
use crate::generators::trails::TrailParams;
use crate::materials::volume::VolumeMat;
use crate::scene::{BaseObject, SceneObject};
use bevy::math::Vec3;

/// One medium of an effect: its material and its puffs.
pub struct Part {
    pub mat: VolumeMat,
    pub blobs: Vec<Blob>,
    pub look: CloudLook,
    pub seed: u32,
}

fn glow_look() -> CloudLook {
    CloudLook { erosion: 0.15, wispiness: 0.0, edge_softness: 0.8, ..CloudLook::default() }
}

fn blob(c: Vec3, r: f32, sq: f32, w: f32) -> Blob {
    Blob { c, r, sq, low: 1.0, w }
}

/// The parts of a trail: smoke (its own Volume material replaces the
/// type's) and flame, each scaled by the trail's Density.
fn trail(p: &TrailParams, own: Option<VolumeMat>) -> Vec<Part> {
    let puffs = p.puffs();
    let media = [(false, p.smoke_mat().map(|m| own.clone().unwrap_or(m))), (true, p.flame_mat())];
    media
        .into_iter()
        .filter_map(|(flame, m)| {
            let blobs = puffs.iter().filter(|q| q.flame == flame).map(|q| blob(q.c, q.r, 1.0, q.w)).collect();
            Some(Part { mat: m?, blobs, look: if flame { glow_look() } else { p.look.clone() }, seed: p.seed })
        })
        .collect()
}

/// The parts of an explosion, one per medium present; an own Volume
/// material replaces the smoke.
fn pyro(p: &PyroParams, own: Option<VolumeMat>) -> Vec<Part> {
    let puffs = p.puffs();
    Medium::ALL
        .into_iter()
        .filter_map(|m| {
            let blobs: Vec<Blob> = puffs.iter().filter(|q| q.medium == m).map(|q| blob(q.c, q.r, q.sq, q.w)).collect();
            let mat = if m == Medium::Smoke { own.clone().or_else(|| p.mat(m)) } else { p.mat(m) }?;
            let look = if m.glows() { glow_look() } else { p.look.clone() };
            Some(Part { mat, blobs, look, seed: p.seed.wrapping_add(m as u32) })
        })
        .collect()
}

/// The parts of a smoke plume: smoke (its own Volume material replaces
/// the style's) and, when it burns, flame.
fn plume(p: &SmokeParams, own: Option<VolumeMat>) -> Vec<Part> {
    let puffs = p.puffs();
    let media = [(false, Some(own.unwrap_or_else(|| p.smoke_mat()))), (true, p.fire_mat())];
    media
        .into_iter()
        .filter_map(|(fire, m)| {
            let blobs = puffs.iter().filter(|q| q.fire == fire).map(|q| blob(q.c, q.r, q.sq, q.w)).collect();
            Some(Part { mat: m?, blobs, look: if fire { glow_look() } else { p.look.clone() }, seed: p.seed.wrapping_add(u32::from(fire)) })
        })
        .collect()
}

/// The effect parts of `o` (empty for other objects and `Render As: Mesh`).
pub fn parts(o: &SceneObject, own: Option<VolumeMat>) -> Vec<Part> {
    let (mut out, look) = match &o.base {
        BaseObject::World(WorldGen::Trail(p)) => (trail(p, own), &p.look),
        BaseObject::World(WorldGen::Pyro(p)) => (pyro(p, own), &p.look),
        BaseObject::World(WorldGen::Smoke(p)) => (plume(p, own), &p.look),
        _ => return Vec::new(),
    };
    if look.render_as == RenderAs::Mesh {
        return Vec::new();
    }
    let k = look.density.max(0.0);
    for part in &mut out {
        part.mat.density *= k;
    }
    out.retain(|q| q.mat.density > 0.0 && !q.blobs.is_empty());
    out
}

/// The first medium's material of `o`, without laying out its puffs (the
/// viewport asks every frame).
pub fn first_mat(o: &SceneObject, own: Option<VolumeMat>) -> Option<VolumeMat> {
    let (m, look) = match &o.base {
        BaseObject::World(WorldGen::Trail(p)) => (p.smoke_mat().map(|m| own.unwrap_or(m)).or_else(|| p.flame_mat()), &p.look),
        BaseObject::World(WorldGen::Pyro(p)) => (own.or_else(|| p.mat(Medium::Smoke)), &p.look),
        BaseObject::World(WorldGen::Smoke(p)) => (Some(own.unwrap_or_else(|| p.smoke_mat())), &p.look),
        BaseObject::World(WorldGen::Vdb(p)) => return p.smoke_mat().map(|m| own.unwrap_or(m)).or_else(|| p.fire_mat()),
        BaseObject::World(WorldGen::SmokeSim(p)) => return p.smoke_mat().map(|m| own.unwrap_or(m)).or_else(|| p.fire_mat()),
        _ => return None,
    };
    let m = m.filter(|_| look.render_as == RenderAs::Volume)?;
    let m = VolumeMat { density: m.density * look.density.max(0.0), ..m };
    (m.density > 0.0).then_some(m)
}

/// Changes when what `o` shows changes without an edit (a simulation
/// frame finishing its bake), so previews refresh.
pub fn stamp(o: &SceneObject) -> u64 {
    match &o.base {
        BaseObject::World(WorldGen::SmokeSim(p)) => p.stamp(),
        _ => 0,
    }
}

/// How many parts `o` may have, cheaply (viewport preview bookkeeping).
pub fn part_count(o: &SceneObject) -> usize {
    match &o.base {
        BaseObject::World(WorldGen::Trail(p)) => usize::from(p.smoke_mat().is_some()) + usize::from(p.flame_mat().is_some()),
        BaseObject::World(WorldGen::Pyro(_)) => Medium::ALL.len(),
        BaseObject::World(WorldGen::Smoke(p)) => 1 + usize::from(p.fire_mat().is_some()),
        BaseObject::World(WorldGen::Vdb(p)) => usize::from(p.smoke_mat().is_some()) + usize::from(p.fire_mat().is_some()),
        BaseObject::World(WorldGen::SmokeSim(p)) => usize::from(p.smoke_mat().is_some()) + usize::from(p.fire_mat().is_some()),
        _ => 1,
    }
}

/// The density source of a part.
pub fn source(part: Part) -> (Source, Vec3, Vec3, VolumeMat) {
    let b = part.blobs;
    let (lo, hi) = b.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), q| (lo.min(q.c - q.half()), hi.max(q.c + q.half())));
    // Erosion features about a third of a typical puff.
    let feature = b.iter().map(|q| q.r).sum::<f32>() / b.len().max(1) as f32 * 0.6;
    let mut c = Cloud::new(b, part.look, feature, part.seed);
    (c.z0, c.z1) = (lo.z, hi.z);
    (Source::Puffs(c), lo, hi, part.mat)
}
