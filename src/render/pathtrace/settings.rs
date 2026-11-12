//! F10 path-tracing parameters: compute device, bounce limits, Russian
//! Roulette, progressive passes and adaptive (noise-threshold) sampling.

/// Which processor runs the F9 render.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ComputeDevice {
    /// The GPU when a wgpu adapter is found, else the CPU.
    #[default]
    Auto,
    /// wgpu compute (Vulkan / Metal / DX12); falls back to the CPU if none.
    Gpu,
    /// Multi-threaded CPU tracer (4-wide BVH).
    Cpu,
}

impl ComputeDevice {
    pub const ALL: [ComputeDevice; 3] = [ComputeDevice::Auto, ComputeDevice::Gpu, ComputeDevice::Cpu];
    pub fn label(self) -> &'static str {
        match self {
            ComputeDevice::Auto => "Auto (wgpu GPU with CPU fallback)",
            ComputeDevice::Gpu => "GPU (wgpu Vulkan Compute)",
            ComputeDevice::Cpu => "CPU (SIMD Multi-Thread)",
        }
    }
}

/// How light is gathered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Integrator {
    /// Unbiased multi-bounce path tracing with MIS (default).
    #[default]
    PathTracer,
    /// The fast classic raytracer (1-bounce GI, AO, fixed AA grid).
    Classic,
}

impl Integrator {
    pub const ALL: [Integrator; 2] = [Integrator::PathTracer, Integrator::Classic];
    pub fn label(self) -> &'static str {
        match self {
            Integrator::PathTracer => "Veach Path Tracer (multi-bounce, MIS)",
            Integrator::Classic => "Classic Raytracer (fast preview)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PathSettings {
    pub device: ComputeDevice,
    pub integrator: Integrator,
    /// Target samples per pixel.
    pub samples: u32,
    /// Total path length limit (1..=8 bounces after the camera hit).
    pub max_bounces: u32,
    pub diffuse_bounces: u32,
    pub glossy_bounces: u32,
    pub transmission_bounces: u32,
    /// Russian Roulette starts after this many bounces.
    pub rr_start: u32,
    /// Stream full-frame passes (1 -> 2 -> 4 -> ... -> target spp).
    pub progressive: bool,
    /// Per-pixel adaptive sampling (stop converged pixels early).
    pub adaptive: bool,
    /// Relative error below which a pixel's 3x3 neighbourhood is done.
    pub noise_threshold: f32,
    /// Samples every pixel takes before it may stop.
    pub min_samples: u32,
    /// Keep one buffer per light group for the Frame Window's LightMix.
    pub lightmix: bool,
}

impl Default for PathSettings {
    fn default() -> Self {
        Self {
            device: ComputeDevice::Auto,
            integrator: Integrator::PathTracer,
            samples: 64,
            max_bounces: 6,
            diffuse_bounces: 4,
            glossy_bounces: 4,
            transmission_bounces: 8,
            rr_start: 2,
            progressive: true,
            adaptive: true,
            noise_threshold: 0.02,
            min_samples: 8,
            lightmix: true,
        }
    }
}

impl PathSettings {
    pub const BOUNCES: std::ops::RangeInclusive<u32> = 1..=8;
    pub const THRESHOLD: std::ops::RangeInclusive<f32> = 0.002..=0.10;
    pub const MIN_SAMPLES: std::ops::RangeInclusive<u32> = 4..=32;

    /// Samples per pixel after each progressive pass: 1, 2, 4, 8, ... target.
    pub fn schedule(&self) -> Vec<u32> {
        let target = self.samples.max(1);
        if !self.progressive {
            return vec![target];
        }
        let mut v = vec![];
        let mut n = 1;
        while n < target {
            v.push(n);
            n *= 2;
        }
        v.push(target);
        v
    }
}
