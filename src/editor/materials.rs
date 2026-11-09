//! Compact Material Editor actions: assign, pick, reset, copy, presets and
//! undoable material edits on the active sample slot.

use super::*;
use crate::materials::{MatId, Material, Preset};

impl Editor {
    /// Material in the active sample slot.
    pub fn active_material(&self) -> MatId {
        self.scene.materials.slot(self.mat_editor.slot)
    }

    /// True when a scene object uses `id` ("hot" material).
    pub fn material_is_hot(&self, id: MatId) -> bool {
        self.scene.objects.values().any(|o| o.material == Some(id))
    }

    /// Applies `f` to the active slot's material, recording undo when `label` is set.
    pub fn edit_material(&mut self, label: Option<&str>, f: impl FnOnce(&mut Material)) {
        if let Some(l) = label {
            self.checkpoint(l);
        }
        let id = self.active_material();
        if let Some(m) = self.scene.materials.mats.get_mut(id) {
            f(m);
        }
    }

    /// `[ Assign Material to Selection ]`.
    pub fn assign_material_to_selection(&mut self) {
        let sel = self.scene.selected();
        if sel.is_empty() {
            self.status = "Assign Material: select one or more objects first".into();
            return;
        }
        self.checkpoint("Assign Material");
        let id = self.active_material();
        for &o in &sel {
            self.scene.objects[o].material = Some(id);
        }
        let name = &self.scene.materials.mats[id].name;
        self.status = format!("Assigned \"{name}\" to {} object(s)", sel.len());
    }

    /// `[ Show Shaded Material in Viewport ]` toggle.
    pub fn toggle_material_in_viewport(&mut self) {
        self.edit_material(Some("Show Material in Viewport"), |m| m.show_in_viewport = !m.show_in_viewport);
    }

    /// `[ Reset Material Slot ]`: assigned objects keep the old material.
    pub fn reset_material_slot(&mut self) {
        self.checkpoint("Reset Material Slot");
        let hot = self.material_is_hot(self.active_material());
        self.scene.materials.reset_slot(self.mat_editor.slot);
        self.status = if hot {
            "Slot reset — objects keep the previous material (it is no longer in a slot)".into()
        } else {
            "Material slot reset".into()
        };
    }

    /// `[ Make Material Copy ]`: the slot becomes an unassigned ("cool") copy.
    pub fn make_material_copy(&mut self) {
        self.checkpoint("Make Material Copy");
        self.scene.materials.copy_slot(self.mat_editor.slot);
        self.status = "Material copied — the slot no longer edits assigned objects".into();
    }

    /// Eyedropper result: loads the object's material into the active slot.
    pub fn pick_material_from(&mut self, obj: ObjId) {
        let Some(id) = self.scene.objects.get(obj).and_then(|o| o.material) else {
            self.status = "Pick Material: that object has no material".into();
            return;
        };
        if let Some(slot) = self.scene.materials.slot_of(id) {
            self.mat_editor.slot = slot;
        } else {
            self.checkpoint("Pick Material");
            self.scene.materials.put_in_slot(self.mat_editor.slot, id);
        }
        self.status = format!("Picked \"{}\"", self.scene.materials.mats[id].name);
    }

    /// Quick Preset dropdown.
    pub fn apply_preset(&mut self, p: Preset) {
        self.edit_material(Some("Material Preset"), |m| p.apply(m));
        self.status = format!("Preset: {}", p.label());
    }

    /// Import PBR Texture Folder into the active material.
    pub fn import_pbr_folder(&mut self) {
        let dir = std::path::PathBuf::from(self.mat_editor.pbr_folder.trim());
        let mut m = self.scene.materials.mats[self.active_material()].clone();
        match crate::materials::import::import_folder(&mut m, &dir) {
            Ok(chs) => {
                self.edit_material(Some("Import PBR Folder"), |dst| *dst = m);
                let names: Vec<&str> = chs.iter().map(|c| c.label()).collect();
                self.status = format!("Imported {} maps: {}", chs.len(), names.join(", "));
            }
            Err(e) => self.status = e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::ShadingModel;

    fn editor_with_box() -> (Editor, ObjId) {
        let mut ed = Editor { scene: test_support::top_selected_box(), ..Default::default() };
        let id = ed.current().unwrap();
        ed.sub = crate::mesh::SubObject::Object;
        (ed, id)
    }

    #[test]
    fn assign_pick_and_undo() {
        let (mut ed, id) = editor_with_box();
        ed.mat_editor.slot = 1;
        ed.assign_material_to_selection();
        let steel = ed.scene.materials.slot(1);
        assert_eq!(ed.scene.objects[id].material, Some(steel));
        assert!(ed.material_is_hot(steel));
        ed.mat_editor.slot = 10;
        ed.pick_material_from(id);
        assert_eq!(ed.mat_editor.slot, 1, "picking a slotted material selects its slot");
        ed.undo();
        assert_eq!(ed.scene.objects[id].material, None);
    }

    #[test]
    fn copy_and_reset_keep_assigned_material() {
        let (mut ed, id) = editor_with_box();
        ed.mat_editor.slot = 6;
        ed.assign_material_to_selection();
        let cel = ed.active_material();
        ed.make_material_copy();
        assert_ne!(ed.active_material(), cel);
        assert!(!ed.material_is_hot(ed.active_material()));
        ed.reset_material_slot();
        assert_eq!(ed.scene.materials.mats[ed.active_material()].model, ShadingModel::Physical);
        assert_eq!(ed.scene.objects[id].material, Some(cel));
        assert_eq!(ed.scene.materials.mats[cel].model, ShadingModel::Toon);
        // Not in a slot any more: the eyedropper loads it into the active slot.
        ed.pick_material_from(id);
        assert_eq!(ed.active_material(), cel);
    }

    #[test]
    fn presets_and_edits_are_undoable() {
        let (mut ed, _) = editor_with_box();
        ed.mat_editor.slot = 12;
        ed.apply_preset(Preset::Crystal);
        assert_eq!(ed.scene.materials.mats[ed.active_material()].model, ShadingModel::Crystal);
        ed.edit_material(Some("Edit"), |m| m.roughness = 0.9);
        ed.undo();
        ed.undo();
        assert_eq!(ed.scene.materials.mats[ed.active_material()].model, ShadingModel::Physical);
    }
}
