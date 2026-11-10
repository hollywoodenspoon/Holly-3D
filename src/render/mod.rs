//! Lighting, environment and the offline renderers (F9 / F10): the
//! built-in CPU raytracer and the Cycles standalone bridge.

pub mod bvh;
pub mod cycles;
pub mod environment;
pub mod framebuffer;
pub mod hdri;
pub mod job;
pub mod lighting;
pub mod outline;
pub mod scene_build;
pub mod settings;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_render;
pub mod trace;

pub use environment::{Environment, LightingPreset};
pub use framebuffer::{ChannelView, FrameBuffer};
pub use job::RenderJob;
pub use lighting::{Light, LightKind};
pub use settings::{Engine, RenderSettings};
