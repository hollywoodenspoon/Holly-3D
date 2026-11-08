//! Pivot tab.

use super::CONTENT_W;
use crate::editor::Editor;
use crate::ui::widgets::{action, dim, rollout, toggle_button};
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let has = ed.current().is_some();
    rollout(ui, "Adjust Pivot", true, |ui| {
        ui.add_enabled_ui(has, |ui| {
            if toggle_button(ui, ed.affect_pivot_only, "Affect Pivot Only").clicked() {
                ed.affect_pivot_only = !ed.affect_pivot_only;
            }
            ui.add_space(4.0);
            let mut run = |ui: &mut egui::Ui, label: &str, f: fn(&mut crate::scene::SceneObject)| {
                if action(ui, label, true, CONTENT_W) {
                    ed.checkpoint(label);
                    for id in ed.scene.selected() {
                        f(&mut ed.scene.objects[id]);
                    }
                }
            };
            run(ui, "Center to Object", |o| o.center_pivot());
            run(ui, "Align to World", |o| o.align_pivot_to_world());
            run(ui, "Bottom-Center Pivot (Z=0)", |o| o.bottom_center_pivot());
            run(ui, "Reset XForm", |o| o.reset_xform());
        });
        if !has {
            ui.label(dim("Select an object first."));
        }
    });
}
