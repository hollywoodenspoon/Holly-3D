//! Edit Vertices / Edit Edges & Borders / Edit Polygons & Elements rollouts.

use super::{HALF, caddy_cell, op_cell};
use crate::editor::poly_tools::{along_spline_default, hinge_default};
use crate::editor::{Editor, Tool};
use crate::mesh::SubObject;
use crate::mesh::ops::{ExtrudeMode, PolyOp};
use crate::ui::widgets::{dim, rollout, split_button};
use bevy_egui::egui;

fn tool_cell(ui: &mut egui::Ui, ed: &mut Editor, label: &str, tool: Tool) {
    let on = std::mem::discriminant(&ed.tool) == std::mem::discriminant(&tool);
    if ui.add_sized([HALF, 24.0], egui::Button::new(label).selected(on)).clicked() {
        ed.tool = if on { Tool::None } else { tool };
    }
}

/// Split cell for pick-based ops: main picks and applies the remembered
/// settings, `[□]` opens the caddy (without a pick) and starts picking.
fn pick_cell(ui: &mut egui::Ui, ed: &mut Editor, label: &str, default: PolyOp, tool: Tool) {
    let (main, caddy) = split_button(ui, label, true, crate::ui::command::CONTENT_W);
    if main {
        ed.tool = tool;
    } else if caddy {
        let mut op = ed.remembered(default);
        match &mut op {
            PolyOp::Hinge { axis, .. } => *axis = bevy::math::Vec3::ZERO,
            PolyOp::ExtrudeAlongSpline { path, .. } => path.clear(),
            _ => {}
        }
        ed.open_caddy(op);
        ed.tool = tool;
    }
}

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([6.0, 6.0]).show(ui, body);
}

pub fn vertices(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Edit Vertices", true, |ui| {
        grid(ui, "ev", |ui| {
            op_cell(ui, ed, "Remove", PolyOp::RemoveVertices, true);
            op_cell(ui, ed, "Break", PolyOp::BreakVertices, true);
            ui.end_row();
            caddy_cell(ui, ed, "Extrude", PolyOp::ExtrudeVertices { height: 0.2, width: 0.1 }, true);
            caddy_cell(ui, ed, "Weld", PolyOp::Weld { level: SubObject::Vertex, threshold: 0.01 }, true);
            ui.end_row();
            caddy_cell(ui, ed, "Chamfer", PolyOp::ChamferVertices { amount: 0.1 }, true);
            tool_cell(ui, ed, "Target Weld", Tool::TargetWeld { first: None });
            ui.end_row();
            op_cell(ui, ed, "Connect", PolyOp::ConnectVertices, true);
            if ui.add_sized([HALF, 24.0], egui::Button::new("Remove Isolated")).clicked() {
                ed.status = "No isolated vertices: polygons always own their vertices".into();
            }
            ui.end_row();
            if ui.add_sized([HALF, 24.0], egui::Button::new("Remove Unused Map")).clicked() {
                ed.status = "UVs are stored per face corner — nothing unused to remove".into();
            }
            ui.end_row();
        });
    });
}

pub fn edges(ui: &mut egui::Ui, ed: &mut Editor) {
    let border = ed.sub == SubObject::Border;
    let title = if border { "Edit Borders" } else { "Edit Edges" };
    rollout(ui, title, true, |ui| {
        grid(ui, "ee", |ui| {
            tool_cell(ui, ed, "Insert Vertex", Tool::InsertVertex);
            let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            if ui
                .add_sized([HALF, 24.0], egui::Button::new("Remove"))
                .on_hover_text("Ctrl+Remove also removes the orphaned vertices")
                .clicked()
            {
                ed.apply_op(PolyOp::RemoveEdges { remove_verts: ctrl });
            }
            ui.end_row();
            op_cell(ui, ed, "Split", PolyOp::SplitEdges, true);
            caddy_cell(ui, ed, "Extrude", PolyOp::ExtrudeEdges { height: 0.2, width: 0.1 }, true);
            ui.end_row();
            caddy_cell(ui, ed, "Weld", PolyOp::Weld { level: SubObject::Edge, threshold: 0.01 }, true);
            tool_cell(ui, ed, "Target Weld", Tool::TargetWeld { first: None });
            ui.end_row();
            caddy_cell(ui, ed, "Chamfer", PolyOp::ChamferEdges { amount: 0.1, segments: 1, tension: 0.5, open: false }, true);
            caddy_cell(ui, ed, "Connect", PolyOp::ConnectEdges { segments: 1, pinch: 0.0, slide: 0.0 }, true);
            ui.end_row();
            caddy_cell(ui, ed, "Bridge", PolyOp::BridgeBorders { segments: 1, taper: 0.0, twist: 0 }, true);
            op_cell(ui, ed, "Cap", PolyOp::Cap, border);
            ui.end_row();
            if ui
                .add_sized([HALF, 24.0], egui::Button::new("Create Shape"))
                .on_hover_text("New Editable Spline from the selected edges")
                .clicked()
            {
                ed.create_shape_from_edges(ed.shape_smooth);
            }
            ui.horizontal(|ui| {
                ui.radio_value(&mut ed.shape_smooth, true, "Smooth");
                ui.radio_value(&mut ed.shape_smooth, false, "Linear");
            });
            ui.end_row();
        });
        if !border {
            ui.label(dim("Shift+Drag open edges with Move (W) to extrude new polygons."));
        }
    });
}

pub fn polygons(ui: &mut egui::Ui, ed: &mut Editor) {
    let title = if ed.sub == SubObject::Element { "Edit Elements" } else { "Edit Polygons" };
    rollout(ui, title, true, |ui| {
        grid(ui, "ep", |ui| {
            tool_cell(ui, ed, "Insert Vertex", Tool::InsertVertex);
            caddy_cell(ui, ed, "Extrude", PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 0.25 }, true);
            ui.end_row();
            caddy_cell(ui, ed, "Outline", PolyOp::Outline { amount: -0.05 }, true);
            caddy_cell(ui, ed, "Bevel", PolyOp::Bevel { mode: ExtrudeMode::Group, height: 0.2, outline: -0.05 }, true);
            ui.end_row();
            caddy_cell(ui, ed, "Inset", PolyOp::Inset { by_polygon: false, amount: 0.05 }, true);
            caddy_cell(ui, ed, "Bridge", PolyOp::BridgePolygons { segments: 1, taper: 0.0, twist: 0 }, true);
            ui.end_row();
            op_cell(ui, ed, "Flip", PolyOp::Flip, true);
            ui.end_row();
        });
        pick_cell(ui, ed, "Hinge From Edge", hinge_default(), Tool::PickHinge);
        pick_cell(ui, ed, "Extrude Along Spline", along_spline_default(), Tool::PickSpline);
    });
}
