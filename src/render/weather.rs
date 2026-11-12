//! PR 8: dynamic weather & storm clouds (`8` -> Dynamic Sky -> Weather).
//! A one-click Weather Preset sets the clouds, fog and precipitation;
//! Cloud Thickness (optical depth) darkens the cloud bottoms by
//! Beer-Lambert extinction, Overcast Sun Dimming attenuates and softens
//! the linked sun through the deck, Thunderstorm adds lightning, and the
//! precipitation (rain streaks, snowflakes, ash with glowing cinders) is
//! drawn in the viewport and as a depth-aware F9 overlay
//! ([`super::precip`]).

use super::Environment;
use super::sky::{SkyMode, SkySettings};
use super::sky_noise::{hash3, smooth};
use bevy::math::{Vec2, Vec3};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Precip {
    #[default]
    None,
    Rain,
    Snow,
    /// Falling ash with a share of glowing orange cinders.
    Ash,
}

impl Precip {
    pub const ALL: [Precip; 4] = [Self::None, Self::Rain, Self::Snow, Self::Ash];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Rain => "Rain Streaks",
            Self::Snow => "Snowflakes",
            Self::Ash => "Ash & Glowing Cinders",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum WeatherPreset {
    #[default]
    Clear,
    PartlyCloudy,
    Overcast,
    GatheringStorm,
    Thunderstorm,
    AshHaze,
    Fog,
    Rainstorm,
    Blizzard,
}

impl WeatherPreset {
    pub const ALL: [WeatherPreset; 9] = [
        Self::Clear,
        Self::PartlyCloudy,
        Self::Overcast,
        Self::GatheringStorm,
        Self::Thunderstorm,
        Self::AshHaze,
        Self::Fog,
        Self::Rainstorm,
        Self::Blizzard,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Clear => "Clear Sky",
            Self::PartlyCloudy => "Partly Cloudy",
            Self::Overcast => "Heavy Overcast",
            Self::GatheringStorm => "Dark Gathering Storm",
            Self::Thunderstorm => "Thunderstorm",
            Self::AshHaze => "Battlefield Ash & Smoke Haze",
            Self::Fog => "Heavy Fog / Mist",
            Self::Rainstorm => "Rainstorm",
            Self::Blizzard => "Blizzard / Snow",
        }
    }

    /// (coverage, density, altitude km, thickness, precip, amount, lightning,
    /// fog density, fog color).
    #[allow(clippy::type_complexity)]
    fn table(self) -> (f32, f32, f32, f32, Precip, f32, bool, f32, [f32; 3]) {
        let grey = [0.6, 0.65, 0.72];
        match self {
            Self::Clear => (0.0, 0.8, 2.0, 0.0, Precip::None, 0.0, false, 0.0, grey),
            Self::PartlyCloudy => (0.4, 0.8, 2.0, 0.2, Precip::None, 0.0, false, 0.0, grey),
            Self::Overcast => (0.95, 0.95, 1.5, 0.55, Precip::None, 0.0, false, 0.004, [0.62, 0.64, 0.68]),
            Self::GatheringStorm => (0.85, 1.0, 1.2, 0.85, Precip::None, 0.0, false, 0.006, [0.35, 0.38, 0.42]),
            Self::Thunderstorm => (1.0, 1.0, 1.0, 1.0, Precip::Rain, 0.7, true, 0.01, [0.3, 0.33, 0.38]),
            Self::AshHaze => (0.8, 0.9, 1.5, 0.7, Precip::Ash, 0.5, false, 0.02, [0.42, 0.36, 0.3]),
            Self::Fog => (0.7, 0.7, 0.8, 0.4, Precip::None, 0.0, false, 0.08, [0.72, 0.74, 0.77]),
            Self::Rainstorm => (1.0, 1.0, 1.0, 0.8, Precip::Rain, 1.0, false, 0.012, [0.45, 0.48, 0.52]),
            Self::Blizzard => (1.0, 0.95, 1.2, 0.6, Precip::Snow, 1.0, false, 0.03, [0.82, 0.85, 0.9]),
        }
    }

