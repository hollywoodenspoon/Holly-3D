//! Phase 10A: RPG-style Character & Creature Studio. Humanoids are built
//! on the MakeHuman 1.x hm08 CC0 base mesh and targets ([`makehuman`]);
//! creatures are procedural ([`creature`]).

pub mod anime;
pub mod archetypes;
pub mod body;
pub mod creature;
pub mod hair;
pub mod landmarks;
pub mod makehuman;
pub mod outfit;
pub mod params;
pub mod shapes;

#[cfg(test)]
mod tests;

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
