//! PR 7: five-state building destruction and rubble mounds.
//!
//! A building mesh (ground at its lowest point) is wrecked to one of
//! `Intact`, `Light Damage`, `Heavy Damage`, `Warzone Collapse`,
//! `Apocalyptic Ruin` or `Pure Rubble`. A load-path stress model
//! (`stress`) decides where the structure gives way: blast-hit columns
//! lose capacity, the storey whose remaining capacity cannot carry the
//! floors above fails, and the building leans or falls toward the side
//! that lost its support. Every state above Light also piles a layered
//! rubble mound (`mound`) of dust, chunks, bricks, girders or timbers,
//! with optional glowing embers (ID 25) and blood smears (ID 28).
//! Period-accurate: Medieval and Ancient never get rebar or girders.

mod collapse;
mod damage;
mod mound;
mod stress;
#[cfg(test)]
mod tests;

pub use mound::rubble_pile;

use super::kit::{Kit, rng};
use crate::mesh::EditablePoly;
use bevy::math::Vec2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum DestructionLevel {
    #[default]
    Intact,
    Light,
    Heavy,
    Collapse,
    Ruin,
    Rubble,
}

impl DestructionLevel {
    pub const ALL: [DestructionLevel; 6] = [Self::Intact, Self::Light, Self::Heavy, Self::Collapse, Self::Ruin, Self::Rubble];
    pub fn label(self) -> &'static str {
        match self {
            Self::Intact => "Intact",
            Self::Light => "Light Damage",
            Self::Heavy => "Heavy Damage",
            Self::Collapse => "Warzone Collapse",
            Self::Ruin => "Apocalyptic Ruin",
            Self::Rubble => "Pure Rubble",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Intact => "Untouched.",
            Self::Light => "Broken glass, bullet scars and soot round the windows, a little debris.",
            Self::Heavy => "A blown corner, shell holes, exposed slabs, a lean, a caved roof, sandbags.",
            Self::Collapse => "The failed storey gives way: the top topples or pancakes onto a rubble mound.",
            Self::Ruin => "A jagged stump of the ground floors over a big mound.",
            Self::Rubble => "Nothing standing: a rubble mound only.",
        }
    }
    /// One step up or down, clamped.
    pub fn step(self, d: i32) -> Self {
        Self::ALL[(self as i32 + d).clamp(0, 5) as usize]
    }
}

/// Which debris goes into the wreck.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RuinEra {
    /// From the building type (medieval types get stone and timber).
    #[default]
    Auto,
    Modern,
    Medieval,
    Ancient,
}

impl RuinEra {
    pub const ALL: [RuinEra; 4] = [Self::Auto, Self::Modern, Self::Medieval, Self::Ancient];
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto (from type)",
            Self::Modern => "Modern: concrete, rebar, girders",
            Self::Medieval => "Medieval: stone, charred timber",
            Self::Ancient => "Ancient: weathered stone, moss",
        }
    }
    /// `Auto` resolved with the building's own period.
    pub fn resolve(self, medieval: bool) -> Self {
        match (self, medieval) {
            (Self::Auto, true) => Self::Medieval,
            (Self::Auto, false) => Self::Modern,
            (e, _) => e,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RuinParams {
    pub level: DestructionLevel,
    pub era: RuinEra,
    pub seed: u32,
    /// Rubble density 0..=1 (chunks per square metre of mound).
    pub rubble: f32,
    /// `[✓] Glowing Embers` in the rubble crevices (ID 25).
    pub embers: bool,
    /// Emission of ID 25 while Glowing Embers is on.
    pub ember_glow: f32,
    /// `[✓] Gore / Blood Smears` (ID 28).
    pub gore: bool,
    /// Blood amount 0..=100 %.
    pub gore_amount: f32,
    /// Rubble Pile type: height of the mound (Width / Length set its spread).
    pub pile_height: f32,
}

impl Default for RuinParams {
    fn default() -> Self {
        Self {
            level: DestructionLevel::Intact,
            era: RuinEra::Auto,
            seed: 11,
            rubble: 0.6,
            embers: true,
            ember_glow: crate::materials::ghost_pack::EMBER_GLOW,
            gore: false,
            gore_amount: 30.0,
            pile_height: 3.0,
        }
    }
}

/// Footprint of the wreck: centre, half size and ground height.
#[derive(Clone, Copy)]
pub(crate) struct Foot {
    pub c: Vec2,
    pub half: Vec2,
    pub z: f32,
}

/// Wrecks `m` in place. `medieval` resolves `RuinEra::Auto`.
pub fn apply(m: &mut EditablePoly, p: &RuinParams, medieval: bool) {
    if p.level == DestructionLevel::Intact || m.polygons.is_empty() {
        return;
    }
    let Some(b) = m.bounds() else { return };
    let era = p.era.resolve(medieval);
    let mut r = rng(p.seed, 0x7E17);
    let st = stress::Stress::new(&b, p.level, &mut r);
    let mut kit = Kit { m: std::mem::replace(m, EditablePoly::new()), ..Kit::new() };
    let foot = Foot {
        c: Vec2::new(b.min.x + b.max.x, b.min.y + b.max.y) * 0.5,
        half: Vec2::new(b.max.x - b.min.x, b.max.y - b.min.y) * 0.5,
        z: b.min.z,
    };
    use DestructionLevel as L;
    match p.level {
        L::Intact => {}
        L::Light => damage::light(&mut kit, &b, era, 0.35, &mut r),
        L::Heavy => {
            damage::light(&mut kit, &b, era, 0.7, &mut r);
            damage::heavy(&mut kit, &b, &st, era, &mut r);
        }
        L::Collapse => {
            damage::light(&mut kit, &b, era, 0.9, &mut r);
            collapse::collapse(&mut kit, &b, &st, era, &mut r);
        }
        L::Ruin => collapse::stump(&mut kit, &b, &st, era, &mut r),
        L::Rubble => kit.m = EditablePoly::new(),
    }
    let height = b.max.z - b.min.z;
    let (size, spread) = match p.level {
        L::Intact | L::Light => (0.0, 0.0),
        L::Heavy => (0.12, 0.55),
        L::Collapse => (0.32, 1.0),
        L::Ruin => (0.45, 1.15),
        L::Rubble => (0.6, 1.25),
    };
    if size > 0.0 {
        let pile = (height * size * 0.35).clamp(0.6, 9.0).min(foot.half.min_element().max(0.5) * 1.2);
        let toward = st.tilt * foot.half.length() * 0.25;
        let f = Foot { c: foot.c + toward, half: foot.half * spread + Vec2::splat(1.0), ..foot };
        mound::mound(&mut kit, &f, pile, era, p, &mut r);
    } else {
        mound::decor(&mut kit, &foot, era, p, &mut r);
    }
    kit.m.remove_isolated_vertices();
    *m = kit.finish();
}
