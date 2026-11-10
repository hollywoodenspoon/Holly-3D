//! "GhostTown" procedural settlements (Create -> Geometry -> Settlement and
//! the GhostTown modifier): Modern Downtown, Modern Suburbs, Medieval
//! Walled City and Medieval Village presets with automatic Multi/Sub-Object
//! Material IDs (`kit::ids`), plus the `[✓] Damaged / Battlefield Mode`
//! toggle that runs the Battlefield / Ruin modifier over the result.

mod downtown;
mod layout;
mod props;
mod suburb;
mod town;
mod townhouse;
mod village;

#[cfg(test)]
mod tests;

pub use layout::{Lot, Plan};
pub(crate) use townhouse::timber_frame;

use super::kit::Kit;
use crate::mesh::EditablePoly;
use crate::modifiers::battlefield::{BattleParams, BattlefieldMod, DestructionMode};
use bevy::math::{Mat4, Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Era {
    #[default]
    Downtown,
    Suburbs,
    WalledCity,
    Village,
}

impl Era {
    pub const ALL: [Era; 4] = [Self::Downtown, Self::Suburbs, Self::WalledCity, Self::Village];
    pub fn label(self) -> &'static str {
        match self {
            Self::Downtown => "Modern Downtown City",
            Self::Suburbs => "Modern Suburbs / Subdivision",
            Self::WalledCity => "Medieval Walled City",
            Self::Village => "Medieval Village / Hamlet",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Self::Downtown => "Downtown",
            Self::Suburbs => "Suburbs",
            Self::WalledCity => "Walled City",
            Self::Village => "Village",
        }
    }
    pub fn is_medieval(self) -> bool {
        matches!(self, Self::WalledCity | Self::Village)
    }
    /// Storey height in metres.
    pub fn floor_height(self) -> f32 {
        match self {
            Self::Downtown => 3.6,
            Self::Suburbs => 3.0,
            Self::WalledCity | Self::Village => 3.1,
        }
    }
    /// The period-accurate destruction mode for the Damaged toggle.
    pub fn destruction(self) -> DestructionMode {
        if self.is_medieval() { DestructionMode::MedievalSiege } else { DestructionMode::Modern }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SettlementParams {
    pub era: Era,
    pub seed: u32,
    pub width: f32,
    pub length: f32,
    /// City block (or farm plot / lot frontage) size in metres.
    pub block_size: f32,
    pub street_width: f32,
    pub sidewalk_width: f32,
    /// Share of lots that receive a building (0..1).
    pub density: f32,
    pub min_floors: u32,
    pub max_floors: u32,
    /// Roof pitch in degrees (0 = flat roofs).
    pub roof_pitch: f32,
    /// Trees, streetlamps, wells and other props (0..1).
    pub prop_density: f32,
    /// `[✓] Damaged / Battlefield Mode`.
    pub damaged: bool,
    pub damage: BattleParams,
}

impl Default for SettlementParams {
    fn default() -> Self {
        Self::of(Era::Downtown)
    }
}

impl SettlementParams {
    /// Era defaults (the preset dropdown resets the layout parameters).
    pub fn of(era: Era) -> Self {
        let base = Self {
            era,
            seed: 1,
            width: 200.0,
            length: 200.0,
            block_size: 56.0,
            street_width: 12.0,
            sidewalk_width: 3.0,
            density: 0.85,
            min_floors: 4,
            max_floors: 36,
            roof_pitch: 0.0,
            prop_density: 0.5,
            damaged: false,
            damage: BattleParams::of(era.destruction()),
        };
        match era {
            Era::Downtown => base,
            Era::Suburbs => Self { block_size: 44.0, street_width: 8.0, sidewalk_width: 1.5, density: 0.8, min_floors: 1, max_floors: 2, roof_pitch: 32.0, prop_density: 0.6, ..base },
            Era::WalledCity => Self { block_size: 9.0, street_width: 4.5, sidewalk_width: 0.0, density: 0.95, min_floors: 2, max_floors: 4, roof_pitch: 52.0, prop_density: 0.3, ..base },
            Era::Village => Self { block_size: 34.0, street_width: 4.0, sidewalk_width: 0.0, density: 0.6, min_floors: 1, max_floors: 1, roof_pitch: 48.0, prop_density: 0.7, ..base },
        }
    }

    /// Builds the settlement centred on the origin (ground at z = 0).
    pub fn build(&self) -> EditablePoly {
        self.build_in(None)
    }

    /// Builds inside an optional closed XY outline (centred coordinates).
    pub fn build_in(&self, outline: Option<&[Vec2]>) -> EditablePoly {
        let half = Vec2::new(self.width.max(10.0), self.length.max(10.0)) * 0.5;
        let mut plan = Plan::new(self, -half, half, outline.map(|o| o.to_vec()));
        match self.era {
            Era::Downtown => downtown::build(&mut plan),
            Era::Suburbs => suburb::build(&mut plan),
            Era::WalledCity => town::build(&mut plan),
            Era::Village => village::build(&mut plan),
        }
        let mut m = plan.kit.finish();
        if self.damaged {
            BattlefieldMod { params: self.damage.clone(), zones: vec![] }.apply(&mut m);
        }
        m
    }
}

/// GhostTown as a stack modifier: fills the input's XY footprint (a flat
/// single polygon's outline, otherwise its bounds) and sits on its top.
#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SettlementMod {
    pub params: SettlementParams,
}

impl SettlementMod {
    pub fn apply(&self, mesh: &mut EditablePoly) {
        let Some(b) = mesh.bounds() else { return };
        let size = b.max - b.min;
        let flat = size.z <= size.truncate().max_element() * 0.02;
        let outline: Option<Vec<Vec2>> = (flat && mesh.polygons.len() == 1)
            .then(|| mesh.polygons.values().next().map(|p| p.vertex_ids().map(|v| mesh.pos(v).truncate()).collect()))
            .flatten();
        *mesh = self.fill(outline, b.min.truncate(), b.max.truncate(), b.max.z);
    }

    /// Fills the first closed spline of a shape (flowing up the stack).
    pub fn build_from_shape(&self, shape: &crate::splines::Shape) -> EditablePoly {
        let lines = shape.polylines();
        let Some(pl) = lines.iter().find(|p| p.closed && p.pts.len() >= 3).or(lines.first()) else { return EditablePoly::new() };
        let Some(b) = crate::mesh::Aabb::from_points(pl.pts.iter().copied()) else { return EditablePoly::new() };
        let outline = pl.closed.then(|| pl.pts.iter().map(|p| p.truncate()).collect());
        self.fill(outline, b.min.truncate(), b.max.truncate(), b.min.z)
    }

    fn fill(&self, outline: Option<Vec<Vec2>>, min: Vec2, max: Vec2, z: f32) -> EditablePoly {
        let c = (min + max) * 0.5;
        let size = (max - min).max(Vec2::splat(10.0));
        let p = SettlementParams { width: size.x, length: size.y, ..self.params.clone() };
        let local: Option<Vec<Vec2>> = outline.map(|o| o.iter().map(|q| *q - c).collect());
        let mut m = p.build_in(local.as_deref());
        m.transform(Mat4::from_translation(c.extend(z)));
        m
    }
}

/// Kit helpers shared by the era builders.
pub(crate) fn window(kit: &mut Kit, x: f32, y: f32, z: f32, w: f32, h: f32, id: u16) {
    kit.mat(id).quad(Vec3::new(x - w * 0.5, y, z), Vec3::new(x + w * 0.5, y, z), Vec3::new(x + w * 0.5, y, z + h), Vec3::new(x - w * 0.5, y, z + h));
}
