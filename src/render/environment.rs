//! Environment & Effects (`8`): background, ambient tint, exponential fog and
//! the 1-click lighting presets (which place real, editable light objects).

use super::lighting::{Light, LightKind, kelvin_to_rgb};
use bevy::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LightingPreset {
    #[default]
    DefaultHeadlight,
    ThreePoint,
    RimContrast,
    Daylight,
    PitchBlack,
}

impl LightingPreset {
    pub const ALL: [LightingPreset; 5] = [Self::DefaultHeadlight, Self::ThreePoint, Self::RimContrast, Self::Daylight, Self::PitchBlack];
    pub fn label(self) -> &'static str {
        match self {
            Self::DefaultHeadlight => "Default Modeling Headlight",
            Self::ThreePoint => "3-Point Studio (Key/Fill/Rim)",
            Self::RimContrast => "High-Contrast Rim",
            Self::Daylight => "Outdoor Daylight",
            Self::PitchBlack => "Pitch Black (Scene Lights Only)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Environment {
    pub preset: LightingPreset,
    /// Background color (sRGB) — viewports and the F9 render.
    pub background: [f32; 3],
    /// Ambient tint (sRGB) and level.
    pub ambient: [f32; 3],
    pub ambient_level: f32,
    /// When false (Pitch Black), the headlight rig never switches on by itself.
    pub auto_default_lights: bool,
    pub fog: bool,
    pub fog_color: [f32; 3],
    /// Exponential fog density (per world unit).
    pub fog_density: f32,
    /// HDRI background / image-based ambient (`.hdr` / `.exr`).
    #[serde(default)]
    pub hdri: Option<super::hdri::Hdri>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            preset: LightingPreset::DefaultHeadlight,
            background: [0.235, 0.24, 0.255],
            ambient: [1.0; 3],
            ambient_level: 1.0,
            auto_default_lights: true,
            fog: false,
            fog_color: [0.6, 0.65, 0.72],
            fog_density: 0.05,
            hdri: None,
        }
    }
}

/// A light a preset wants in the scene.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PresetLight {
    pub name: &'static str,
    pub light: Light,
    pub position: Vec3,
    /// Target helper position (target lights) — also the aim of free lights.
    pub aim: Vec3,
}

fn light(kind: LightKind, kelvin: f32, mult: f32) -> Light {
    let mut l = Light::new(kind);
    (l.color, l.multiplier, l.from_preset) = (kelvin_to_rgb(kelvin), mult, true);
    l
}

/// Applies a preset to the environment and returns the lights it places,
/// sized to a scene centered at `c` with radius `r`.
pub fn apply_preset(env: &mut Environment, p: LightingPreset, c: Vec3, r: f32) -> Vec<PresetLight> {
    let r = r.max(1.0);
    let at = |x: f32, y: f32, z: f32| c + Vec3::new(x, y, z) * r;
    env.preset = p;
    env.auto_default_lights = p != LightingPreset::PitchBlack;
    let (bg, amb, level) = match p {
        LightingPreset::DefaultHeadlight => ([0.235, 0.24, 0.255], [1.0; 3], 1.0),
        LightingPreset::ThreePoint => ([0.16, 0.165, 0.18], [1.0; 3], 0.35),
        LightingPreset::RimContrast => ([0.03, 0.03, 0.035], [0.7, 0.75, 1.0], 0.12),
        LightingPreset::Daylight => ([0.53, 0.68, 0.86], [0.75, 0.85, 1.0], 0.6),
        LightingPreset::PitchBlack => ([0.0; 3], [1.0; 3], 0.0),
    };
    (env.background, env.ambient, env.ambient_level) = (bg, amb, level);
    match p {
        LightingPreset::DefaultHeadlight | LightingPreset::PitchBlack => vec![],
        LightingPreset::ThreePoint => vec![
            PresetLight { name: "Key Light", light: light(LightKind::TargetSpot, 4800.0, 1.4), position: at(-2.2, -2.6, 2.4), aim: c },
            PresetLight { name: "Fill Light", light: light(LightKind::Omni, 7500.0, 0.45), position: at(2.6, -1.6, 1.0), aim: c },
            PresetLight { name: "Rim Light", light: light(LightKind::TargetDirect, 6500.0, 0.9), position: at(0.6, 2.8, 2.2), aim: c },
        ],
        LightingPreset::RimContrast => vec![
            PresetLight { name: "Rim Left", light: light(LightKind::TargetDirect, 9000.0, 1.3), position: at(-2.0, 2.4, 1.2), aim: c },
            PresetLight { name: "Rim Right", light: light(LightKind::TargetDirect, 3200.0, 1.1), position: at(2.2, 2.0, 1.4), aim: c },
            PresetLight { name: "Key Light", light: light(LightKind::TargetSpot, 5600.0, 0.5), position: at(0.0, -3.0, 2.6), aim: c },
        ],
        LightingPreset::Daylight => {
            let mut sky = light(LightKind::Skylight, 10000.0, 0.8);
            sky.cast_shadows = false;
            vec![
                PresetLight { name: "Sun", light: light(LightKind::TargetDirect, 5600.0, 1.3), position: at(-2.4, -1.6, 3.4), aim: c },
                PresetLight { name: "Sky", light: sky, position: at(0.0, 0.0, 2.0), aim: c },
            ]
        }
    }
}
