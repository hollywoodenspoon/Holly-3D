//! Step D: Smoke Simulation. A grid fluid solver for smoke and fire:
//! hot gas rises (buoyancy), swirls (vorticity confinement keeps the small
//! eddies that make tendrils), burns fuel into heat and soot, drifts with
//! the wind and fades. Each frame is a density and a temperature grid that
//! the volume renderers draw like an imported VDB.
//!
//! The domain is a box standing on the object's origin: centred in X / Y,
//! from 0 up to `size.z`. The source is a sphere near the floor.

pub mod bake;
mod field;
mod object;
mod solver;
#[cfg(test)]
mod tests;

pub use bake::shutdown;
pub use field::{Field, par_fill};
pub use object::SmokeSimParams;
pub use solver::SimState;

/// Simulation settings (the Smoke Simulation rollout).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SimParams {
    /// Domain size in metres (X, Y, Z).
    pub size: [f32; 3],
    /// Cells along the longest side.
    pub resolution: u32,
    /// Source sphere radius (m) and the height of its centre (m).
    pub source_radius: f32,
    pub source_height: f32,
    /// Upward speed of the gas leaving the source (m/s).
    pub source_speed: f32,
    /// Smoke density and temperature written at the source.
    pub smoke: f32,
    pub heat: f32,
    /// Fuel emitted per second (0 = smoke only, no flames).
    pub fuel: f32,
    /// Lift per unit of temperature, and sinking per unit of smoke (m/s²).
    pub buoyancy: f32,
    pub weight: f32,
    /// Swirl: vorticity confinement strength.
    pub swirl: f32,
    /// Turbulence of the source (noise on its smoke and speed).
    pub turbulence: f32,
    /// Smoke and heat fading per second.
    pub dissipation: f32,
    pub cooling: f32,
    /// Fuel burnt per second where it is hot enough, the temperature it
    /// needs, the heat each unit releases and the soot it leaves.
    pub burn_rate: f32,
    pub ignition: f32,
    pub heat_release: f32,
    pub soot: f32,
    /// Wind (m/s, X and Y).
    pub wind: [f32; 2],
    /// Frames per second and frames simulated.
    pub fps: f32,
    pub frames: u32,
    /// Pressure solver iterations (more = less compressible, slower).
    pub pressure_iters: u32,
    pub seed: u32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            size: [4.0, 4.0, 8.0],
            resolution: 96,
            source_radius: 0.5,
            source_height: 0.5,
            source_speed: 1.5,
            smoke: 1.0,
            heat: 1.0,
            fuel: 0.0,
            buoyancy: 1.5,
            weight: 0.1,
            swirl: 0.35,
            turbulence: 0.5,
            dissipation: 0.05,
            cooling: 0.6,
            burn_rate: 2.0,
            ignition: 0.3,
            heat_release: 3.0,
            soot: 0.6,
            wind: [0.0, 0.0],
            fps: 24.0,
            frames: 96,
            pressure_iters: 40,
            seed: 1,
        }
    }
}

impl SimParams {
    /// Cell size (m) and cell counts of the grid.
    pub fn grid(&self) -> (f32, [usize; 3]) {
        let s = self.size.map(|v| v.max(0.05));
        let h = s.iter().fold(0.0f32, |m, v| m.max(*v)) / self.resolution.clamp(8, 512) as f32;
        (h, s.map(|v| ((v / h).round() as usize).max(4)))
    }
}
