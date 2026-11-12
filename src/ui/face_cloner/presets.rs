//! The photo skin switch and Cloned Face Presets (Save / Load / Delete) in
//! the character's "Clone Face" rollout.

use crate::character::face_cloner::preset;
use crate::character::params::CharacterParams;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{dim, Track};
use bevy_egui::egui;

/// The photo skin rows (only when the clone baked one).
pub fn skin(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    let Some(tex) = &p.face.texture else {
        ui.label(dim("No photo skin (it is baked when \"Bake the photos into the skin\" is on in the Face Cloner)."));
        return;
    };
    let (w, h) = (tex.tex.width, tex.tex.height);
    let r = ui.checkbox(&mut p.face.use_texture, "Use Photo Skin").on_hover_text(format!("The photos baked into the skin texture ({w} x {h}) on Material ID 1 \"Skin: Cloned Face\""));
    if r.changed() {
        (t.started, t.changed) = (true, true);
    }
    if ui.add_sized([CONTENT_W, 22.0], egui::Button::new("Remove Photo Skin")).on_hover_text("Drops the baked skin and goes back to the character's own skin (the face shape stays)").clicked() {
        p.face.texture = None;
        (t.started, t.changed) = (true, true);
    }
}

/// Save / Load / Delete Cloned Face Preset.
pub fn presets(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    ui.separator();
    ui.label(egui::RichText::new("Cloned Face Presets").strong());
    let id = ui.id().with("face_preset");
    let (mut name, mut picked, mut note): (String, Option<String>, String) = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    let names = preset::list();
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut name).hint_text("Preset name").desired_width(CONTENT_W - 64.0));
        let can = p.face.views > 0 && !name.trim().is_empty();
        if ui.add_enabled(can, egui::Button::new("💾 Save")).on_hover_text("Saves this face (sliders, warp, photo skin) for any character").clicked() {
            note = match preset::of(&name, p).map(|f| preset::save(&f)) {
                Some(Ok(path)) => format!("Saved {}", path.display()),
                Some(Err(e)) => e,
                None => "Nothing to save yet".into(),
            };
            picked = Some(name.trim().to_string());
        }
    });
    let shown = picked.clone().filter(|n| names.contains(n)).unwrap_or_else(|| if names.is_empty() { "(none saved)".into() } else { "Pick a preset".into() });
    egui::ComboBox::from_id_salt(id.with("list")).width(CONTENT_W).selected_text(shown).show_ui(ui, |ui| {
        for n in &names {
            ui.selectable_value(&mut picked, Some(n.clone()), n);
        }
    });
    let chosen = picked.clone().filter(|n| names.contains(n));
    ui.horizontal(|ui| {
        if ui.add_enabled(chosen.is_some(), egui::Button::new("📂 Load onto Character")).on_hover_text("Replaces this character's face with the saved one (undoable)").clicked() {
            match preset::load(chosen.as_deref().unwrap_or_default()) {
                Ok(f) => {
                    *p = preset::apply(&f, p);
                    (t.started, t.changed) = (true, true);
                    note = format!("Loaded \"{}\"", f.name);
                }
                Err(e) => note = e,
            }
        }
        if ui.add_enabled(chosen.is_some(), egui::Button::new("🗑 Delete")).on_hover_text("Deletes the preset file").clicked() {
            note = match preset::delete(chosen.as_deref().unwrap_or_default()) {
                Ok(()) => "Deleted".into(),
                Err(e) => e,
            };
            picked = None;
        }
    });
    if !note.is_empty() {
        ui.label(dim(&note));
    }
    ui.data_mut(|d| d.insert_temp(id, (name, picked, note)));
}
