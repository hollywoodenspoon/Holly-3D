//! "Render Turntable / Sprite Sheet" floating dialog (F10 and F9 toolbar).

use crate::editor::Editor;
use crate::render::turntable::{PITCHES, SheetMode};
use crate::ui::widgets::{dim, max_spinner_range};
use bevy_egui::egui;

pub fn window(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.rfw.sheet.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Render Turntable / Sprite Sheet").open(&mut open).resizable(false).default_width(340.0).show(ctx, |ui| body(ui, ed));
    ed.rfw.sheet.open &= open;
}

fn body(ui: &mut egui::Ui, ed: &mut Editor) {
    let busy = ed.rfw.sheet.job.is_some();
    ui.add_enabled_ui(!busy, |ui| {
        let s = &mut ed.rfw.sheet.settings;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut s.mode, SheetMode::Turntable, "🎞 Turntable");
            ui.selectable_value(&mut s.mode, SheetMode::Sprites, "Isometric Sprite Sheet");
        });
        egui::Grid::new("sheet_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            match s.mode {
                SheetMode::Turntable => {
                    ui.label("Frames");
                    max_spinner_range(ui, &mut s.frames, 0.2, 24, 8..=72);
                    ui.end_row();
                }
                SheetMode::Sprites => {
                    ui.label("Directions");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut s.dirs, 4, "4-Dir");
                        ui.selectable_value(&mut s.dirs, 8, "8-Dir");
                    });
                    ui.end_row();
                    ui.label("Camera Pitch");
                    egui::ComboBox::from_id_salt("sheet_pitch").selected_text(format!("{:.3}°", s.pitch)).show_ui(ui, |ui| {
                        for (p, label) in PITCHES {
                            ui.selectable_value(&mut s.pitch, p, label);
                        }
                    });
                    ui.end_row();
                    ui.label("Cell Size (px)");
                    max_spinner_range(ui, &mut s.cell, 1.0, 256, 16..=2048);
                    ui.end_row();
                }
            }
            ui.label("Folder");
            ui.text_edit_singleline(&mut s.folder);
            ui.end_row();
            ui.label("Name");
            ui.text_edit_singleline(&mut s.name);
            ui.end_row();
        });
        ui.checkbox(&mut s.selection_only, "Frame Selection (else whole scene)");
        ui.checkbox(&mut s.transparent, "Transparent Background (Alpha)");
        ui.checkbox(&mut s.outline, "Universal Ink Outline");
    });
    let s = &ed.rfw.sheet.settings;
    let info = match s.mode {
        SheetMode::Turntable => format!("{} numbered PNGs at {}x{} (F10 resolution).", s.frames, ed.render.width, ed.render.height),
        SheetMode::Sprites => format!("One atlas PNG + .json, {} cells of {}px (N, NE, E…).", s.dirs, s.cell),
    };
    ui.label(dim(&info));
    ui.separator();
    ui.horizontal(|ui| match &ed.rfw.sheet.job {
        Some(j) => {
            ui.add(egui::ProgressBar::new(j.progress()).desired_width(ui.available_width().min(200.0)).show_percentage());
            if ui.button("■ Cancel").clicked() {
                ed.cancel_sheet();
            }
        }
        None => {
            if ui.button(egui::RichText::new("🎞 Render").strong()).clicked() {
                ed.start_sheet();
            }
        }
    });
    if !ed.rfw.sheet.message.is_empty() {
        ui.label(dim(&ed.rfw.sheet.message));
    }
}
