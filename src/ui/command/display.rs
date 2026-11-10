//! Display tab: hide and freeze by category or selection, and the display
//! properties of the active viewport and the selected objects.

use super::CONTENT_W;
use crate::editor::{Editor, ObjCategory};
use crate::ui::action_ui::sized_button;
use crate::ui::actions::Action;
use crate::ui::widgets::{dim, rollout};
use bevy_egui::egui;

/// Two big action buttons per row.
pub fn pairs(ui: &mut egui::Ui, ed: &mut Editor, id: &str, actions: &[Action]) {
    let size = egui::vec2(CONTENT_W / 2.0 - 4.0, 26.0);
    egui::Grid::new(id).num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        for (i, a) in actions.iter().enumerate() {
            sized_button(ui, ed, *a, &a.label(), size);
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Hide by Category", true, |ui| {
        egui::Grid::new("hide_cat").num_columns(3).spacing([6.0, 4.0]).show(ui, |ui| {
            for cat in &ObjCategory::ALL[1..] {
                let total = ed.scene.objects.values().filter(|o| cat.accepts(o)).count();
                let hidden = ed.scene.objects.values().filter(|o| cat.accepts(o) && o.hidden).count();
                ui.label(format!("{} ({hidden}/{total} hidden)", cat.label()));
                if ui.add_enabled(hidden < total, egui::Button::new("Hide")).clicked() {
                    ed.hide_category(*cat, true);
                }
                if ui.add_enabled(hidden > 0, egui::Button::new("Unhide")).clicked() {
                    ed.hide_category(*cat, false);
                }
                ui.end_row();
            }
        });
    });
    rollout(ui, "Hide", true, |ui| {
        pairs(ui, ed, "hide", &[Action::HideSelected, Action::HideUnselected, Action::UnhideAll, Action::Isolate]);
    });
    rollout(ui, "Freeze", true, |ui| {
        pairs(ui, ed, "freeze", &[Action::FreezeSelected, Action::UnfreezeAll]);
        ui.label(dim("Frozen objects stay visible but cannot be selected."));
    });
    rollout(ui, "Display Properties", true, |ui| {
        pairs(ui, ed, "disp_props", &[Action::SeeThrough, Action::EdgedFaces, Action::Wireframe, Action::ToggleGrid]);
        ui.label(dim("See-Through applies to the selection; the others to the active viewport."));
    });
}
