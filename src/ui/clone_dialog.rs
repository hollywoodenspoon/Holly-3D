//! Clone Options modal (after Shift+Drag in Object mode).

use super::widgets::{max_spinner_range, toggle_button};
use crate::editor::Editor;
use crate::scene::CloneKind;
use bevy_egui::egui::{self, RichText};

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    let Some(mut d) = ed.clone_dialog.clone() else { return };
    let mut result = None;
    egui::Modal::new(egui::Id::new("clone_options")).show(ctx, |ui| {
        ui.set_width(260.0);
        ui.label(RichText::new("Clone Options").strong().size(15.0));
        ui.add_space(6.0);
        ui.label("Object:");
        ui.horizontal(|ui| {
            for k in CloneKind::ALL {
                if toggle_button(ui, d.kind == k, k.label()).clicked() {
                    d.kind = k;
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Number of Copies:");
            max_spinner_range(ui, &mut d.copies, 0.1, 1, 1..=500);
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("  OK  ").clicked() {
                result = Some(true);
            }
            if ui.button("Cancel").clicked() {
                result = Some(false);
            }
        });
    });
    ed.clone_dialog = Some(d);
    match result {
        Some(true) => ed.confirm_clone(),
        Some(false) => ed.clone_dialog = None,
        None => {}
    }
}
