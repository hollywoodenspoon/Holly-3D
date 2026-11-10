//! Right Command Panel (310px): Create | Modify | Hierarchy | Display | Utilities.

mod assembly;
mod camera_params;
mod create;
mod display;
mod light_params;
mod modifier_list;
mod modify;
mod params;
mod pivot;
mod stack;
mod utilities;

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
                ui.spacing_mut().item_spacing.x = 3.0;
                let tabs = [
                    (CommandTab::Create, "✚", "Create"),
                    (CommandTab::Modify, "🛠", "Modify"),
                    (CommandTab::Hierarchy, "⌖", "Hierarchy: pivot and assembly"),
                    (CommandTab::Display, "👁", "Display: hide, freeze and display properties"),
                    (CommandTab::Utilities, "⚒", "Utilities: Reset XForm, Collapse, Measure"),
                ];
                let w = (CONTENT_W + 10.0) / tabs.len() as f32 - 3.0;
                for (tab, icon, tip) in tabs {
                    let caption = if ed.tab == tab { format!("{icon} {}", tip.split(':').next().unwrap_or(tip)) } else { icon.to_string() };
                    let width = if ed.tab == tab { w * 2.0 } else { w * 0.75 };
                    let r = ui.add_sized([width, 26.0], egui::Button::new(caption).selected(ed.tab == tab)).on_hover_text(tip);
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
                    CommandTab::Hierarchy => pivot::show(ui, ed),
                    CommandTab::Display => display::show(ui, ed),
                    CommandTab::Utilities => utilities::show(ui, ed),
                }
            });
        })
        .response
        .rect
}
