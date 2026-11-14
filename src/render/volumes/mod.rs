//! Volume objects for the renderers: any object whose material uses the
//! Volume shading model renders as a participating medium instead of a
//! surface. Clouds and Cloud Fields give a soft union of their puffs; other
//! meshes fill their local bounding box. Collisions come from delta tracking
//! and shadows from ratio tracking, both stepping through a majorant grid.

pub mod density;
pub mod majorant;
mod render;
pub mod settings;
mod tracking;

#[cfg(test)]
mod tests;

use crate::generators::clouds::CloudShape;
use crate::materials::volume::VolumeMat;
use crate::materials::{Material, ShadingModel};
use crate::scene::{BaseObject, Scene, SceneObject};
use crate::generators::WorldGen;
use bevy::math::{Mat4, Vec3};
use density::{Blob, Source};
use majorant::{Grid, dims_for};

pub use render::sample_phase;
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

fn blobs(s: &CloudShape, flat: f32, at: Vec3, out: &mut Vec<Blob>) {
    let low = 1.0 - flat.clamp(0.0, 0.98);
    out.extend(s.puffs.iter().map(|q| Blob { c: q.c + at, r: q.r, sq: q.sq, low }));
}

fn source_of(o: &SceneObject) -> Option<(Source, Vec3, Vec3)> {
    let mut b = Vec::new();
    match &o.base {
        BaseObject::World(WorldGen::Cloud(p)) => blobs(&p.shape(), p.base_flatness, Vec3::ZERO, &mut b),
        BaseObject::World(WorldGen::CloudField(f)) => {
            for pl in f.placements() {
                blobs(&pl.params.shape(), pl.params.base_flatness, pl.offset, &mut b);
            }
        }
        _ => {
            let bb = o.render_mesh().bounds()?;
            let hi = bb.max.max(bb.min + Vec3::splat(1e-3));
            return ((bb.max - bb.min).max_element() > 0.0).then_some((Source::Uniform, bb.min, hi));
        }
    }
    let (lo, hi) = b.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), q| (lo.min(q.c - q.half()), hi.max(q.c + q.half())));
    (!b.is_empty()).then_some((Source::Puffs(b), lo, hi))
}

impl VolumeObj {
    pub fn new(src: Source, lo: Vec3, hi: Vec3, world: Mat4, mat: VolumeMat, obj: u32) -> Self {
        let target = match &src {
            Source::Puffs(b) if b.len() > 600 => 48,
            Source::Puffs(b) if b.len() > 150 => 32,
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
            Source::Puffs(b) => {
                let i = self.grid.index(self.grid.cell_of(p));
                density::union(self.grid.lists[i].iter().map(|&k| b[k as usize].at(p)))
            }
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
            let Some(m) = volume_material(scene, o) else { continue };
            let Some((src, lo, hi)) = source_of(o) else { continue };
            if m.volume.density > 0.0 {
                list.push(VolumeObj::new(src, lo, hi, o.world_matrix(), m.volume.clone(), obj as u32));
            }
        }
        Self { list }
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
