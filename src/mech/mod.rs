//! Mech primitives (Create -> Geometry -> Mech): Involute Gear, Hydraulic
//! Piston parts, Hose / Mecha Cable and threaded fasteners (bolt, nut,
//! tapped hole block). Like the standard primitives their
//! pivot sits at the base and every parameter stays live.

mod bind;
pub mod fastener;
pub mod gear;
pub mod hose;
pub mod piston;
pub mod spider;
#[cfg(test)]
mod tests;

use crate::mesh::EditablePoly;
pub use fastener::{Fastener, FastenerKind, ThreadStd};
pub use gear::{Gear, GearType};
pub use hose::{Hose, HoseCurve};
pub use piston::PistonPart;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MechPart {
    Gear(Gear),
    Piston(PistonPart),
    Hose(Hose),
    Fastener(Fastener),
}

/// The Create tab's Mech buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechKind {
    Gear,
    Piston,
    Hose,
    Bolt,
    Nut,
    TappedHole,
}

impl MechKind {
    pub const ALL: [MechKind; 6] = [MechKind::Gear, MechKind::Piston, MechKind::Hose, MechKind::Bolt, MechKind::Nut, MechKind::TappedHole];
    pub fn label(self) -> &'static str {
        match self {
            MechKind::Gear => "Gear",
            MechKind::Piston => "Piston",
            MechKind::Hose => "Hose",
            MechKind::Bolt => "Bolt",
            MechKind::Nut => "Nut",
            MechKind::TappedHole => "Threaded Hole",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            MechKind::Gear => "Involute Gear: spur, internal ring or rack",
            MechKind::Piston => "Hydraulic Piston: housing + rod with eyelets, joined by a Piston joint",
            MechKind::Hose => "Hose / Mecha Cable: free, or bound to two objects",
            MechKind::Bolt => "ISO metric / UNC hex bolt with a real helical thread",
            MechKind::Nut => "ISO metric / UNC hex nut, threaded to fit the same-size bolt",
            MechKind::TappedHole => "A block with a threaded hole straight through",
        }
    }

    /// The fastener a Bolt / Nut / Threaded Hole button makes.
    pub fn fastener(self) -> Option<FastenerKind> {
        match self {
            MechKind::Bolt => Some(FastenerKind::Bolt),
            MechKind::Nut => Some(FastenerKind::Nut),
            MechKind::TappedHole => Some(FastenerKind::TappedHole),
            _ => None,
        }
    }
}

impl MechPart {
    pub fn build(&self) -> EditablePoly {
        match self {
            MechPart::Gear(g) => g.build(),
            MechPart::Piston(p) => p.build(),
            MechPart::Hose(h) => h.build(),
            MechPart::Fastener(f) => f.build(),
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
            MechPart::Fastener(f) => match f.kind {
                FastenerKind::Bolt => "Bolt",
                FastenerKind::Nut => "Nut",
                FastenerKind::TappedHole => "ThreadedHole",
            },
        }
    }
}
