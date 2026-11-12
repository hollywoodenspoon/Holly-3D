//! Phase 10A: placing and editing Character Studio objects. Every new
//! character gets a fresh Character Pack (Multi/Sub-Object) so its
//! Material IDs (skin, eyes, hair, cloth, metal, leather) render at once.

use super::*;
use crate::character::params::{Archetype, Creature};
use crate::character::CharacterParams;
use crate::materials::{Material, MultiPreset};
use crate::scene::BaseObject;

impl Editor {
    /// Places a character built from `p` at `at` and selects it.
    pub fn create_character(&mut self, p: CharacterParams, at: Vec3) -> ObjId {
        self.checkpoint("Create Character");
        let id = self.scene.add(BaseObject::Character(Box::new(p)), at);
        let mut m = Material::named(format!("{} Skin", self.scene.objects[id].name));
        MultiPreset::Character.apply(&mut m);
        self.scene.objects[id].material = Some(self.scene.materials.mats.insert(m));
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {} with the Character Pack", self.scene.objects[id].name);
        id
    }

    /// One-click archetype (Human, Elf, Dwarf, ... Anime Heroine).
    pub fn create_archetype(&mut self, a: Archetype, at: Vec3) -> ObjId {
        let mut p = CharacterParams::default();
        a.apply(&mut p);
        self.create_character(p, at)
    }

    pub fn create_creature(&mut self, c: Creature, at: Vec3) -> ObjId {
        self.create_character(CharacterParams { creature: c, ..Default::default() }, at)
    }

    /// The current object's character parameters, if it is one.
    pub fn character(&self) -> Option<(ObjId, CharacterParams)> {
        let id = self.current()?;
        match &self.scene.objects[id].base {
            BaseObject::Character(c) => Some((id, CharacterParams::clone(c))),
            _ => None,
        }
    }

    /// Replaces the character's parameters (the caller records the undo
    /// step) and re-evaluates it.
    pub fn set_character(&mut self, id: ObjId, p: CharacterParams) {
        let o = &mut self.scene.objects[id];
        if let BaseObject::Character(cur) = &mut o.base {
            **cur = p;
        }
        o.evaluate();
        self.scene.propagate_from(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_edit_and_undo_a_character() {
        let mut ed = Editor::default();
        let id = ed.create_archetype(Archetype::Elf, Vec3::ZERO);
        let polys = ed.scene.objects[id].evaluated_mesh.counts().2;
        assert!(polys > 13000);
        assert!(ed.scene.objects[id].material.is_some());
        let (_, mut p) = ed.character().unwrap();
        p.creature = Creature::Slime;
        ed.checkpoint("Character Parameters");
        ed.set_character(id, p);
        assert!(ed.scene.objects[id].evaluated_mesh.counts().2 < 2000);
        ed.undo();
        assert!(ed.scene.objects[id].evaluated_mesh.counts().2 > 13000);
    }
}
