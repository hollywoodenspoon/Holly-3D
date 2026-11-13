//! Phase 10A: RPG-style Character & Creature Studio. Humanoids are built
//! on the MakeHuman 1.x hm08 CC0 base mesh and targets ([`makehuman`]);
//! creatures are procedural ([`creature`]). Phase 10B rigs humanoids with
//! the MakeHuman default skeleton and poses them with Easy Pose ([`rig`]).

pub mod anime;
pub mod archetypes;
pub mod body;
pub mod creature;
pub mod face_cloner;
pub mod hair;
pub mod landmarks;
pub mod makehuman;
pub mod outfit;
pub mod params;
pub mod rig;
pub mod shapes;
pub mod wardrobe;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_look;

pub use params::CharacterParams;

impl CharacterParams {
    /// The character mesh (humanoid or creature).
    pub fn build(&self) -> crate::mesh::EditablePoly {
        if self.creature == params::Creature::Humanoid {
            body::build_human(self)
        } else {
            creature::build(self)
        }
    }
}
