//! GPU selection: Vulkan (Mesa RADV on Radeon RX 9000 cards) on the
//! high-performance adapter. `WGPU_BACKEND` / `WGPU_POWER_PREF` still override.

use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, PowerPreference, RenderCreation, WgpuSettings};

/// Backends to request: `WGPU_BACKEND` if set, otherwise Vulkan only.
pub fn preferred_backends() -> Backends {
    Backends::from_env().unwrap_or(Backends::VULKAN)
}

pub fn wgpu_settings() -> WgpuSettings {
    WgpuSettings {
        backends: Some(preferred_backends()),
        power_preference: PowerPreference::from_env().unwrap_or(PowerPreference::HighPerformance),
        ..Default::default()
    }
}

pub fn render_plugin() -> RenderPlugin {
    RenderPlugin { render_creation: RenderCreation::Automatic(Box::new(wgpu_settings())), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_vulkan_high_performance() {
        if std::env::var_os("WGPU_BACKEND").is_none() && std::env::var_os("WGPU_POWER_PREF").is_none() {
            let s = wgpu_settings();
            assert_eq!(s.backends, Some(Backends::VULKAN));
            assert_eq!(s.power_preference, PowerPreference::HighPerformance);
        }
    }
}
