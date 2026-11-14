//! Volume objects for the renderers: any object whose material uses the
//! Volume shading model renders as a participating medium instead of a
//! surface. Clouds and Cloud Fields give a soft union of their puffs; other
//! meshes fill their local bounding box. Collisions come from delta tracking
//! and shadows from ratio tracking, both stepping through a majorant grid.

pub mod bake;
pub mod density;
pub mod erode;
pub mod majorant;
mod render;
pub mod settings;
mod tracking;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_v2;
#[cfg(test)]
mod tests_bake;

use crate::generators::clouds::{CloudLook, CloudShape, CloudType, RenderAs};
use crate::materials::volume::{VolumeMat, VolumePreset};
use crate::materials::{Material, ShadingModel};
use crate::scene::{BaseObject, Scene, SceneObject};
use crate::generators::WorldGen;
use bevy::math::{Mat4, Vec3};
use density::{Blob, Source};
use erode::Cloud;
use majorant::{Grid, dims_for};

pub use render::sample_phase;
pub use erode::NoiseDetail;
pub use settings::{ShadowQuality, VolumeRender};

/// One medium in the render scene.
pub struct VolumeObj {
    /// World to local.
    pub inv: Mat4,
    pub src: Source,
    pub mat: VolumeMat,
    pub grid: Grid,
    /// Object index (as in `Tri::obj`).
    pub obj: u32,
}

/// A real collision found by delta tracking.
#[derive(Clone, Copy, Debug)]
pub struct Collision {
    pub t: f32,
    pub vol: usize,
    /// 0..=1 shape density at the collision (drives the glow ramp).
    pub density: f32,
}

#[derive(Default)]
pub struct Volumes {
    pub list: Vec<VolumeObj>,
}

/// The volume material of `o` (Material ID 1 of a Multi/Sub-Object), if any.
pub fn volume_material<'a>(scene: &'a Scene, o: &SceneObject) -> Option<&'a Material> {
    let m = o.material.and_then(|id| scene.materials.mats.get(id))?.for_id(1);
    (m.model == ShadingModel::Volume).then_some(m)
}

/// The cloud look of a Cloud or Cloud Field object.
fn cloud_look(o: &SceneObject) -> Option<&CloudLook> {
    match &o.base {
        BaseObject::World(WorldGen::Cloud(p)) => Some(&p.look),
        BaseObject::World(WorldGen::CloudField(f)) => Some(&f.look),
        _ => None,
    }
}

/// What `o` renders as a medium with, if anything: its Volume material, or
/// for a cloud in `Render As: Volume` mode a type preset when its material
/// is a surface, scaled by the cloud's Density. Clouds hidden by the
/// Environment's 3D clouds switch render nothing.
pub fn volume_of(scene: &Scene, o: &SceneObject) -> Option<VolumeMat> {
    let own = volume_material(scene, o).map(|m| m.volume.clone());
    let Some(look) = cloud_look(o) else { return own };
    if look.render_as == RenderAs::Mesh || !scene.environment.sky.clouds.three_d {
        return None;
    }
    let storm = matches!(&o.base, BaseObject::World(WorldGen::Cloud(p)) if p.kind == CloudType::Cumulonimbus);
    let mut m = own.unwrap_or_else(|| if storm { VolumePreset::StormCloud } else { VolumePreset::CumulusCloud }.params());
    m.density *= look.density.max(0.0);
    Some(m)
}

fn blobs(s: &CloudShape, flat: f32, at: Vec3, out: &mut Vec<Blob>) {
    let low = 1.0 - flat.clamp(0.0, 0.98);
    out.extend(s.puffs.iter().map(|q| Blob { c: q.c + at, r: q.r, sq: q.sq, low }));
}

