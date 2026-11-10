//! Editable Spline Geometry tools: Refine (click on a segment) and the
//! Spline level (Outline, Mirror, Boolean, Reverse, Close, Delete).

use super::HALF;
use super::spline::spin_action;
use crate::editor::{Editor, Tool, types::SplineUi};
use crate::splines::boolean::SplineBoolOp;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{action, dim};
use bevy_egui::egui;

fn toggle(ui: &mut egui::Ui, ed: &mut Editor, label: &str, tool: Tool, width: f32, hint: &str) {
    let on = std::mem::discriminant(&ed.tool) == std::mem::discriminant(&tool);
    if ui.add(egui::Button::new(label).selected(on).min_size(egui::vec2(width, 24.0))).on_hover_text(hint).clicked() {
        ed.tool = if on { Tool::None } else { tool };
    }
}

/// Vertex level: Refine adds a vertex where a segment is clicked.
pub fn refine_button(ui: &mut egui::Ui, ed: &mut Editor) {
    toggle(ui, ed, "Refine", Tool::RefineSpline, CONTENT_W, "Click on segments to add vertices without changing the curve");
}

pub fn spline_level(ui: &mut egui::Ui, ed: &mut Editor, st: &mut SplineUi) {
    if spin_action(ui, "Outline", &mut st.outline, 0.05) {
        ed.edit_shape("Outline", |s| s.outline_selected(st.outline));
    }
    ui.horizontal(|ui| {
        for (i, l) in ["↔", "↕", "✣"].iter().enumerate() {
            ui.radio_value(&mut st.mirror_axis, i, *l);
        }
        ui.checkbox(&mut st.mirror_copy, "Copy");
        if action(ui, "Mirror", true, 80.0) {
            ed.edit_shape("Mirror", |s| s.mirror_selected(st.mirror_axis, st.mirror_copy));
        }
    });
    ui.horizontal(|ui| {
        toggle(
            ui,
            ed,
            "Boolean",
            Tool::PickSplineBoolean,
            80.0,
            "Select a closed spline, then click another closed spline of the same shape",
        );
        for op in SplineBoolOp::ALL {
            let b = egui::Button::new(op.symbol()).selected(st.bool_op == op).min_size(egui::vec2(28.0, 24.0));
            if ui.add(b).on_hover_text(op.label()).clicked() {
                st.bool_op = op;
            }
        }
    });
    ui.label(dim(&format!("Boolean ({}): select a closed spline, then click the other one.", st.bool_op.label())));
    ui.horizontal(|ui| {
        if action(ui, "Reverse", true, HALF) {
            ed.edit_shape("Reverse", |s| s.splines.iter_mut().filter(|s| s.selected).for_each(|s| s.reverse()));
        }
        if action(ui, "Close", true, HALF) {
            ed.edit_shape("Close", |s| s.splines.iter_mut().filter(|s| s.selected && s.knots.len() > 2).for_each(|s| s.closed = true));
        }
    });
    if action(ui, "Delete", true, CONTENT_W) {
        ed.delete_selection();
    }
}
