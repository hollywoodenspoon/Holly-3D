//! 3D Snaps UI: the Grid and Snap Settings popup (right-click `S`) and the
//! cyan crosshair drawn at the snap point under the cursor.

use crate::editor::Editor;
use crate::editor::snap::SnapSettings;
use crate::ui::widgets::max_spinner_range;
use bevy::math::Vec2;
use bevy_egui::egui::{self, Color32, FontId, Stroke};

pub const CYAN: Color32 = Color32::from_rgb(0x2E, 0xE6, 0xF0);

/// Grid and Snap Settings: snap types, grid spacing and snap strength.
pub fn settings(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.set_min_width(190.0);
    ui.label(egui::RichText::new("Snaps").strong());
    let c = &mut ed.snap_cfg;
    for (on, label) in [
        (&mut c.grid, "Grid Points"),
        (&mut c.pivot, "Pivot"),
        (&mut c.vertex, "Vertex"),
        (&mut c.edge, "Edge/Segment"),
        (&mut c.midpoint, "Midpoint"),
        (&mut c.face, "Face Center"),
    ] {
        ui.checkbox(on, label);
    }
    ui.separator();
    let d = SnapSettings::default();
    egui::Grid::new("snap_opts").num_columns(2).show(ui, |ui| {
        ui.label("Grid Spacing");
        max_spinner_range(ui, &mut c.grid_size, 0.01, d.grid_size, 0.01..=100.0);
        ui.end_row();
        ui.label("Snap Strength (px)");
        max_spinner_range(ui, &mut c.radius, 0.5, d.radius, 2.0..=50.0);
        ui.end_row();
    });
    ui.horizontal(|ui| {
        if ui.button("Clear All").clicked() {
            *c = SnapSettings { grid: false, pivot: false, vertex: false, edge: false, midpoint: false, face: false, ..c.clone() };
        }
        if ui.button("Defaults").clicked() {
            *c = d;
        }
    });
}

/// Cyan crosshair + snap type at the snap point under the cursor (`S` on).
pub fn crosshair(painter: &egui::Painter, ed: &Editor, vi: usize, pointer: Vec2) {
    let Some(h) = ed.snap_point(vi, pointer) else { return };
    let Some(s) = ed.viewports.views[vi].to_screen(h.pos) else { return };
    let c = egui::pos2(s.x, s.y);
    let st = Stroke::new(1.5, CYAN);
    let r = 9.0;
    painter.line_segment([c - egui::vec2(r, 0.0), c + egui::vec2(r, 0.0)], st);
    painter.line_segment([c - egui::vec2(0.0, r), c + egui::vec2(0.0, r)], st);
    painter.circle_stroke(c, 4.0, st);
    painter.text(c + egui::vec2(10.0, 8.0), egui::Align2::LEFT_TOP, h.kind.label(), FontId::proportional(11.0), CYAN);
}