fn source_of(o: &SceneObject) -> Option<(Source, Vec3, Vec3)> {
    let mut b = Vec::new();
    // Erosion feature size: about a fourteenth of a typical cloud.
    let (look, feature, seed) = match &o.base {
        BaseObject::World(WorldGen::Cloud(p)) => {
            blobs(&p.shape(), p.base_flatness, Vec3::ZERO, &mut b);
            (&p.look, p.size * 0.07, p.seed)
        }
        BaseObject::World(WorldGen::CloudField(f)) => {
            for pl in f.placements() {
                blobs(&pl.params.shape(), pl.params.base_flatness, pl.offset, &mut b);
            }
            (&f.look, (f.min_size + f.max_size) * 0.035, f.seed)
        }
        _ => {
            let bb = o.render_mesh().bounds()?;
            let hi = bb.max.max(bb.min + Vec3::splat(1e-3));
            return ((bb.max - bb.min).max_element() > 0.0).then_some((Source::Uniform, bb.min, hi));
        }
    };
    let (lo, hi) = b.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), q| (lo.min(q.c - q.half()), hi.max(q.c + q.half())));
    let mut c = Cloud::new(b, look.clone(), feature, seed);
    (c.z0, c.z1) = (lo.z, hi.z);
    (!c.blobs.is_empty()).then_some((Source::Puffs(c), lo, hi))
}

impl VolumeObj {
    pub fn new(src: Source, lo: Vec3, hi: Vec3, world: Mat4, mat: VolumeMat, obj: u32) -> Self {
        let target = match &src {
            Source::Puffs(c) if c.blobs.len() > 600 => 48,
            Source::Puffs(c) if c.blobs.len() > 150 => 32,
            Source::Uniform => 1,
            _ => 16,
        };
        let grid = Grid::build(&src, lo, hi, dims_for(hi - lo, target));
        Self { inv: world.inverse(), src, mat, grid, obj }
    }

    /// 0..=1 shape density at local point `p`.
    pub fn density(&self, p: Vec3) -> f32 {
        match &self.src {
            Source::Uniform => 1.0,
            Source::Grid(v) => v.sample((p - self.grid.lo) / (self.grid.hi - self.grid.lo).max(Vec3::splat(1e-9))),
            Source::Puffs(c) => c.density(p, &self.grid.lists[self.grid.index(self.grid.cell_of(p))]),
        }
    }

    /// Extinction per metre where the shape is fully dense.
    pub fn sigma(&self) -> f32 {
        self.mat.density.max(0.0)
    }
}

impl Volumes {
    pub fn build(scene: &Scene) -> Self {
        let mut list = Vec::new();
        for (obj, o) in scene.objects.values().enumerate().filter(|(_, o)| !o.hidden && !o.is_helper()) {
            let Some(m) = volume_of(scene, o) else { continue };
            let Some((src, lo, hi)) = source_of(o) else { continue };
            if m.density > 0.0 {
                list.push(VolumeObj::new(src, lo, hi, o.world_matrix(), m, obj as u32));
            }
        }
        Self { list }
    }

    /// Applies F10 Noise Detail to every cloud's erosion.
    pub fn set_detail(&mut self, detail: NoiseDetail) {
        for v in &mut self.list {
            if let Source::Puffs(c) = &mut v.src {
                c.octaves = detail.octaves();
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Nearest real collision with any volume before `tmax` (each volume
    /// tracked on its own; the nearest wins, as for summed extinction).
    pub fn collide(&self, o: Vec3, d: Vec3, tmax: f32, rng: &mut crate::render::trace::Rng) -> Option<Collision> {
        let mut best: Option<Collision> = None;
        for (i, v) in self.list.iter().enumerate() {
            let lim = best.map_or(tmax, |b| b.t);
            if let Some((t, density)) = v.collide(o, d, lim, rng) {
                best = Some(Collision { t, vol: i, density });
            }
        }
        best
    }

    /// Ratio-tracked transmittance through every volume, averaged over `n`
    /// estimates (Shadow Quality).
    pub fn transmittance(&self, o: Vec3, d: Vec3, tmax: f32, n: u32, rng: &mut crate::render::trace::Rng) -> f32 {
        if self.list.is_empty() {
            return 1.0;
        }
        let n = n.max(1);
        let sum: f32 = (0..n).map(|_| self.list.iter().map(|v| v.transmittance(o, d, tmax, rng)).product::<f32>()).sum();
        sum / n as f32
    }
}
