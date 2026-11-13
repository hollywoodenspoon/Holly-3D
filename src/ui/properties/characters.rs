//! Characters tab: human archetypes and creatures to place, the selected
//! character's parameters, and the MakeHuman and Face Cloner windows.

use super::content_w;
use crate::editor::Editor;
use crate::ui::character_studio;
use crate::ui::widgets::{dim, rollout};
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let w = content_w();
    let open = ed.iface.character_studio_open;
    if ui.add_sized([w, 28.0], egui::Button::new("♀ Open MakeHuman Window").selected(open)).clicked() {
        ed.iface.character_studio_open ^= true;
    }
    rollout(ui, "Quick Start", true, |ui| character_studio::gallery(ui, ed, 2));
    rollout(ui, "Selected Character", true, |ui| {
        if ed.character().is_some() {
            character_studio::params::base(ui, ed);
        } else {
            ui.label(dim("Select a character to edit its body, hair, outfit and weapon here."));
        }
    });
    rollout(ui, "Face Cloner", true, |ui| {
        let on = ed.iface.face_cloner_open;
        if ui.add_sized([w - 12.0, 26.0], egui::Button::new("📸 Build a Face from Photos...").selected(on)).clicked() {
            ed.iface.face_cloner_open ^= true;
        }
        ui.label(dim("Fits the selected human's face to your photos or video frames."));
    });
}
