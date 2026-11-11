//! Floating in-viewport Caddy: live-preview spinners + OK / Apply & Continue / Cancel.

use super::widgets::{max_spinner_range, toggle_button};
use crate::editor::{Editor, Tool};
use crate::mesh::ops::{ExtrudeMode, PolyOp};
use bevy_egui::egui::{self, RichText};

fn mode_picker(ui: &mut egui::Ui, mode: &mut ExtrudeMode) {
    ui.horizontal(|ui| {
        for m in ExtrudeMode::ALL {
            if toggle_button(ui, *mode == m, m.label()).clicked() {
                *mode = m;
            }
        }
    });
}

fn row<T: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, v: &mut T, speed: f64, default: T, range: std::ops::RangeInclusive<T>) {
    ui.label(label);
    max_spinner_range(ui, v, speed, default, range);
    ui.end_row();
}

/// Parameter editor for each op; returns false for ops without parameters.
pub fn params(ui: &mut egui::Ui, op: &mut PolyOp) -> bool {
    use PolyOp::*;
    let big = -1000.0..=1000.0;
    match op {
        ExtrudePolygons { mode, .. } | Bevel { mode, .. } => mode_picker(ui, mode),
        Inset { by_polygon, .. } => {
            ui.horizontal(|ui| {
                if toggle_button(ui, !*by_polygon, "Group").clicked() {
                    *by_polygon = false;
                }
                if toggle_button(ui, *by_polygon, "By Polygon").clicked() {
                    *by_polygon = true;
                }
            });
        }
        Hole { spec } => {
            ui.horizontal(|ui| {
                for k in crate::modifiers::cad::HoleKind::ALL {
                    if toggle_button(ui, spec.kind == k, k.label()).clicked() {
                        spec.kind = k;
                    }
                }
            });
        }
        Tessellate { face_center } => {
            ui.horizontal(|ui| {
                if toggle_button(ui, !*face_center, "Edge").clicked() {
                    *face_center = false;
                }
                if toggle_button(ui, *face_center, "Face-Center").clicked() {
                    *face_center = true;
                }
            });
            return true;
        }
        _ => {}
    }
    let mut any = true;
    egui::Grid::new("caddy_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| match op {
        ExtrudeVertices { height, width } | ExtrudeEdges { height, width } => {
            row(ui, "Height", height, 0.005, 0.2, big.clone());
            row(ui, "Width", width, 0.005, 0.1, 0.0..=100.0);
        }
        ExtrudePolygons { height, .. } => row(ui, "Height", height, 0.005, 0.25, big.clone()),
        Bevel { height, outline, .. } => {
            row(ui, "Height", height, 0.005, 0.2, big.clone());
            row(ui, "Outline", outline, 0.002, -0.05, big.clone());
        }
        Inset { amount, .. } => row(ui, "Amount", amount, 0.002, 0.05, 0.0..=100.0),
        Outline { amount } => row(ui, "Amount", amount, 0.002, 0.0, big.clone()),
        ChamferVertices { amount } => row(ui, "Amount", amount, 0.002, 0.1, 0.0..=100.0),
        ChamferEdges { amount, segments, tension, open } => {
            row(ui, "Amount", amount, 0.002, 0.1, 0.0..=100.0);
            row(ui, "Segments", segments, 0.1, 1, 1..=32);
            row(ui, "Tension", tension, 0.005, 0.5, 0.0..=1.0);
            ui.label("Open");
            ui.checkbox(open, "");
            ui.end_row();
        }
        PanelGroove { width, depth, lip, material_id } => {
            row(ui, "Groove Width", width, 0.001, 0.04, 0.0..=100.0);
            row(ui, "Groove Depth", depth, 0.001, 0.03, 0.0..=100.0);
            row(ui, "Bevel Lip Width", lip, 0.001, 0.008, 0.0..=100.0);
            row(ui, "Inner Groove Mat ID", material_id, 0.1, 2, 1..=64);
        }
        ConnectEdges { segments, pinch, slide } => {
            row(ui, "Segments", segments, 0.1, 1, 1..=32);
            row(ui, "Pinch", pinch, 0.5, 0.0, -100.0..=100.0);
            row(ui, "Slide", slide, 0.5, 0.0, -100.0..=100.0);
        }
        Weld { threshold, .. } => row(ui, "Threshold", threshold, 0.001, 0.01, 0.0..=100.0),
        BridgeBorders { segments, taper, twist } | BridgePolygons { segments, taper, twist } => {
            row(ui, "Segments", segments, 0.1, 1, 1..=64);
            row(ui, "Taper", taper, 0.005, 0.0, -1.0..=1.0);
            row(ui, "Twist", twist, 0.1, 0, -64..=64);
        }
        MSmooth { strength } => row(ui, "Smoothness", strength, 0.005, 0.5, 0.0..=1.0),
        Hinge { angle, segments, .. } => {
            row(ui, "Angle", angle, 0.5, 45.0, -360.0..=360.0);
            row(ui, "Segments", segments, 0.1, 1, 1..=64);
        }
        ExtrudeAlongSpline { segments, taper, twist, align, .. } => {
            row(ui, "Segments", segments, 0.1, 6, 1..=256);
            row(ui, "Taper Amount", taper, 0.005, 0.0, -1.0..=10.0);
            row(ui, "Twist", twist, 0.5, 0.0, -3600.0..=3600.0);
            ui.label("Align to Face Normal");
            ui.checkbox(align, "");
            ui.end_row();
        }
        Hole { spec } => {
            row(ui, "Diameter", &mut spec.diameter, 0.002, 0.1, 0.0001..=100.0);
            ui.label("Through All");
            ui.checkbox(&mut spec.through, "");
            ui.end_row();
            if !spec.through {
                row(ui, "Depth", &mut spec.depth, 0.002, 0.2, 0.0001..=100.0);
            }
            if spec.kind != crate::modifiers::cad::HoleKind::Simple {
                row(ui, "Top Diameter", &mut spec.top_diameter, 0.002, 0.18, 0.0001..=100.0);
            }
            match spec.kind {
                crate::modifiers::cad::HoleKind::Counterbore => row(ui, "Bore Depth", &mut spec.counterbore_depth, 0.002, 0.06, 0.0001..=100.0),
                crate::modifiers::cad::HoleKind::Countersink => row(ui, "Sink Angle", &mut spec.countersink_angle, 0.5, 90.0, 30.0..=150.0),
                _ => {}
            }
            row(ui, "Sides", &mut spec.sides, 0.1, 24, 6..=128);
        }
        _ => any = false,
    });
    any
}

