//! Edit Geometry rollout (all sub-object modes).

use super::{HALF, caddy_cell, later, op_cell};
use crate::editor::{Editor, Tool};
use crate::mesh::ops::PolyOp;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{action, dim, rollout};
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Edit Geometry", true, |ui| {
        let sub = ed.sub;
        let in_sub = ed.in_sub_object();
        let last = ed.last_op.as_ref().map(|o| o.name()).unwrap_or("—");
        if action(ui, &format!("Repeat Last  ({last})"), ed.last_op.is_some() && in_sub, CONTENT_W) {
            ed.repeat_last();
        }
        egui::Grid::new("eg").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            later(ui, "Create");
            op_cell(ui, ed, "Collapse", PolyOp::Collapse { level: sub }, in_sub);
            ui.end_row();
            let attaching = ed.tool == Tool::Attach;
            if ui
                .add_sized([HALF, 24.0], egui::Button::new("Attach").selected(attaching))
                .on_hover_text("Click an object to merge it")
                .clicked()
            {
                ed.tool = if attaching { Tool::None } else { Tool::Attach };
            }
            let poly_sel = ed.current_obj().is_some_and(|o| !o.level_mesh.selected_polygons().is_empty());
            let r = ui.add_enabled(poly_sel, egui::Button::new("Detach").min_size(egui::vec2(HALF, 24.0)));
            if r.clicked() {
                ed.detach(false);
            }
            r.context_menu(|ui| {
                if ui.button("Detach as Clone").clicked() {
                    ed.detach(true);
                    ui.close();
                }
            });
            ui.end_row();
        });
        ui.separator();
        let slicing = ed.slice_plane.is_some();
        egui::Grid::new("eg_slice").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            if ui.add_enabled(in_sub, egui::Button::new("Slice Plane").selected(slicing).min_size(egui::vec2(HALF, 24.0))).clicked() {
                ed.toggle_slice_plane();
            }
            let mut split = ed.slice_plane.as_ref().is_some_and(|s| s.split);
            if ui.add_enabled(slicing, egui::Checkbox::new(&mut split, "Split")).changed()
                && let Some(s) = ed.slice_plane.as_mut()
            {
                s.split = split;
            }
            ui.end_row();
            if ui.add_enabled(slicing, egui::Button::new("Slice").min_size(egui::vec2(HALF, 24.0))).clicked() {
                ed.slice_with_plane();
            }
            if ui.add_enabled(slicing, egui::Button::new("Reset Plane").min_size(egui::vec2(HALF, 24.0))).clicked() {
                ed.reset_slice_plane();
            }
            ui.end_row();
            tool(ui, ed, "QuickSlice", Tool::QuickSlice { start: None }, in_sub);
            tool(ui, ed, "Cut", Tool::Cut { last: None }, in_sub);
            ui.end_row();
            op_cell(ui, ed, "MSmooth", PolyOp::MSmooth { strength: 0.5 }, in_sub && sub.is_face_like());
            caddy_cell(ui, ed, "Tessellate", PolyOp::Tessellate { face_center: false }, in_sub && sub.is_face_like());
            ui.end_row();
        });
        if slicing {
            ui.label(dim("Move/rotate the yellow Slice Plane with W / E, then press Slice."));
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if ui.add_enabled(in_sub, egui::Button::new("Make Planar").min_size(egui::vec2(CONTENT_W - 3.0 * 34.0 - 12.0, 24.0))).clicked()
            {
                ed.apply_op(PolyOp::MakePlanar { level: sub });
            }
            for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
                if ui
                    .add_enabled(in_sub, egui::Button::new(*label).min_size(egui::vec2(30.0, 24.0)))
                    .on_hover_text(format!("Flatten selection to its average {label}"))
                    .clicked()
                {
                    ed.apply_op(PolyOp::Flatten { level: sub, axis });
                }
            }
        });
    });
}

fn tool(ui: &mut egui::Ui, ed: &mut Editor, label: &str, tool: Tool, enabled: bool) {
    let on = std::mem::discriminant(&ed.tool) == std::mem::discriminant(&tool);
    if ui.add_enabled(enabled, egui::Button::new(label).selected(on).min_size(egui::vec2(HALF, 24.0))).clicked() {
        ed.tool = if on { Tool::None } else { tool };
    }
}
