//! The Volume shading model's parameters: what a cloud, haze, smoke or fire
//! volume is made of. Any object with a Volume material renders as a
//! participating medium instead of a surface (Clouds from their puffs,
//! other objects as a box filling their bounds).

/// Temperature -> colour ramp of a volume's glow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EmissionRamp {
    #[default]
    None,
    /// Physical blackbody colours (dull red -> orange -> white).
    Blackbody,
    /// A stylised fire ramp: deep red embers to yellow-white cores.
    Fire,
}

impl EmissionRamp {
    pub const ALL: [Self; 3] = [Self::None, Self::Blackbody, Self::Fire];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Blackbody => "Blackbody",
            Self::Fire => "Fire",
        }
    }

    /// Linear colour at normalised temperature `t` (0 cold .. 1 hottest).
    pub fn color(self, t: f32) -> bevy::math::Vec3 {
        use bevy::math::Vec3;
        let t = t.clamp(0.0, 1.0);
        let lerp3 = |stops: &[(f32, Vec3)]| {
            let i = stops.iter().rposition(|s| s.0 <= t).unwrap_or(0).min(stops.len() - 2);
            let (a, b) = (stops[i], stops[i + 1]);
            a.1.lerp(b.1, ((t - a.0) / (b.0 - a.0).max(1e-6)).clamp(0.0, 1.0))
        };
        match self {
            Self::None => Vec3::ZERO,
            Self::Blackbody => lerp3(&[
                (0.0, Vec3::ZERO),
                (0.25, Vec3::new(0.35, 0.02, 0.0)),
                (0.5, Vec3::new(1.0, 0.18, 0.01)),
                (0.75, Vec3::new(1.0, 0.55, 0.15)),
                (1.0, Vec3::new(1.0, 0.92, 0.8)),
            ]),
            Self::Fire => lerp3(&[
                (0.0, Vec3::ZERO),
                (0.3, Vec3::new(0.6, 0.04, 0.0)),
                (0.6, Vec3::new(1.0, 0.35, 0.02)),
                (1.0, Vec3::new(1.0, 0.85, 0.35)),
            ]),
        }
    }
}

/// Volume material parameters (the Material Editor's Volume rollout).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct VolumeMat {
    /// Scatter Colour (sRGB): the colour light picks up when it bounces.
    pub scatter: [f32; 3],
    /// Absorption 0..1: 0 = a cloud (scatters everything), 1 = black smoke.
    pub absorption: f32,
    /// Density: extinction per metre where the shape is fully dense.
    pub density: f32,
    /// Two-lobe phase: Forward g (silver lining), Back g, and the Blend
    /// (share of the back lobe).
    pub g_forward: f32,
    pub g_back: f32,
    pub blend: f32,
    /// Emission: temperature ramp, strength and Temperature Cooling (how
    /// quickly the glow fades toward thin edges).
    pub ramp: EmissionRamp,
    pub emission: f32,
    pub cooling: f32,
}

impl Default for VolumeMat {
    fn default() -> Self {
        VolumePreset::CumulusCloud.params()
    }
}

/// One-click volume looks (Material Editor preset dropdown).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolumePreset {
    CumulusCloud,
    StormCloud,
    ThinHaze,
    BlackSmoke,
    GreySmoke,
    Fire,
    Fireball,
}

impl VolumePreset {
    pub const ALL: [Self; 7] = [Self::CumulusCloud, Self::StormCloud, Self::ThinHaze, Self::BlackSmoke, Self::GreySmoke, Self::Fire, Self::Fireball];
    pub fn label(self) -> &'static str {
        match self {
            Self::CumulusCloud => "Cumulus Cloud",
            Self::StormCloud => "Storm Cloud",
            Self::ThinHaze => "Thin Haze",
            Self::BlackSmoke => "Black Smoke",
            Self::GreySmoke => "Grey Smoke",
            Self::Fire => "Fire",
            Self::Fireball => "Fireball",
        }
    }
    pub fn params(self) -> VolumeMat {
        let base = VolumeMat {
            scatter: [1.0, 1.0, 1.0],
            absorption: 0.0,
            density: 0.05,
            g_forward: 0.8,
            g_back: -0.3,
            blend: 0.2,
            ramp: EmissionRamp::None,
            emission: 0.0,
            cooling: 0.5,
        };
        match self {
            Self::CumulusCloud => base,
            Self::StormCloud => VolumeMat { scatter: [0.9, 0.92, 0.95], absorption: 0.02, density: 0.12, ..base },
            Self::ThinHaze => VolumeMat { scatter: [0.85, 0.9, 1.0], density: 0.005, g_forward: 0.6, blend: 0.1, ..base },
            Self::BlackSmoke => VolumeMat { scatter: [0.3, 0.3, 0.3], absorption: 0.85, density: 1.5, g_forward: 0.4, ..base },
            Self::GreySmoke => VolumeMat { scatter: [0.75, 0.75, 0.75], absorption: 0.3, density: 0.8, g_forward: 0.5, ..base },
            Self::Fire => VolumeMat { scatter: [0.4, 0.35, 0.3], absorption: 0.7, density: 0.6, ramp: EmissionRamp::Fire, emission: 6.0, cooling: 0.4, ..base },
            Self::Fireball => {
                VolumeMat { scatter: [0.5, 0.45, 0.4], absorption: 0.5, density: 1.0, ramp: EmissionRamp::Blackbody, emission: 12.0, cooling: 0.25, ..base }
            }
        }
    }
}

impl VolumeMat {
    /// Single-scattering albedo (linear).
    pub fn albedo(&self) -> bevy::math::Vec3 {
        super::shading::lin3(self.scatter) * (1.0 - self.absorption.clamp(0.0, 1.0))
    }
    /// The two-lobe phase function value.
    pub fn phase(&self, cos: f32) -> f32 {
        use crate::render::volumetrics::hg_phase;
        let b = self.blend.clamp(0.0, 1.0);
        hg_phase(cos, self.g_forward) * (1.0 - b) + hg_phase(cos, self.g_back) * b
    }
    /// Glow radiance at local density `d` (0..1): hotter where denser,
    /// fading toward thin edges by the cooling.
    pub fn glow(&self, d: f32) -> bevy::math::Vec3 {
        if self.ramp == EmissionRamp::None || self.emission <= 0.0 {
            return bevy::math::Vec3::ZERO;
        }
        let t = d.clamp(0.0, 1.0).powf(self.cooling.clamp(0.0, 1.0) * 2.0 + 0.25);
        self.ramp.color(t) * self.emission
    }
}
