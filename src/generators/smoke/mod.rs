//! PR 8: procedural billowing smoke & fire plumes (Create -> Geometry ->
//! Worlds / FX -> Smoke Plume, and the Battlefield / Settlement
//! `[✓] Spawn Rising Smoke Plumes` toggle).
//!
//! A plume is a stack of noise-displaced lobe shells (soft metaball-like
//! puffs) plus alpha-eroded turbulence cards. `column` holds the plume
//! physics: a buoyant plume widens linearly with height (entrainment),
//! slows as it rises (lobes bunch up into a mushroom cap), bends over in
//! the wind (rise ~ drift^(2/3)) and sways from side to side as vortices
//! shed at a Strouhal number of about 0.2. Density lives in the vertex
//! alpha and the tint in the vertex colour, so the translucent Smoke
//! material casts soft partial shadows in F9.

mod column;
mod shapes;
#[cfg(test)]
mod tests;

pub use column::Column;

use crate::mesh::EditablePoly;
use bevy::math::Vec3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SmokeStyle {
    /// Thick black oily smoke from burning fuel and buildings.
    #[default]
    Battlefield,
    /// Soft grey chimney / wood smoke.
    Chimney,
    /// White steam.
    Steam,
}

impl SmokeStyle {
    pub const ALL: [SmokeStyle; 3] = [Self::Battlefield, Self::Chimney, Self::Steam];
    pub fn label(self) -> &'static str {
        match self {
            Self::Battlefield => "Oily Battlefield Smoke (black)",
            Self::Chimney => "Chimney Smoke (grey)",
            Self::Steam => "Steam (white)",
        }
    }
    /// Linear tint of the smoke.
    pub fn color(self) -> Vec3 {
        match self {
            Self::Battlefield => Vec3::new(0.06, 0.055, 0.05),
            Self::Chimney => Vec3::new(0.4, 0.4, 0.41),
            Self::Steam => Vec3::new(0.9, 0.91, 0.93),
        }
    }
    /// Peak vertex alpha (optical density) of the lobes.
    pub fn density(self) -> f32 {
        match self {
            Self::Battlefield => 0.9,
            Self::Chimney => 0.65,
            Self::Steam => 0.45,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SmokeParams {
    pub style: SmokeStyle,
    pub seed: u32,
    /// Plume Height (m).
    pub height: f32,
    /// Base Radius (m) at the source.
    pub base_radius: f32,
    /// Top Spread: radius at the top as a multiple of the base radius.
    pub top_spread: f32,
    /// Billow Turbulence 0..=1 (lobe noise, sway and card count).
    pub turbulence: f32,
    /// Wind Lean Angle (degrees from vertical) and Direction (compass, deg).
    pub wind_lean: f32,
    pub wind_dir: f32,
    /// Time / Rise Phase 0..=1: scrubs the puffs up the column (loops).
    pub phase: f32,
    /// Fire Base Intensity 0..=100 % (0 = smoke only).
    pub fire: f32,
    /// Number of lobes up the column.
    pub lobes: u32,
    /// `[✓] Turbulence Cards`.
    pub cards: bool,
    /// `[✓] Mushroom Cap` (the ring vortex at the top).
    pub mushroom: bool,
}

impl Default for SmokeParams {
    fn default() -> Self {
        Self {
            style: SmokeStyle::Battlefield,
            seed: 3,
            height: 30.0,
            base_radius: 2.0,
            top_spread: 4.0,
            turbulence: 0.5,
            wind_lean: 18.0,
            wind_dir: 0.0,
            phase: 0.0,
            fire: 60.0,
            lobes: 16,
            cards: true,
            mushroom: true,
        }
    }
}

/// Material IDs a plume uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmokeIds {
    pub fire: u16,
    pub smoke: u16,
    pub wisp: u16,
}

impl SmokeIds {
    /// The Smoke & Fire pack (standalone plumes).
    pub const PACK: Self = Self { fire: 1, smoke: 2, wisp: 3 };
    /// Inside the Settlement pack (battlefield plumes).
    pub const TOWN: Self = Self { fire: super::kit::ids::EMBERS, smoke: super::kit::ids::SMOKE, wisp: super::kit::ids::SMOKE };
}

impl SmokeParams {
    /// Plume standing on the origin, rising along +Z.
    pub fn build(&self) -> EditablePoly {
        self.build_with(SmokeIds::PACK)
    }

    pub fn build_with(&self, ids: SmokeIds) -> EditablePoly {
        shapes::plume(self, ids)
    }
}

/// Appends a plume rising from `base` to `m` (Battlefield auto-spawn).
pub fn spawn(m: &mut EditablePoly, base: Vec3, p: &SmokeParams) {
    let plume = p.build_with(SmokeIds::TOWN);
    m.append(&plume, bevy::math::Mat4::from_translation(base));
}
