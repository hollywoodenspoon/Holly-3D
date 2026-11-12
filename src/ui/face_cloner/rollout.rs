//! "📸 Clone Face from Photos / Videos" rollout on a selected character:
//! the way into the Face Cloner window, plus the cloned face's own
//! controls (warp strength, reset).

use crate::character::face_cloner::clone::reset;
use crate::character::params::CharacterParams;
use crate::editor::interface::InterfaceState;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, rollout};
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, iface: &mut InterfaceState, t: &mut Track, p: &mut CharacterParams) {
    rollout(ui, "📸 Clone Face from Photos / Videos", false, |ui| {
        if ui.add_sized([CONTENT_W, 26.0], egui::Button::new("📂 Load Photos / Frame Folder...")).on_hover_text("Opens the Face Cloner and asks for photos").clicked() {
            iface.face_cloner_open = true;
            iface.face_load_request = true;
        }
        if ui.add_sized([CONTENT_W, 22.0], egui::Button::new("Open Face Cloner...").selected(iface.face_cloner_open)).clicked() {
            iface.face_cloner_open ^= true;
        }
        if p.face.views == 0 {
            ui.label(dim("No cloned face yet. Load a few sharp photos (front, three-quarter, profile) or a folder of frames from a slow head turn."));
            return;
        }
        ui.label(format!("Cloned from {} picture(s).", p.face.views));
        ui.horizontal(|ui| {
            ui.add_sized([110.0, 18.0], egui::Label::new("Warp Strength"));
            let r = t.r(ui.add(egui::Slider::new(&mut p.face.strength, 0.0..=1.5).max_decimals(2)));
            if r.secondary_clicked() {
                p.face.strength = 1.0;
                t.changed = true;
            }
        });
        ui.label(dim("The fitted slider values are in Detail Sliders (head, nose, mouth, eyes…)."));
        if ui.add_sized([CONTENT_W, 22.0], egui::Button::new("Reset Cloned Face")).on_hover_text("Clears the face sliders and the fine warp").clicked() {
            *p = reset(p);
            t.started = true;
            t.changed = true;
        }
    });
}
