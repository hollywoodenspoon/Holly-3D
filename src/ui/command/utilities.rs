//! Utilities tab: Reset XForm, Collapse, Measure and other one-shot tools.

use super::CONTENT_W;
use super::display::pairs;
use crate::editor::Editor;
use crate::ui::actions::Action;
use crate::ui::widgets::{action, dim, rollout};
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let any = !ed.scene.selected().is_empty();
    rollout(ui, "Utilities", true, |ui| {
        if action(ui, "Reset XForm", any, CONTENT_W) {
            ed.checkpoint("Reset XForm");
            for id in ed.scene.selected() {
                ed.scene.objects[id].reset_xform();
            }
        }
        if action(ui, "Collapse Stack to Editable Poly", ed.current().is_some(), CONTENT_W) {
            ed.collapse_stack();
        }
        pairs(ui, ed, "utils", &[Action::Measure, Action::SelectByName, Action::ConvertToPoly, Action::ConvertToSpline]);
        if !any {
            ui.label(dim("Select an object for Reset XForm and Collapse."));
        }
    });
    rollout(ui, "Program", true, |ui| {
        pairs(ui, ed, "program", &[Action::HotkeyList, Action::About]);
        ui.label(dim("Asset packs (photo, low-poly and realistic) arrive with the Material Library update."));
    });
}
