//! Right Command Panel (310px): Create | Modify | Pivot.

mod camera_params;
mod create;
mod light_params;
mod modify;
mod params;
mod pivot;
mod stack;

use super::UiState;
use super::theme;
use crate::editor::{CommandTab, Editor};
use bevy_egui::egui;

pub const PANEL_W: f32 = 310.0;
/// Usable content width inside the panel.
pub const CONTENT_W: f32 = PANEL_W - 46.0;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UiState) -> egui::Rect {
    egui::Panel::right("command_panel")
        .resizable(false)
        .show_separator_line(false)
        .exact_size(PANEL_W)
        .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::same(10)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [(CommandTab::Create, "➕ Create"), (CommandTab::Modify, "🌈 Modify"), (CommandTab::Pivot, "⌖ Pivot")]
                {
                    let w = (CONTENT_W + 10.0) / 3.0 - 4.0;
                    let r = ui.add_sized([w, 26.0], egui::Button::new(label).selected(ed.tab == tab));
                    if r.clicked() {
                        ed.tab = tab;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(CONTENT_W + 16.0);
                ui.spacing_mut().button_padding = egui::vec2(5.0, 4.0);
                match ed.tab {
                    CommandTab::Create => create::show(ui, ed),
                    CommandTab::Modify => modify::show(ui, ed, st),
                    CommandTab::Pivot => pivot::show(ui, ed),
                }
            });
        })
        .response
        .rect
}
