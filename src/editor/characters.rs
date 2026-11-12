//! Phase 10A: placing and editing Character Studio objects. Every new
//! character gets a fresh Character Pack (Multi/Sub-Object) so its
//! Material IDs (skin, eyes, hair, cloth, metal, leather) render at once.

use super::*;
use crate::character::params::{Archetype, Creature};
use crate::character::wardrobe::{self, MhAssets};
use crate::character::CharacterParams;
use crate::materials::{Material, MultiPreset};
use crate::scene::BaseObject;

/// What the Create Character tool places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterPick {
    Archetype(Archetype),
    Creature(Creature),
}

impl CharacterPick {
    pub fn label(self) -> &'static str {
        match self {
            Self::Archetype(a) => a.label(),
            Self::Creature(c) => c.label(),
        }
    }
}

impl Editor {
    /// Places the picked archetype or creature at `at`.
    pub fn create_pick(&mut self, pick: CharacterPick, at: Vec3) -> ObjId {
        match pick {
            CharacterPick::Archetype(a) => self.create_archetype(a, at),
            CharacterPick::Creature(c) => self.create_creature(c, at),
        }
    }

    /// Places a character built from `p` at `at` and selects it.
    pub fn create_character(&mut self, p: CharacterParams, at: Vec3) -> ObjId {
        self.checkpoint("Create Character");
        let id = self.scene.add(BaseObject::Character(Box::new(p)), at);
        let mut m = Material::named(format!("{} Skin", self.scene.objects[id].name));
        MultiPreset::Character.apply(&mut m);
        if let (Some(ms), BaseObject::Character(c)) = (m.multi.as_mut(), &self.scene.objects[id].base) {
            wardrobe::materials::sync(&c.mh, ms);
        }
        self.scene.objects[id].material = Some(self.scene.materials.mats.insert(m));
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {} with the Character Pack", self.scene.objects[id].name);
        id
    }

    /// One-click archetype (Human, Elf, Dwarf, ... Anime Heroine).
    pub fn create_archetype(&mut self, a: Archetype, at: Vec3) -> ObjId {
        let mut p = CharacterParams { mh: MhAssets::face_kit(), ..Default::default() };
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
        self.sync_character_materials(id, &p.mh);
        let o = &mut self.scene.objects[id];
        if let BaseObject::Character(cur) = &mut o.base {
            **cur = p;
        }
        o.evaluate();
        self.scene.propagate_from(id);
    }
}

impl Editor {
    /// Textures the character's Character Pack for its MakeHuman assets.
    /// A plain (non Multi/Sub-Object) material is left as the user set it.
    pub fn sync_character_materials(&mut self, id: ObjId, mh: &MhAssets) {
        let Some(mid) = self.scene.objects.get(id).and_then(|o| o.material) else { return };
        if let Some(ms) = self.scene.materials.mats.get_mut(mid).and_then(|m| m.multi.as_mut()) {
            wardrobe::materials::sync(mh, ms);
        }
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

    #[test]
    fn click_places_and_drag_sets_the_height() {
        use crate::viewport::layout::ScreenRect;
        let mut ed = Editor::default();
        (ed.viewports.maximized, ed.viewports.active) = (true, 0);
        ed.viewports.layout(ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
        let pick = CharacterPick::Creature(Creature::Slime);
        ed.tool = Tool::CreateCharacter(pick);
        let screen = |ed: &Editor, p: Vec3| ed.viewports.views[0].to_screen(p).unwrap();
        // A plain click places a default-size creature where clicked.
        let at = Vec3::new(2.0, 1.0, 0.0);
        assert!(ed.tool_click(0, screen(&ed, at)));
        let id = ed.current().unwrap();
        assert!((ed.scene.objects[id].transform.translation - at).length() < 0.05);
        assert_eq!(ed.tool, Tool::CreateCharacter(pick), "the tool stays armed");
        // Dragging 3 units scales it to 3 units tall.
        let (a, b) = (Vec3::new(-2.0, 0.0, 0.0), Vec3::new(-2.0, 3.0, 0.0));
        assert!(ed.create_press(0, screen(&ed, a)));
        ed.create_move(screen(&ed, b));
        ed.create_release(screen(&ed, b));
        assert!(ed.iface.create_drag.is_none(), "one step");
        let id2 = ed.current().unwrap();
        assert_ne!(id, id2);
        let o = &ed.scene.objects[id2];
        let bb = o.evaluated_mesh.bounds().unwrap();
        let tall = (bb.max.z - bb.min.z) * o.transform.scale.z;
        assert!((tall - 3.0).abs() < 0.1, "{tall}");
        assert!((o.transform.translation - a).length() < 0.05);
    }
}
