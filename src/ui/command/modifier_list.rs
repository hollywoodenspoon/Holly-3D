//! "Modifier List ⏷": every modifier in one tall scrolling list, grouped
//! under headings, plus 3ds Max's Show Buttons modifier set.

use super::CONTENT_W;
use crate::editor::Editor;
use crate::modifiers::catalog::{self, GROUPS};
use crate::ui::UiState;
use crate::ui::menubar::modifier_row;
use crate::ui::theme;
use crate::ui::widgets::{dim, toggle_button};
use bevy_egui::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UiState) {
    ui.horizontal(|ui| {
        let caption = if st.modifier_list_open { "Modifier List ⏶" } else { "Modifier List ⏷" };
        let r = ui.add_sized([CONTENT_W - 64.0, 26.0], egui::Button::new(caption).selected(st.modifier_list_open));
        if r.on_hover_text("Every modifier, grouped: scroll and click one to add it on top of the stack").clicked() {
            st.modifier_list_open = !st.modifier_list_open;
        }
        let tip = "Show Buttons: a grid of your favourite modifiers under the list";
        if toggle_button(ui, ed.iface.modifier_buttons, "Buttons").on_hover_text(tip).clicked() {
            ed.iface.modifier_buttons = !ed.iface.modifier_buttons;
        }
    });
    if st.modifier_list_open {
        list(ui, ed, st);
    }
    if ed.iface.modifier_buttons {
        button_set(ui, ed);
    }
}

fn list(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UiState) {
    let mut added = false;
    egui::Frame::new().fill(theme::input()).corner_radius(4).inner_margin(6).show(ui, |ui| {
        ui.set_width(CONTENT_W - 12.0);
        ui.add(egui::TextEdit::singleline(&mut ed.modifier_filter).hint_text("Filter (optional)…").desired_width(f32::INFINITY));
        let filter = ed.modifier_filter.to_lowercase();
        egui::ScrollArea::vertical().max_height(380.0).id_salt("modlist").show(ui, |ui| {
            for (group, items) in GROUPS {
                let shown: Vec<_> = items.iter().filter(|i| filter.is_empty() || i.name.to_lowercase().contains(&filter)).collect();
                if shown.is_empty() {
                    continue;
                }
                ui.label(RichText::new(group.to_uppercase()).color(theme::amber()).small().strong());
                for info in shown {
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        added |= modifier_row(ui, ed, info.name, info.tip);
                    });
                }
                ui.add_space(4.0);
            }
        });
    });
    if added {
        ed.modifier_filter.clear();
        st.modifier_list_open = false;
    }
}

/// The Show Buttons grid; right-click the header to choose its modifiers.
fn button_set(ui: &mut egui::Ui, ed: &mut Editor) {
    let header = ui.label(dim("Modifier set (right-click to edit)"));
    header.context_menu(|ui| {
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (group, items) in GROUPS {
                ui.label(RichText::new(*group).color(theme::amber()).small());
                for info in *items {
                    let mut on = ed.iface.modifier_set.iter().any(|n| n == info.name);
                    if ui.checkbox(&mut on, info.name).changed() {
                        if on {
                            ed.iface.modifier_set.push(info.name.to_string());
                        } else {
                            ed.iface.modifier_set.retain(|n| n != info.name);
                        }
                    }
                }
            }
        });
        if ui.button("Reset to default set").clicked() {
            ed.iface.modifier_set = crate::editor::interface::DEFAULT_MODIFIER_SET.iter().map(|s| s.to_string()).collect();
        }
    });
    let names = ed.iface.modifier_set.clone();
    egui::Grid::new("mod_set").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        for (i, name) in names.iter().enumerate() {
            let fit = catalog::fit(name, ed.current().map(|id| &ed.scene.objects[id]));
            let tip = catalog::info(name).map_or("", |i| i.tip);
            let enabled = !matches!(fit, catalog::Fit::No(_));
            let r = ui.add_enabled(enabled, egui::Button::new(name.as_str()).min_size(egui::vec2(CONTENT_W / 2.0 - 8.0, 26.0)));
            if r.on_hover_text(tip).clicked() {
                ed.add_modifier(name);
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}
