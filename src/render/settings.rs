//! Render Setup (F10) parameters shared by both renderer engines.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Engine {
    /// Built-in high-quality CPU raytracer (default).
    #[default]
    Raytracer,
    /// Exports a Cycles XML scene and runs the standalone `cycles` CLI.
    Cycles,
}

impl Engine {
    pub const ALL: [Engine; 2] = [Engine::Raytracer, Engine::Cycles];
    pub fn label(self) -> &'static str {
        match self {
            Engine::Raytracer => "Built-in High-Quality Raytracer",
            Engine::Cycles => "Cycles Bridge (Blender / Standalone)",
        }
    }
}

/// How the Cycles engine is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CyclesVia {
    /// `blender --background -E CYCLES` with a Python scene loader (default).
    #[default]
    Blender,
    /// The standalone `cycles` CLI reading Cycles XML.
    Standalone,
}

impl CyclesVia {
    pub const ALL: [CyclesVia; 2] = [CyclesVia::Blender, CyclesVia::Standalone];
    pub fn label(self) -> &'static str {
        match self {
            CyclesVia::Blender => "Blender (background)",
            CyclesVia::Standalone => "Standalone cycles CLI",
        }
    }
}

/// Cycles compute device (Blender's Cycles device names).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CyclesDevice {
    /// AMD GPUs (HIP, optionally HIP-RT hardware ray tracing).
    #[default]
    Hip,
    Optix,
    Cuda,
    OneApi,
    Metal,
    Cpu,
}

impl CyclesDevice {
    pub const ALL: [CyclesDevice; 6] = [Self::Hip, Self::Optix, Self::Cuda, Self::OneApi, Self::Metal, Self::Cpu];
    /// Device type name as Blender / the cycles CLI spell it.
    pub fn id(self) -> &'static str {
        match self {
            Self::Hip => "HIP",
            Self::Optix => "OPTIX",
            Self::Cuda => "CUDA",
            Self::OneApi => "ONEAPI",
            Self::Metal => "METAL",
            Self::Cpu => "CPU",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Hip => "HIP (AMD)",
            Self::Optix => "OptiX (NVIDIA RTX)",
            Self::Cuda => "CUDA (NVIDIA)",
            Self::OneApi => "oneAPI (Intel Arc)",
            Self::Metal => "Metal (Apple)",
            Self::Cpu => "CPU",
        }
    }
}

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
    pub engine: Engine,
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
    pub cycles_exe: String,
    pub cycles_samples: u32,
    pub cycles_via: CyclesVia,
    pub blender_exe: String,
    pub cycles_device: CyclesDevice,
    /// HIP-RT hardware ray tracing (HIP device only).
    pub hiprt: bool,
    /// OpenImageDenoise on the final image.
    pub denoise: bool,
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
            engine: Engine::Raytracer,
            aa: 2,
            shadows: true,
            shadow_samples: 8,
            max_depth: 4,
            sky_samples: 8,
            outlines: true,
            cycles_exe: "cycles".into(),
            cycles_samples: 64,
            cycles_via: CyclesVia::Blender,
            blender_exe: if cfg!(windows) { "blender.exe".into() } else { "/usr/bin/blender".into() },
            cycles_device: CyclesDevice::Hip,
            hiprt: true,
            denoise: true,
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