    /// Sets the clouds, weather and fog (switching the sky to Earth mode).
    pub fn apply(self, env: &mut Environment) {
        let (cov, den, alt, thick, precip, amount, lightning, fog, fog_color) = self.table();
        let s = &mut env.sky;
        s.mode = SkyMode::Earth;
        s.clouds.enabled = cov > 0.0;
        (s.clouds.coverage, s.clouds.density, s.clouds.altitude) = (cov, den, alt);
        let w = &mut s.weather;
        (w.preset, w.thickness, w.precip, w.amount, w.lightning) = (self, thick, precip, amount, lightning);
        w.sun_dimming = cov > 0.5;
        env.fog = fog > 0.0;
        if env.fog {
            (env.fog_density, env.fog_color) = (fog, fog_color);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Weather {
    /// The last applied preset (shown in the dropdown).
    pub preset: WeatherPreset,
    /// Cloud Thickness / Optical Depth 0..=1 (0 = thin white, 1 = black storm base).
    pub thickness: f32,
    /// `[✓] Overcast Sun Dimming`: the deck attenuates and softens the sun.
    pub sun_dimming: bool,
    pub precip: Precip,
    /// Precipitation amount 0..=1.
    pub amount: f32,
    /// Fall slant in degrees from vertical (wind-driven rain / snow).
    pub slant: f32,
    /// `[✓] Lightning Flash` (Thunderstorm).
    pub lightning: bool,
    /// Picks the lightning bolt and the particle layout.
    pub seed: u32,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            preset: WeatherPreset::Clear,
            thickness: 0.2,
            sun_dimming: false,
            precip: Precip::None,
            amount: 0.6,
            slant: 12.0,
            lightning: false,
            seed: 1,
        }
    }
}

/// Optical depth at Cloud Thickness 1.
const MAX_TAU: f32 = 4.5;

impl Weather {
    pub fn tau(&self) -> f32 {
        self.thickness.clamp(0.0, 1.0) * MAX_TAU
    }

    /// Beer-Lambert light reaching the cloud base through `n` (0..1 local
    /// noise density) of the deck: `exp(-tau * depth)`.
    pub fn base_transmittance(&self, n: f32) -> f32 {
        (-self.tau() * (0.35 + 0.65 * n.clamp(0.0, 1.0))).exp()
    }

    /// Storm tint of a cloud color: bottoms darken toward slate blue-grey.
    pub fn shade_cloud(&self, col: Vec3, n: f32, view_up: f32) -> Vec3 {
        if self.thickness <= 0.0 {
            return col;
        }
        // Looking up at the base sees the most attenuated light.
        let t = self.base_transmittance(n).powf(0.6 + 0.8 * view_up.clamp(0.0, 1.0));
        let slate = Vec3::new(0.16, 0.18, 0.22);
        col * t + slate * col.max_element().min(1.0) * (1.0 - t) * 0.35
    }

    /// Unit direction of the current lightning bolt's top (in the clouds).
    fn bolt_anchor(&self) -> Vec2 {
        let s = self.seed as i32;
        Vec2::new(hash3(s, 7, 1) * TAU, 0.18 + 0.2 * hash3(s, 3, 9))
    }

    /// Lightning glow in sky direction `d`: a jagged branching bolt below
    /// the deck plus a broad flash lighting the clouds around it.
    pub fn lightning(&self, d: Vec3) -> Vec3 {
        if !self.lightning || d.z <= -0.02 {
            return Vec3::ZERO;
        }
        let a = self.bolt_anchor();
        let az = d.y.atan2(d.x);
        let el = d.z.clamp(-1.0, 1.0).asin();
        let daz = (az - a.x + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
        // Broad flash around the bolt top.
        let flash = (-(daz * daz + (el - a.y) * (el - a.y)) * 18.0).exp() * 1.6;
        // The bolt: an elevation-indexed zig-zag from the base to the horizon.
        let mut glow = 0.0f32;
        if (0.0..a.y).contains(&el) {
            let steps = 9;
            let u = 1.0 - el / a.y;
            let k = (u * steps as f32).floor() as i32;
            let f = u * steps as f32 - k as f32;
            let off = |i: i32| (hash3(self.seed as i32, i, 5) - 0.5) * 0.05 * (i as f32 * 0.3 + 1.0);
            let x = off(k) + (off(k + 1) - off(k)) * f;
            let dist = (daz - x).abs() * el.cos();
            glow = (1.0 - dist / 0.0035).max(0.0) * 40.0 + (1.0 - dist / 0.02).max(0.0) * 1.5;
        }
        Vec3::new(0.75, 0.82, 1.0) * (flash + glow)
    }

    /// Sun / moon multiplier through the cloud deck (1 = clear).
    pub fn sun_transmittance(&self, sky: &SkySettings) -> f32 {
        let c = &sky.clouds;
        if !self.sun_dimming || !c.enabled {
            return 1.0;
        }
        let cover = smooth(0.1, 0.9, c.coverage) * c.density.clamp(0.0, 1.0);
        let t = (-(self.tau() + 0.6) * cover).exp();
        1.0 - cover + cover * t.max(0.06)
    }

    /// Shadow softness the dimmed sun should get (0 = crisp).
    pub fn sun_softness(&self, sky: &SkySettings) -> f32 {
        if !self.sun_dimming || !sky.clouds.enabled {
            return 0.0;
        }
        (1.0 - self.sun_transmittance(sky)) * 1.5
    }
}
