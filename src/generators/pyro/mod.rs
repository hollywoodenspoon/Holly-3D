//! Clouds PR 4: explosions and the mushroom cloud (Create -> Demiurge Pyro,
//! and the Demiurge tab).
//!
//! A pyro object is a cluster of soft puffs at one moment of the blast: the
//! Stage slider (0..100 %) sets that moment, which drives size, rise, how
//! much fire has cooled into smoke, and darkening. Puffs belong to one of
//! five media (fire, smoke, dust, vapour, sparks) that render as separate
//! volumes, so the fire glows with its own ramp. Sparks are tiny glowing
//! streaks along seeded ballistic arcs; burning debris leaves smoke trails
//! with a glowing tip. An automatic explosion light sits at the heat-weighted
//! centre of the fire.

mod layout;
mod mushroom;
mod sparks;
#[cfg(test)]
mod tests;

pub use layout::{Medium, PyroPuff};

use super::clouds::CloudLook;
use crate::materials::volume::{EmissionRamp, VolumeMat, VolumePreset};
use crate::mesh::EditablePoly;
use bevy::math::Vec3;

/// The explosion types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PyroKind {
    #[default]
    Fireball,
    GroundExplosion,
    FlakBurst,
    AerialExplosion,
    Napalm,
    Mushroom,
}

impl PyroKind {
    pub const ALL: [Self; 6] = [Self::Fireball, Self::GroundExplosion, Self::FlakBurst, Self::AerialExplosion, Self::Napalm, Self::Mushroom];
    pub fn label(self) -> &'static str {
        match self {
            Self::Fireball => "Fireball",
            Self::GroundExplosion => "Ground Explosion",
            Self::FlakBurst => "Flak Burst",
            Self::AerialExplosion => "Aerial Explosion",
            Self::Napalm => "Napalm / Fuel Fire",
            Self::Mushroom => "Nuclear Mushroom Cloud",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Self::Fireball => "A rising ball of fire that cools into rolling black smoke",
            Self::GroundExplosion => "A dome of fire on the ground with a dust skirt, debris and sparks",
            Self::FlakBurst => "A short flash in the air, then a ragged black puff",
            Self::AerialExplosion => "A big fireball in the air with showers of sparks and burning debris",
            Self::Napalm => "A long rolling wall of fire under thick black smoke",
            Self::Mushroom => "Ground fireball, stem, rolling cap with a condensation ring, ice cap and base surge",
        }
    }
    /// Seconds the Stage slider spans.
    pub fn duration(self) -> f32 {
        match self {
            Self::FlakBurst => 3.0,
            Self::Fireball => 4.0,
            Self::AerialExplosion => 5.0,
            Self::GroundExplosion => 6.0,
            Self::Napalm => 10.0,
            Self::Mushroom => 60.0,
        }
    }
}

/// Spark glow colours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SparkColor {
    #[default]
    Orange,
    WhiteGold,
}

impl SparkColor {
    pub const ALL: [Self; 2] = [Self::Orange, Self::WhiteGold];
    pub fn label(self) -> &'static str {
        match self {
            Self::Orange => "Orange",
            Self::WhiteGold => "White-Gold",
        }
    }
}

/// One pyro object's parameters (the Modify panel "Pyro" rollout).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PyroParams {
    pub kind: PyroKind,
    pub seed: u32,
    /// Stage 0..=1: the moment of the blast shown.
    pub stage: f32,
    /// Size (m): the main fireball's radius when grown.
    pub size: f32,
    /// Billow 0..=1: how lumpy the fire and smoke are.
    pub billow: f32,
    /// Heat 0..=2: glow strength.
    pub heat: f32,
    /// Smoke Darkness 0..=1 (grey to black).
    pub darkness: f32,
    /// Sparks: count, launch speed (m/s), Speed Spread 0..=1, Streak Length
    /// (multiplier), colour and Cooling 0..=1.
    pub sparks: u32,
    pub spark_speed: f32,
    pub speed_spread: f32,
    pub streak: f32,
    pub spark_color: SparkColor,
    pub spark_cooling: f32,
    /// Burning debris pieces flung out on arcs.
    pub debris: u32,
    /// Explosion light at the fire's centre, and its strength multiplier.
    pub light: bool,
    pub light_strength: f32,
    pub look: CloudLook,
}

impl Default for PyroParams {
    fn default() -> Self {
        Self::of(PyroKind::Fireball)
    }
}

