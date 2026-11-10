//! Multi/Sub-Object Basic Parameters: `[ Set Number ]`, then one row per
//! sub-material (ID #, name, color swatch, On/Off, click to edit).

use super::edit_root;
use crate::editor::Editor;
use crate::materials::multi::MAX_SUBS;
use crate::ui::theme;
use crate::ui::widgets::{dim, rollout};
use bevy_egui::egui::{self, RichText};

const LIST_H: f32 = 230.0;
/// Sub-material name column (wide enough for names like "Cliff Rock").
const NAME_W: f32 = 132.0;
/// Sub-Material button column.
const SUB_W: f32 = 100.0;

fn set_number(ui: &mut egui::Ui, ed: &mut Editor, count: usize) {
    let id = egui::Id::new("multi_set_number");
    let mut n: usize = ui.data(|d| d.get_temp(id)).unwrap_or(count);
    ui.horizontal(|ui| {
        if ui.button("Set Number").on_hover_text("Grow or shrink the sub-material list").clicked() && n != count {
            ed.edit_material_root(Some("Set Number"), |m| {
                if let Some(ms) = m.multi.as_mut() {
                    ms.set_number(n);
                }
            });
            ed.mat_editor.sub = ed.mat_editor.sub.min(n.saturating_sub(1));
        }
        ui.add(egui::DragValue::new(&mut n).range(1..=MAX_SUBS).speed(0.1));
        ui.label(dim(&format!("{count} sub-materials")));
    });
    ui.data_mut(|d| d.insert_temp(id, n));
}

fn rows(ui: &mut egui::Ui, ed: &mut Editor) {
    let current = ed.mat_editor.sub;
    let mut pick = None;
    edit_root(ed, |m, t| {
        let ms = m.multi.as_mut()?;
        egui::ScrollArea::vertical().id_salt("multi_rows").max_height(LIST_H).show(ui, |ui| {
            egui::Grid::new("multi_grid").num_columns(5).spacing([6.0, 4.0]).striped(true).show(ui, |ui| {
                for h in ["ID", "Name", "Color", "On", "Sub-Material"] {
                    ui.label(RichText::new(h).small().strong());
                }
                ui.end_row();
                for (i, s) in ms.subs.iter_mut().enumerate() {
                    t.r(ui.add(egui::DragValue::new(&mut s.id).range(1..=999).speed(0.1)));
                    t.r(ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(NAME_W)));
                    t.r(ui.color_edit_button_rgb(&mut s.mat.base_color));
                    t.r(ui.checkbox(&mut s.enabled, ""));
                    let label = format!("{} ({})", s.mat.name, s.mat.model.label().split(" /").next().unwrap_or(""));
                    let text = if i == current { RichText::new(label).color(theme::INPUT).strong() } else { RichText::new(label) };
                    if ui.add(egui::Button::new(text).selected(i == current).truncate().min_size(egui::vec2(SUB_W, 0.0))).on_hover_text("Edit this sub-material").clicked() {
                        pick = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        None
    });
    if let Some(i) = pick {
        ed.mat_editor.sub = i;
    }
}

/// The Multi/Sub-Object list (only shown for Multi/Sub-Object slots).
pub fn rollout_ui(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(count) = ed.scene.materials.mats[ed.active_material()].multi.as_ref().map(|m| m.subs.len()) else { return };
    rollout(ui, "Multi/Sub-Object Basic Parameters", true, |ui| {
        set_number(ui, ed, count);
        rows(ui, ed);
        ui.label(dim("Polygons render with the sub-material matching their Material ID; higher IDs wrap around the list."));
    });
    let sub = ed.mat_editor.sub.min(count - 1);
    let m = &ed.scene.materials.mats[ed.active_material()];
    let s = &m.multi.as_ref().expect("multi").subs[sub];
    ui.label(RichText::new(format!("Editing ID {}: {}", s.id, s.name)).strong().color(theme::AMBER));
}
