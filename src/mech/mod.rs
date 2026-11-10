//! Mech primitives (Create -> Geometry -> Mech): Involute Gear, Hydraulic
//! Piston parts and Hose / Mecha Cable. Like the standard primitives their
//! pivot sits at the base and every parameter stays live.

mod bind;
pub mod gear;
pub mod hose;
pub mod piston;
#[cfg(test)]
mod tests;

use crate::mesh::EditablePoly;
pub use gear::{Gear, GearType};
pub use hose::{Hose, HoseCurve};
pub use piston::PistonPart;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MechPart {
    Gear(Gear),
    Piston(PistonPart),
    Hose(Hose),
}

/// The Create tab's Mech buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechKind {
    Gear,
    Piston,
    Hose,
}

impl MechKind {
    pub const ALL: [MechKind; 3] = [MechKind::Gear, MechKind::Piston, MechKind::Hose];
    pub fn label(self) -> &'static str {
        match self {
            MechKind::Gear => "Gear",
            MechKind::Piston => "Piston",
            MechKind::Hose => "Hose",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            MechKind::Gear => "Involute Gear: spur, internal ring or rack",
            MechKind::Piston => "Hydraulic Piston: housing + rod with eyelets, joined by a Piston joint",
            MechKind::Hose => "Hose / Mecha Cable: free, or bound to two objects",
        }
    }
}

impl MechPart {
    pub fn build(&self) -> EditablePoly {
        match self {
            MechPart::Gear(g) => g.build(),
            MechPart::Piston(p) => p.build(),
            MechPart::Hose(h) => h.build(),
        }
    }

    /// Object name stem.
    pub fn name(&self) -> &'static str {
        match self {
            MechPart::Gear(g) if g.kind == GearType::Rack => "Rack",
            MechPart::Gear(_) => "Gear",
            MechPart::Piston(p) if p.rod => "PistonRod",
            MechPart::Piston(_) => "PistonHousing",
            MechPart::Hose(_) => "Hose",
        }
    }
}
