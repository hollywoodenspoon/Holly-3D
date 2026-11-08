//! Floating in-viewport Caddy: live-preview spinners + OK / Apply & Continue / Cancel.

use super::widgets::{max_spinner_range, toggle_button};
use crate::editor::Editor;
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
        _ => any = false,
    });
    any
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
