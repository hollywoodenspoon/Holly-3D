//! Clouds PR 3: trails (Create -> Geometry -> Trails, and the Demiurge tab).
//!
//! A trail is a chain of soft puffs along a path: the source (an engine, a
//! wingtip, a nozzle) sits at the start and the trail ages away from it,
//! widening, breaking into puffs and fading. The path is the built-in arc
//! (Length / Turn / Climb) or a picked spline. Puffs are the one source of
//! truth, as for clouds: a light preview mesh, and density for the volume
//! renderers (smoke and flame are separate media).

mod puffs;
pub mod path;
#[cfg(test)]
mod tests;

pub use puffs::TrailPuff;

use super::clouds::CloudLook;
use crate::materials::volume::{EmissionRamp, VolumeMat, VolumePreset};
use crate::mesh::EditablePoly;
use bevy::math::Vec3;

/// The trail types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TrailKind {
    #[default]
    Contrail,
    WingtipVortices,
    Afterburner,
    VaporCone,
    DamageSmoke,
    MissileTrail,
}

impl TrailKind {
    pub const ALL: [Self; 6] = [Self::Contrail, Self::WingtipVortices, Self::Afterburner, Self::VaporCone, Self::DamageSmoke, Self::MissileTrail];
    pub fn label(self) -> &'static str {
        match self {
            Self::Contrail => "Contrail",
            Self::WingtipVortices => "Wingtip Vortices",
            Self::Afterburner => "Afterburner Flame",
            Self::VaporCone => "Vapor Cone",
            Self::DamageSmoke => "Damage Smoke",
            Self::MissileTrail => "Missile Trail",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Self::Contrail => "White trail per engine: a gap behind the engine, then it widens and breaks into puffs",
            Self::WingtipVortices => "Thin white streaks spiralling off the wingtips; stronger in tight turns",
            Self::Afterburner => "Glowing exhaust cone with shock diamonds",
            Self::VaporCone => "Short white shell around the aircraft body, densest at the front",
            Self::DamageSmoke => "Thick black or grey trail, with optional fire at the start",
            Self::MissileTrail => "Bright motor flame and a thick white curving smoke trail",
        }
    }
}

/// One trail object's parameters (the Modify panel "Trail" rollout).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TrailParams {
    pub kind: TrailKind,
    pub seed: u32,
    /// Picked path in object space, source first (empty = built-in arc).
    pub path: Vec<Vec3>,
    /// Built-in arc: Length (m), Turn (degrees, + = left) and Climb (m).
    pub length: f32,
    pub turn: f32,
    pub climb: f32,
    /// Sources (engines or wingtips) side by side, `spacing` metres apart.
    pub sources: u32,
    pub spacing: f32,
    /// Start Gap (m): clear air between the source and the trail.
    pub start_gap: f32,
    pub start_radius: f32,
    pub end_radius: f32,
    /// Spread 0..=1: how far puffs drift sideways as the trail ages.
    pub spread: f32,
    /// Breakup 0..=1: how much the old trail splits into separate puffs.
    pub breakup: f32,
    /// Fade 0..=1: how thin the trail's end gets.
    pub fade: f32,
    /// Dissipation Length (m): the trail is gone this far from the source.
    pub dissipation: f32,
    /// Throttle 0..=1 (flames: length and glow).
    pub throttle: f32,
    /// Shock diamonds in an afterburner flame.
    pub diamonds: u32,
    /// Spiral turns per 100 m (Wingtip Vortices).
    pub twist: f32,
    /// Damage Smoke: fire at the start, and black (else grey) smoke.
    pub fire: bool,
    pub black: bool,
    pub look: CloudLook,
}

impl Default for TrailParams {
    fn default() -> Self {
        Self::of(TrailKind::Contrail)
    }
}

