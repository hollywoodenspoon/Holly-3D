//! Step C: VDB Volume. Imports simulated smoke and fire from an OpenVDB
//! file (File -> Import VDB Volume…, or Create -> Clouds & FX) as a voxel
//! grid volume: a density grid renders as smoke and an optional
//! temperature / flame grid as glowing fire. The scene keeps only the file
//! path and settings; the voxels are read (and cached) when needed. In the
//! viewport the object shows its bounds as a thin cage and, with volume
//! previews on, the baked smoke itself.

mod load;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod write;

pub use load::{GridData, float_grids, grid, resample};

use crate::generators::kit::Kit;
use crate::materials::volume::{EmissionRamp, VolumeMat, VolumePreset};
use crate::mesh::EditablePoly;
use crate::render::volumes::density::Source;
use bevy::math::{Quat, Vec3};
use std::sync::Arc;

/// Which file axis points up (simulation tools mostly write Y-up).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum UpAxis {
    #[default]
    Y,
    Z,
}

/// The look of the density grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VdbStyle {
    #[default]
    GreySmoke,
    BlackSmoke,
    Steam,
    Cloud,
}

impl VdbStyle {
    pub const ALL: [VdbStyle; 4] = [Self::GreySmoke, Self::BlackSmoke, Self::Steam, Self::Cloud];
    pub fn label(self) -> &'static str {
        match self {
            Self::GreySmoke => "Grey Smoke",
            Self::BlackSmoke => "Black Smoke",
            Self::Steam => "Steam (white)",
            Self::Cloud => "Cloud",
        }
    }
    fn mat(self) -> VolumeMat {
        match self {
            Self::GreySmoke => VolumePreset::GreySmoke.params(),
            Self::BlackSmoke => VolumePreset::BlackSmoke.params(),
            Self::Steam => VolumeMat { scatter: [0.96, 0.96, 0.97], g_forward: 0.6, ..VolumePreset::CumulusCloud.params() },
            Self::Cloud => VolumePreset::CumulusCloud.params(),
        }
    }
}

/// Names a fire grid usually has, in order of preference.
pub const FIRE_NAMES: [&str; 6] = ["temperature", "flame", "flames", "fire", "heat", "temp"];

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct VdbParams {
    /// The .vdb file.
    pub path: String,
    /// Grid rendered as smoke ("" = none).
    pub density_grid: String,
    /// Grid rendered as glowing fire ("" = none).
    pub fire_grid: String,
    pub style: VdbStyle,
    pub up: UpAxis,
    /// Scale: metres per file unit.
    pub scale: f32,
    /// Max Resolution: voxels along the longest axis after resampling.
    pub resolution: u32,
    /// Smoke Density: extinction per metre where the grid is at its peak.
    pub density: f32,
    /// Fire Intensity 0..=100 (emission strength of the fire grid).
    pub fire: f32,
}

impl Default for VdbParams {
    fn default() -> Self {
        Self { path: String::new(), density_grid: "density".into(), fire_grid: String::new(), style: VdbStyle::GreySmoke, up: UpAxis::Y, scale: 1.0, resolution: 192, density: 2.0, fire: 60.0 }
    }
}

/// One medium of a VDB volume, ready for the renderers.
pub struct VdbPart {
    pub src: Source,
    pub lo: Vec3,
    pub hi: Vec3,
    pub mat: VolumeMat,
}

impl VdbParams {
    /// Settings for a freshly picked file: its density grid (or the first
    /// float grid) and a fire grid when one has a usual name.
    pub fn for_file(path: &str) -> Result<Self, String> {
        let grids = float_grids(path)?;
        let density = ["density", "smoke", "soot", "dens"].iter().find(|n| grids.iter().any(|g| g == *n)).map(|n| n.to_string());
        let fire = FIRE_NAMES.iter().find(|n| grids.iter().any(|g| g == *n)).map(|n| n.to_string()).unwrap_or_default();
        let density = density.or_else(|| grids.iter().find(|g| **g != fire).cloned()).unwrap_or_default();
        if density.is_empty() && fire.is_empty() {
            return Err(format!("{path} has no scalar float grids to render"));
        }
        Ok(Self { path: path.into(), density_grid: density, fire_grid: fire, ..Self::default() })
    }

    fn load(&self, name: &str) -> Option<Arc<GridData>> {
        (!name.is_empty() && !self.path.is_empty()).then(|| grid(&self.path, name, self.up, self.resolution).ok()).flatten()
    }

    pub fn smoke_mat(&self) -> Option<VolumeMat> {
        let d = self.density.max(0.0);
        (!self.density_grid.is_empty() && d > 0.0).then(|| VolumeMat { density: d, ..self.style.mat() })
    }

    pub fn fire_mat(&self) -> Option<VolumeMat> {
        let f = (self.fire / 100.0).clamp(0.0, 1.0);
        (!self.fire_grid.is_empty() && f > 0.0).then(|| VolumeMat { ramp: EmissionRamp::Fire, emission: 4.0 + 8.0 * f, density: 1.2, ..VolumePreset::Fire.params() })
    }

    /// The smoke and fire media (only grids that load).
    pub fn parts(&self, own: Option<VolumeMat>) -> Vec<VdbPart> {
        let media = [(self.density_grid.as_str(), self.smoke_mat().map(|m| own.unwrap_or(m))), (self.fire_grid.as_str(), self.fire_mat())];
        let s = self.scale.max(1e-6);
        media
            .into_iter()
            .filter_map(|(name, mat)| {
                let (mat, g) = (mat?, self.load(name)?);
                Some(VdbPart { src: Source::Grid(g.vox.clone()), lo: g.lo * s, hi: g.hi * s, mat })
            })
            .collect()
    }

    /// Local bounds of everything that loads, if anything does.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let s = self.scale.max(1e-6);
        [self.density_grid.as_str(), self.fire_grid.as_str()]
            .into_iter()
            .filter_map(|n| self.load(n))
            .map(|g| (g.lo * s, g.hi * s))
            .reduce(|(a, b), (c, d)| (a.min(c), b.max(d)))
    }

    /// Why nothing renders, for the rollout ("" when the grids load).
    pub fn problem(&self) -> String {
        if self.path.is_empty() {
            return "No file picked".into();
        }
        for n in [&self.density_grid, &self.fire_grid].into_iter().filter(|n| !n.is_empty()) {
            if let Err(e) = grid(&self.path, n, self.up, self.resolution) {
                return e;
            }
        }
        String::new()
    }

    /// The viewport cage: the twelve edges of the bounds as thin beams (a
    /// 1 m cage while the file is missing).
    pub fn build(&self) -> EditablePoly {
        let (lo, hi) = self.bounds().unwrap_or((Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 1.0)));
        let t = ((hi - lo).length() * 0.004).max(1e-3);
        let mut k = Kit::new();
        let corner = |i: usize| Vec3::new(if i & 1 == 0 { lo.x } else { hi.x }, if i & 2 == 0 { lo.y } else { hi.y }, if i & 4 == 0 { lo.z } else { hi.z });
        for (a, b) in [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)] {
            let (p, q) = (corner(a), corner(b));
            let d = q - p;
            let rot = Quat::from_rotation_arc(Vec3::X, d.normalize_or(Vec3::X));
            k.cuboid((p + q) * 0.5, Vec3::new(d.length() * 0.5 + t * 0.5, t * 0.5, t * 0.5), rot);
        }
        k.finish()
    }
}
