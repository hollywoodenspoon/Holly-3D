//! The Smoke Simulation scene object: simulation settings, the frame to
//! show, and how the baked grids look. The scene stores only settings;
//! frames come from the background bake (`bake.rs`) and render as voxel
//! volumes like an imported VDB. In the viewport the object is the
//! simulation domain drawn as a thin cage.

use super::SimParams;
use super::bake::{Bake, bake_of};
use crate::generators::vdb::{VdbPart, VdbStyle, cage};
use crate::materials::volume::{EmissionRamp, VolumeMat, VolumePreset};
use crate::mesh::EditablePoly;
use crate::render::volumes::density::Source;
use bevy::math::Vec3;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SmokeSimParams {
    pub sim: SimParams,
    /// Frame shown in the viewport and rendered (0-based).
    pub frame: u32,
    pub style: VdbStyle,
    /// Smoke Density: extinction per metre at a density of 1.
    pub density: f32,
    /// Fire Intensity 0..=100.
    pub fire: f32,
}

impl Default for SmokeSimParams {
    fn default() -> Self {
        Self { sim: SimParams::default(), frame: 72, style: VdbStyle::GreySmoke, density: 3.0, fire: 60.0 }
    }
}

impl SmokeSimParams {
    /// A fire: fuel burning at the source, dark soot above.
    pub fn fire_preset() -> Self {
        let sim = SimParams { fuel: 3.0, smoke: 0.3, heat: 1.2, buoyancy: 2.0, cooling: 2.5, swirl: 0.5, ..SimParams::default() };
        Self { sim, style: VdbStyle::BlackSmoke, ..Self::default() }
    }

    /// Local bounds of the simulation domain.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let (h, n) = self.sim.grid();
        let e = Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32) * h;
        (Vec3::new(-0.5 * e.x, -0.5 * e.y, 0.0), Vec3::new(0.5 * e.x, 0.5 * e.y, e.z))
    }

    pub fn bake(&self) -> Arc<Bake> {
        bake_of(&self.sim)
    }

    pub fn smoke_mat(&self) -> Option<VolumeMat> {
        let d = self.density.max(0.0);
        (d > 0.0 && self.sim.smoke + self.sim.soot * self.sim.fuel > 0.0).then(|| VolumeMat { density: d, ..self.style.mat() })
    }

    pub fn fire_mat(&self) -> Option<VolumeMat> {
        let f = (self.fire / 100.0).clamp(0.0, 1.0);
        (self.sim.fuel > 0.0 && f > 0.0).then(|| VolumeMat { ramp: EmissionRamp::Fire, emission: 4.0 + 8.0 * f, density: 1.2, ..VolumePreset::Fire.params() })
    }

    /// The smoke and fire media of the shown frame (none until the bake
    /// has a frame; a frame not baked yet shows the latest that is).
    pub fn parts(&self, own: Option<VolumeMat>) -> Vec<VdbPart> {
        let Some((_, f)) = self.bake().frame(self.frame) else { return Vec::new() };
        let (lo, hi) = self.bounds();
        let media = [(self.smoke_mat().map(|m| own.unwrap_or(m)), f.smoke.clone()), (self.fire_mat(), f.fire.clone())];
        media.into_iter().filter_map(|(m, v)| Some(VdbPart { src: Source::Grid(v), lo, hi, mat: m? })).collect()
    }

    /// Which baked frame is shown (+1; 0 while none is), so previews know
    /// when to refresh.
    pub fn stamp(&self) -> u64 {
        self.bake().frame(self.frame).map_or(0, |(k, _)| u64::from(k) + 1)
    }

    /// Bake progress for the rollout.
    pub fn status(&self) -> String {
        let b = self.bake();
        let (r, t) = (b.ready(), b.total);
        let shown = b.frame(self.frame).map_or("nothing yet".to_string(), |(k, _)| format!("frame {}", k + 1));
        let on = match b.solver() {
            "" => String::new(),
            s => format!(" on the {s}"),
        };
        if r >= t { format!("Baked {t} frames{on}; showing {shown}") } else { format!("Baking {r} / {t} frames{on}; showing {shown}") }
    }

    /// Memory one bake takes (MB).
    pub fn memory_mb(&self) -> f32 {
        let (_, n) = self.sim.grid();
        (n[0] * n[1] * n[2]) as f32 * 8.0 * self.sim.frames as f32 / 1_048_576.0
    }

    pub fn build(&self) -> EditablePoly {
        let (lo, hi) = self.bounds();
        cage(lo, hi)
    }
}