/// Hinge / spline pick button for the pick-based caddies.
fn pick_row(ui: &mut egui::Ui, ed: &mut Editor, op: &PolyOp) {
    let (tool, label, picked) = match op {
        PolyOp::Hinge { axis, .. } => (Tool::PickHinge, "Pick Hinge", *axis != bevy::math::Vec3::ZERO),
        PolyOp::ExtrudeAlongSpline { path, .. } => (Tool::PickSpline, "Pick Spline", !path.is_empty()),
        _ => return,
    };
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new(label).selected(ed.tool == tool)).clicked() {
            ed.tool = tool;
        }
        ui.label(super::widgets::dim(if picked { "picked" } else { "none picked" }));
    });
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    let Some(mut caddy) = ed.caddy.clone() else {
        return;
    };
    let vr = ed.viewports.active_view().rect;
    let center = vr.center();
    let x = (center.x + 60.0).min(vr.max.x - 360.0).max(vr.min.x + 8.0);
    let mut action = None;
    egui::Window::new(RichText::new(caddy.op.name()).strong())
        .id(egui::Id::new("caddy"))
        .default_pos(egui::pos2(x, center.y - 60.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.set_min_width(210.0);
            params(ui, &mut caddy.op);
            pick_row(ui, ed, &caddy.op);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("✔ OK").clicked() {
                    action = Some(0);
                }
                if ui.button("➕ Apply & Continue").clicked() {
                    action = Some(1);
                }
                if ui.button("✖ Cancel").clicked() {
                    action = Some(2);
                }
            });
        });
    if let Some(c) = ed.caddy.as_mut() {
        c.op = caddy.op;
    }
    match action {
        Some(0) => {
            ed.update_caddy_preview();
            ed.commit_caddy();
        }
        Some(1) => {
            ed.update_caddy_preview();
            ed.apply_and_continue();
        }
        Some(2) => ed.cancel_caddy(),
        _ => {}
    }
}