impl TrailParams {
    /// The preset for `kind` (Type dropdown).
    pub fn of(kind: TrailKind) -> Self {
        let look = CloudLook { erosion: 0.45, wispiness: 0.0, edge_softness: 0.6, ..CloudLook::default() };
        let base = Self {
            kind,
            seed: 1,
            path: Vec::new(),
            length: 1500.0,
            turn: 0.0,
            climb: 0.0,
            sources: 2,
            spacing: 14.0,
            start_gap: 40.0,
            start_radius: 0.8,
            end_radius: 16.0,
            spread: 0.3,
            breakup: 0.5,
            fade: 0.6,
            dissipation: 1500.0,
            throttle: 1.0,
            diamonds: 5,
            twist: 4.0,
            fire: true,
            black: true,
            look: look.clone(),
        };
        match kind {
            TrailKind::Contrail => base,
            TrailKind::WingtipVortices => Self {
                length: 80.0,
                turn: 25.0,
                spacing: 11.0,
                start_gap: 0.0,
                start_radius: 0.18,
                end_radius: 0.45,
                spread: 0.0,
                breakup: 0.25,
                fade: 0.85,
                dissipation: 80.0,
                look: CloudLook { erosion: 0.2, ..look.clone() },
                ..base
            },
            TrailKind::Afterburner => Self {
                length: 9.0,
                sources: 1,
                start_gap: 0.0,
                start_radius: 0.55,
                end_radius: 0.12,
                spread: 0.0,
                breakup: 0.0,
                fade: 0.7,
                dissipation: 9.0,
                look: CloudLook { erosion: 0.15, edge_softness: 0.8, ..look.clone() },
                ..base
            },
            TrailKind::VaporCone => Self {
                length: 9.0,
                sources: 1,
                start_gap: 0.0,
                start_radius: 1.6,
                end_radius: 4.0,
                spread: 0.0,
                breakup: 0.3,
                fade: 0.85,
                dissipation: 9.0,
                look: CloudLook { erosion: 0.5, ..look.clone() },
                ..base
            },
            TrailKind::DamageSmoke => Self {
                length: 300.0,
                sources: 1,
                start_gap: 0.0,
                start_radius: 1.0,
                end_radius: 14.0,
                spread: 0.5,
                breakup: 0.35,
                fade: 0.5,
                dissipation: 300.0,
                ..base
            },
            TrailKind::MissileTrail => Self {
                length: 800.0,
                turn: 40.0,
                climb: 120.0,
                sources: 1,
                start_gap: 2.0,
                start_radius: 0.7,
                end_radius: 7.0,
                spread: 0.45,
                breakup: 0.4,
                fade: 0.45,
                dissipation: 800.0,
                ..base
            },
        }
    }

    /// The path points, source first.
    pub fn path_points(&self) -> Vec<Vec3> {
        if self.path.len() >= 2 { self.path.clone() } else { path::Path::arc(self.length, self.turn, self.climb) }
    }

    /// Every puff (smoke and flame).
    pub fn puffs(&self) -> Vec<TrailPuff> {
        puffs::layout(self)
    }

    /// The preview mesh: smoke puffs on Material ID 2, flame on ID 1
    /// (Smoke & Fire pack).
    pub fn build(&self) -> EditablePoly {
        puffs::mesh(&self.puffs())
    }

    /// The smoke medium, if this type has smoke.
    pub fn smoke_mat(&self) -> Option<VolumeMat> {
        let white = |density: f32| VolumeMat { scatter: [0.97, 0.97, 0.98], density, ..VolumePreset::CumulusCloud.params() };
        match self.kind {
            TrailKind::Contrail => Some(white(0.15)),
            TrailKind::WingtipVortices => Some(white(0.6)),
            TrailKind::VaporCone => Some(white(0.5)),
            TrailKind::MissileTrail => Some(VolumeMat { absorption: 0.05, ..white(0.35) }),
            TrailKind::DamageSmoke => Some(VolumeMat { density: 0.5, ..if self.black { VolumePreset::BlackSmoke } else { VolumePreset::GreySmoke }.params() }),
            TrailKind::Afterburner => None,
        }
    }

    /// The flame medium, if this trail has fire.
    pub fn flame_mat(&self) -> Option<VolumeMat> {
        let t = self.throttle.clamp(0.0, 1.0);
        match self.kind {
            TrailKind::Afterburner => Some(VolumeMat {
                scatter: [0.6, 0.6, 0.7],
                absorption: 0.8,
                density: 2.0,
                ramp: EmissionRamp::Blackbody,
                emission: 4.0 + 16.0 * t,
                cooling: 0.3,
                ..VolumePreset::Fire.params()
            }),
            TrailKind::MissileTrail => Some(VolumeMat { emission: 8.0 + 10.0 * t, ..VolumePreset::Fireball.params() }),
            TrailKind::DamageSmoke if self.fire => Some(VolumePreset::Fire.params()),
            _ => None,
        }
    }
}