impl PyroParams {
    /// The preset for `kind` (Type dropdown).
    pub fn of(kind: PyroKind) -> Self {
        let base = Self {
            kind,
            seed: 1,
            stage: 0.35,
            size: 5.0,
            billow: 0.6,
            heat: 1.0,
            darkness: 0.8,
            sparks: 0,
            spark_speed: 25.0,
            speed_spread: 0.5,
            streak: 1.0,
            spark_color: SparkColor::Orange,
            spark_cooling: 0.5,
            debris: 0,
            light: true,
            light_strength: 1.0,
            look: CloudLook { erosion: 0.55, wispiness: 0.1, edge_softness: 0.5, ..CloudLook::default() },
        };
        match kind {
            PyroKind::Fireball => base,
            PyroKind::GroundExplosion => Self { size: 6.0, sparks: 400, debris: 10, ..base },
            PyroKind::FlakBurst => Self { size: 2.5, stage: 0.4, billow: 0.9, darkness: 1.0, sparks: 150, spark_speed: 40.0, ..base },
            PyroKind::AerialExplosion => Self { size: 8.0, stage: 0.25, sparks: 1200, spark_speed: 45.0, debris: 16, spark_color: SparkColor::WhiteGold, ..base },
            PyroKind::Napalm => Self { size: 4.0, stage: 0.5, darkness: 1.0, ..base },
            PyroKind::Mushroom => Self { size: 120.0, stage: 0.6, billow: 0.7, darkness: 0.6, spark_speed: 0.0, light_strength: 1.0, ..base },
        }
    }

    /// Seconds into the blast at the current Stage.
    pub fn time(&self) -> f32 {
        self.stage.clamp(0.0, 1.0) * self.kind.duration()
    }

    /// Every puff (all media).
    pub fn puffs(&self) -> Vec<PyroPuff> {
        layout::layout(self)
    }

    /// The preview mesh: one sphere per puff, Material ID by medium
    /// (Smoke & Fire pack: 1 fire, 2 smoke).
    pub fn build(&self) -> EditablePoly {
        layout::mesh(&self.puffs())
    }

    /// The medium's volume material (`None` when it doesn't render).
    pub fn mat(&self, m: Medium) -> Option<VolumeMat> {
        let s = self.stage.clamp(0.0, 1.0);
        let mat = match m {
            Medium::Fire => {
                let fade = 1.0 - 0.6 * s;
                VolumeMat { emission: 14.0 * self.heat.max(0.0) * fade, ..VolumePreset::Fireball.params() }
            }
            Medium::Smoke => {
                let (g, b) = (VolumePreset::GreySmoke.params(), VolumePreset::BlackSmoke.params());
                // Smoke darkens as the blast matures.
                let k = (self.darkness * (0.6 + 0.4 * s)).clamp(0.0, 1.0);
                let l = |x: f32, y: f32| x + (y - x) * k;
                VolumeMat {
                    scatter: [0, 1, 2].map(|i| l(g.scatter[i], b.scatter[i])),
                    absorption: l(g.absorption, b.absorption),
                    density: l(g.density, b.density),
                    ..g
                }
            }
            Medium::Dust => VolumeMat { scatter: [0.62, 0.53, 0.42], absorption: 0.35, density: 0.6, g_forward: 0.5, ..VolumePreset::GreySmoke.params() },
            Medium::Vapor => VolumeMat { scatter: [0.97, 0.97, 0.98], density: 0.4, ..VolumePreset::CumulusCloud.params() },
            Medium::Sparks => VolumeMat {
                scatter: [0.2; 3],
                absorption: 0.9,
                density: 6.0,
                ramp: if self.spark_color == SparkColor::Orange { EmissionRamp::Fire } else { EmissionRamp::Blackbody },
                emission: 30.0 * self.heat.max(0.0),
                cooling: 0.1,
                ..VolumePreset::Fire.params()
            },
        };
        // The large mushroom cloud is thinner per metre.
        let scale = if self.kind == PyroKind::Mushroom && m != Medium::Sparks { 0.08 } else { 1.0 };
        let mat = VolumeMat { density: mat.density * scale, ..mat };
        (mat.density > 0.0).then_some(mat)
    }

    /// The explosion light in object space: position and linear colour ×
    /// strength (an omni light's colour × multiplier).
    pub fn light(&self) -> Option<(Vec3, Vec3)> {
        if !self.light || self.light_strength <= 0.0 {
            return None;
        }
        let fire: Vec<_> = self.puffs().into_iter().filter(|q| q.medium == Medium::Fire).collect();
        let heat: f32 = fire.iter().map(|q| q.w * q.r * q.r * q.r).sum();
        if heat <= 0.0 {
            return None;
        }
        let c = fire.iter().map(|q| q.c * (q.w * q.r * q.r * q.r)).sum::<Vec3>() / heat;
        let mean_w = fire.iter().map(|q| q.w).sum::<f32>() / fire.len() as f32;
        let mat = self.mat(Medium::Fire)?;
        let col = mat.ramp.color(mean_w * 0.85);
        // Grows with the glowing surface (effective radius²); a 5 m
        // fireball comes out near a multiplier-6 omni.
        let r = heat.cbrt();
        Some((c, col * mat.emission * r * r * 0.011 * self.light_strength))
    }
}
