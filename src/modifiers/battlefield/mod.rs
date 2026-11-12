//! Battlefield & Ruin: a period-accurate destruction modifier (also run by
//! the `Damaged / Battlefield Mode` toggles of GhostTown and the Medieval
//! Architect). Blast zones (user-placed sub-object spheres, or seeded ones
//! on the structures) drive a cut height field per mode:
//! - Modern / Mecha: tilted Voronoi-style shear planes, pancaked slabs,
//!   shattered windows, bent rebar, bomb and mech-footprint craters,
//!   buckled roads, bent streetlamps and concrete rubble.
//! - Medieval Siege: V-shaped breaches, knocked-off crenellations, burned
//!   roofs with charred rafters, impact pits, siege ruts, talus piles,
//!   splintered beams, carts and barrels. Never rebar or concrete.
//! - Ancient Overgrown Ruins: eroded wall tops, caved roofs, snapped
//!   pillars with fallen drums, sinkholes, moss and ivy.

mod clusters;
mod craters;
pub(crate) mod debris;
mod field;
mod plumes;
mod weather;
mod wreck;

#[cfg(test)]
mod tests;

pub use field::Zone;

use crate::generators::kit::Kit;
use crate::generators::smoke::{SmokeParams, SmokeStyle};
use crate::mesh::EditablePoly;
use bevy::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DestructionMode {
    #[default]
    Modern,
    MedievalSiege,
    AncientRuins,
}

impl DestructionMode {
    pub const ALL: [DestructionMode; 3] = [Self::Modern, Self::MedievalSiege, Self::AncientRuins];
    pub fn label(self) -> &'static str {
        match self {
            Self::Modern => "Modern / Mecha Warfare",
            Self::MedievalSiege => "Medieval Siege",
            Self::AncientRuins => "Ancient Overgrown Ruins",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BattleParams {
    pub mode: DestructionMode,
    /// Destruction Intensity, 0..=100 % (0 leaves the input untouched).
    pub intensity: f32,
    pub seed: u32,
    pub crater_count: u32,
    pub crater_radius: f32,
    pub crater_depth: f32,
    /// Debris / Rubble Density, 0..=1.
    pub rubble: f32,
    /// Soot / Burn Darkening, 0..=1.
    pub soot: f32,
    /// PR 7 Building Destruction applied per building (Intact = off).
    pub buildings: crate::generators::ruin::RuinParams,
    /// PR 8 `[✓] Spawn Rising Smoke Plumes` + Smoke Plume Count.
    pub plumes: bool,
    pub plume_count: u32,
    /// The look of the spawned plumes.
    pub plume: crate::generators::smoke::SmokeParams,
}

impl Default for BattleParams {
    fn default() -> Self {
        Self::of(DestructionMode::Modern)
    }
}

impl BattleParams {
    pub fn of(mode: DestructionMode) -> Self {
        let base = Self {
            mode,
            intensity: 60.0,
            seed: 7,
            crater_count: 4,
            crater_radius: 6.0,
            crater_depth: 2.5,
            rubble: 0.6,
            soot: 0.6,
            buildings: Default::default(),
            plumes: false,
            plume_count: 3,
            plume: crate::generators::smoke::SmokeParams { height: 45.0, base_radius: 3.0, ..Default::default() },
        };
        match mode {
            DestructionMode::Modern => base,
            DestructionMode::MedievalSiege => {
                let plume = SmokeParams { style: SmokeStyle::Chimney, height: 30.0, base_radius: 2.0, ..base.plume.clone() };
                Self { intensity: 55.0, crater_count: 3, crater_radius: 3.5, crater_depth: 1.2, soot: 0.5, plume, ..base }
            }
            DestructionMode::AncientRuins => {
                let plume = SmokeParams { style: SmokeStyle::Chimney, fire: 20.0, height: 25.0, base_radius: 2.0, ..base.plume.clone() };
                Self { intensity: 65.0, crater_count: 2, crater_radius: 4.0, crater_depth: 3.0, rubble: 0.4, soot: 0.2, plume, ..base }
            }
        }
    }
    /// Intensity as 0..=1.
    pub fn strength(&self) -> f32 {
        (self.intensity / 100.0).clamp(0.0, 1.0)
    }
}

/// A user-placed blast zone (the `Blast Zones` sub-object), object space.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlastZone {
    pub center: Vec3,
    pub radius: f32,
    #[serde(default)]
    pub selected: bool,
}

impl BlastZone {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius: radius.max(0.1), selected: false }
    }
}

#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BattlefieldMod {
    pub params: BattleParams,
    /// Empty = seeded zones on the structures, one per crater.
    pub zones: Vec<BlastZone>,
}

impl BattlefieldMod {
    pub fn apply(&self, mesh: &mut EditablePoly) {
        let p = &self.params;
        if mesh.polygons.is_empty() {
            return;
        }
        clusters::run(mesh, p);
        self.blast(mesh);
        // Plumes last, so they rise from the finished wrecks.
        plumes::run(mesh, p, &self.zones);
    }

    fn blast(&self, mesh: &mut EditablePoly) {
        let p = &self.params;
        if p.strength() <= 0.0 {
            return;
        }
        let Some(info) = field::Field::new(mesh, p, &self.zones) else { return };
        let mut kit = Kit { m: std::mem::replace(mesh, EditablePoly::new()), ..Kit::new() };
        // Buildings first (on the original elements), then the ground, then
        // surface weathering of everything that is left.
        wreck::run(&mut kit, &info);
        craters::run(&mut kit, &info);
        weather::run(&mut kit.m, &info);
        kit.m.remove_isolated_vertices();
        *mesh = kit.finish();
    }

    /// Zone added by `[ Add Zone ]`: centred on the input (or beside the
    /// last zone), sized from the crater radius.
    pub fn add_zone(&mut self, bounds: Option<crate::mesh::Aabb>) {
        let r = (self.params.crater_radius * 2.5).max(1.0);
        let c = match (self.zones.last(), bounds) {
            (Some(z), _) => z.center + Vec3::X * z.radius * 1.5,
            (None, Some(b)) => Vec3::new((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5, b.min.z),
            (None, None) => Vec3::ZERO,
        };
        self.zones.iter_mut().for_each(|z| z.selected = false);
        self.zones.push(BlastZone { selected: true, ..BlastZone::new(c, r) });
    }

    /// Centroid of the selected zones (the W / E / R gizmo position).
    pub fn selected_center(&self) -> Option<Vec3> {
        let sel: Vec<Vec3> = self.zones.iter().filter(|z| z.selected).map(|z| z.center).collect();
        (!sel.is_empty()).then(|| sel.iter().sum::<Vec3>() / sel.len() as f32)
    }

    /// Applies a local-space drag to the selected zones: W moves their
    /// centres, R scales the spheres (by the mean axis scale).
    pub fn transform_selected(&mut self, local: bevy::math::Mat4) {
        let (scale, _, _) = local.to_scale_rotation_translation();
        let k = (scale.x + scale.y + scale.z) / 3.0;
        for z in self.zones.iter_mut().filter(|z| z.selected) {
            z.center = local.transform_point3(z.center);
            z.radius = (z.radius * k).max(0.1);
        }
    }

    /// `[ Remove Zone ]`: drops the selected zones.
    pub fn remove_selected(&mut self) -> usize {
        let n = self.zones.len();
        self.zones.retain(|z| !z.selected);
        n - self.zones.len()
    }
}
