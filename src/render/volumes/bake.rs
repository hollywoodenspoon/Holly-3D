//! Clouds V2b: bakes a cloud's eroded density into a small voxel grid for
//! the viewport preview (a Bevy fog volume samples it as a 3D texture).
//! The bake runs the same density as the F9 renderers, so the preview shows
//! the same billows, only coarser.

use super::{NoiseDetail, VolumeObj, parts_of};
use crate::materials::volume::VolumeMat;
use crate::scene::{Scene, SceneObject};
use bevy::math::{Mat4, Vec3};

/// A baked density grid in the object's local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Baked {
    /// Local bounds the grid covers.
    pub lo: Vec3,
    pub hi: Vec3,
    /// Voxels along local x, y, z (x fastest in `data`).
    pub dims: [u32; 3],
    /// 0..=255 shape density per voxel.
    pub data: Vec<u8>,
    /// The medium (extinction per metre, absorption, scatter colour).
    pub mat: VolumeMat,
}

impl Baked {
    pub fn voxel(&self, x: u32, y: u32, z: u32) -> u8 {
        let [w, h, _] = self.dims;
        self.data[(x + y * w + z * w * h) as usize]
    }
}

/// Voxel dims for a box of `size` holding about `budget` voxels, at least 4
/// and at most 192 along any axis.
pub fn dims_for(size: Vec3, budget: u32) -> [u32; 3] {
    let s = size.max(Vec3::splat(1e-3));
    let k = (budget as f32 / (s.x * s.y * s.z)).cbrt();
    let d = |v: f32| ((v * k).round() as u32).clamp(4, 192);
    [d(s.x), d(s.y), d(s.z)]
}

/// Share of the box size added around the puffs on every side.
const PAD: f32 = 0.06;

/// The preview media of `o` (puff volumes only: clouds and trails).
pub fn preview_volumes(scene: &Scene, o: &SceneObject) -> Vec<VolumeObj> {
    parts_of(scene, o)
        .into_iter()
        .filter(|(src, ..)| matches!(src, super::density::Source::Puffs(_)))
        .map(|(src, lo, hi, m)| {
            // Padding keeps the outer puffs off the box faces, where the
            // coarse grid would show them as flat cuts.
            let pad = (hi - lo) * PAD;
            VolumeObj::new(src, lo - pad, hi + pad, Mat4::IDENTITY, m, 0)
        })
        .collect()
}

/// The first preview medium of `o`, if it renders as a puff volume.
pub fn preview_volume(scene: &Scene, o: &SceneObject) -> Option<VolumeObj> {
    preview_volumes(scene, o).into_iter().next()
}

/// Bakes `v` at voxel centres with about `budget` voxels (the outer shell
/// of voxels left empty). Low noise detail
/// keeps the bake quick; the grid is too coarse for finer octaves anyway.
pub fn bake(mut v: VolumeObj, budget: u32) -> Baked {
    if let super::density::Source::Puffs(c) = &mut v.src {
        c.octaves = NoiseDetail::Low.octaves();
    }
    let (lo, hi) = (v.grid.lo, v.grid.hi);
    let dims = dims_for(hi - lo, budget);
    let [w, h, d] = dims;
    let cell = (hi - lo) / Vec3::new(w as f32, h as f32, d as f32);
    let mut data = Vec::with_capacity((w * h * d) as usize);
    for z in 0..d {
        for y in 0..h {
            for x in 0..w {
                // The outer shell stays empty so the preview fades out
                // instead of showing a flat cut where the box clips a puff.
                let shell = x == 0 || y == 0 || z == 0 || x == w - 1 || y == h - 1 || z == d - 1;
                let p = lo + (Vec3::new(x as f32, y as f32, z as f32) + 0.5) * cell;
                let rho = if shell { 0.0 } else { v.density(p).clamp(0.0, 1.0) };
                data.push((rho * 255.0).round() as u8);
            }
        }
    }
    Baked { lo, hi, dims, data, mat: v.mat }
}
