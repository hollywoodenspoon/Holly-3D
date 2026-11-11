//! Render Setup (F10) parameters. Old scenes that chose the removed Cycles
//! bridge still load: the unknown fields are ignored and they render with
//! the native path tracer.

/// Output resolution presets (Common tab).
pub const RESOLUTIONS: [(u32, u32); 5] = [(800, 600), (1280, 720), (1920, 1080), (2048, 2048), (3840, 2160)];

/// Anti-aliasing: samples per pixel (stratified n x n grid).
pub const AA_LEVELS: [(u32, &str); 4] = [(1, "Off (1 sample)"), (2, "2x2 (4 samples)"), (3, "3x3 (9 samples)"), (4, "4x4 (16 samples)")];

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RenderSettings {
    pub width: u32,
    pub height: u32,
    pub lock_aspect: bool,
    /// Transparent background: alpha 0 where no geometry is hit.
    pub transparent: bool,
    /// Viewport whose camera is rendered (`None` = the active viewport).
    pub view: Option<usize>,
    /// Camera object rendered instead of a viewport (Render Setup's View list).
    #[serde(default)]
    pub camera: Option<crate::scene::ObjId>,
    /// Compute device, integrator, bounces and sampling (path tracer).
    pub path: super::pathtrace::PathSettings,
    /// AA grid size (n x n samples per pixel).
    pub aa: u32,
    pub shadows: bool,
    /// Shadow rays per light for soft shadows (lights with Softness > 0).
    pub shadow_samples: u32,
    /// Reflection / refraction bounce limit.
    pub max_depth: u32,
    /// Skylight hemisphere / ambient occlusion rays.
    pub sky_samples: u32,
    /// Universal ink outline post pass.
    pub outlines: bool,
    /// GI & AO rollout (built-in raytracer).
    pub gi: super::post::GiSettings,
    /// Post-FX rollout (bloom, depth of field, tonemapping).
    pub post: super::post::PostFx,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            lock_aspect: false,
            transparent: false,
            view: None,
            camera: None,
            path: Default::default(),
            aa: 2,
            shadows: true,
            shadow_samples: 8,
            max_depth: 4,
            sky_samples: 8,
            outlines: true,
            gi: Default::default(),
            post: Default::default(),
        }
    }
}

impl RenderSettings {
    pub fn aspect(&self) -> f32 {
        self.width.max(1) as f32 / self.height.max(1) as f32
    }

    /// Sets the width, keeping the aspect when locked.
    pub fn set_width(&mut self, w: u32) {
        let a = self.aspect();
        self.width = w.clamp(16, 8192);
        if self.lock_aspect {
            self.height = ((self.width as f32 / a).round() as u32).clamp(16, 8192);
        }
    }

    pub fn set_height(&mut self, h: u32) {
        let a = self.aspect();
        self.height = h.clamp(16, 8192);
        if self.lock_aspect {
            self.width = ((self.height as f32 * a).round() as u32).clamp(16, 8192);
        }
    }
}
