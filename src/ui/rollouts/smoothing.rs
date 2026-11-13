//! Polygon: Material IDs + the 4x8 Smoothing Group grid.

use crate::editor::Editor;
use crate::mesh::ops::PolyOp;
use crate::ui::properties::content_w;
use crate::ui::theme;
use crate::ui::widgets::{max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Polygon: Material IDs", true, |ui| {
        ui.horizontal(|ui| {
            ui.label("Set ID:");
            max_spinner_range(ui, &mut ed.set_id, 0.1, 1, 1..=64);
            if ui.button("Apply").clicked() {
                let id = ed.set_id;
                ed.apply_op(PolyOp::SetMaterialId { id });
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Select ID").clicked() {
                let id = ed.select_id;
                let add = ui.input(|i| i.modifiers.ctrl);
                ed.edit_selection(|m, _| m.select_by_material_id(id, add));
            }
            max_spinner_range(ui, &mut ed.select_id, 0.1, 1, 1..=64);
        });
    });
    rollout(ui, "Polygon: Smoothing Groups", true, |ui| {
        let (all, any) = ed
            .current_obj()
            .map(|o| {
                let sel: Vec<u32> = o.level_mesh.polygons.values().filter(|p| p.selected).map(|p| p.smoothing_group).collect();
                if sel.is_empty() { (0, 0) } else { (sel.iter().fold(u32::MAX, |a, b| a & b), sel.iter().fold(0, |a, b| a | b)) }
            })
            .unwrap_or((0, 0));
        let cell = (content_w() - 7.0 * 3.0) / 8.0;
        egui::Grid::new("sg_grid").num_columns(8).spacing([3.0, 3.0]).show(ui, |ui| {
            for bit in 0..32u32 {
                let on = all & (1 << bit) != 0;
                let partial = !on && any & (1 << bit) != 0;
                let text = RichText::new(format!("{}", bit + 1)).size(11.0);
                let text = if on {
                    text.color(theme::input()).strong()
                } else if partial {
                    text.color(theme::amber())
                } else {
                    text
                };
                let r = ui.add_sized([cell, 22.0], egui::Button::new(text).selected(on));
                if r.clicked() {
                    ed.apply_op(PolyOp::SetSmoothing { bit, on: !on });
                }
                if bit % 8 == 7 {
                    ui.end_row();
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Clear All").clicked() {
                ed.apply_op(PolyOp::ClearSmoothing);
            }
            if ui.button("Auto Smooth").clicked() {
                let threshold = ed.autosmooth_threshold;
                ed.apply_op(PolyOp::AutoSmooth { threshold });
            }
            max_spinner_range(ui, &mut ed.autosmooth_threshold, 0.5, 45.0, 0.0..=180.0);
            ui.label("°");
        });
        ui.horizontal(|ui| {
            if ui.button("Select By SG").clicked() {
                let bits = all.max(any);
                ed.edit_selection(|m, _| m.select_by_smoothing_group(bits));
            }
        });
    });
}
