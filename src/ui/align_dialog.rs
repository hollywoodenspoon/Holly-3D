//! Tools -> Align (Alt+A): the Align Selection dialog with
//! position / orientation / scale options, previewed live.

use super::widgets::dim;
use crate::editor::Editor;
use crate::editor::align::{AlignPoint, AlignSettings};
use bevy_egui::egui::{self, RichText};

fn xyz(ui: &mut egui::Ui, flags: &mut [bool; 3], prefix: &str) {
    ui.horizontal(|ui| {
        for (a, f) in ["X", "Y", "Z"].iter().zip(flags.iter_mut()) {
            ui.checkbox(f, format!("{a} {prefix}"));
        }
    });
}

fn points(ui: &mut egui::Ui, title: &str, v: &mut AlignPoint) {
    ui.vertical(|ui| {
        ui.label(RichText::new(title).strong());
        for p in AlignPoint::ALL {
            ui.radio_value(v, p, p.label());
        }
    });
}

fn body(ui: &mut egui::Ui, s: &mut AlignSettings) {
    ui.label(RichText::new("Align Position (World)").strong());
    xyz(ui, &mut s.pos, "Position");
    ui.columns(2, |c| {
        points(&mut c[0], "Current Object:", &mut s.current);
        points(&mut c[1], "Target Object:", &mut s.target);
    });
    ui.separator();
    ui.label(RichText::new("Align Orientation (Local)").strong());
    xyz(ui, &mut s.orient, "Axis");
    ui.separator();
    ui.label(RichText::new("Match Scale").strong());
    xyz(ui, &mut s.scale, "Axis");
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    let Some(d) = ed.align_dialog.as_ref() else { return };
    let mut s = d.settings.clone();
    let target = ed.scene.objects.get(d.target).map(|o| o.name.clone()).unwrap_or_default();
    let mut result = None;
    egui::Window::new(format!("Align Selection ({target})"))
        .id(egui::Id::new("align_dialog"))
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center() - egui::vec2(140.0, 180.0))
        .show(ctx, |ui| {
            ui.set_width(280.0);
            body(ui, &mut s);
            ui.label(dim("Apply keeps the result and starts a new alignment."));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() {
                    result = Some(0);
                }
                if ui.button("  OK  ").clicked() {
                    result = Some(1);
                }
                if ui.button("Cancel").clicked() {
                    result = Some(2);
                }
            });
        });
    if let Some(d) = &mut ed.align_dialog {
        d.settings = s;
    }
    match result {
        Some(0) => ed.apply_align(),
        Some(1) => ed.confirm_align(),
        Some(2) => ed.cancel_align(),
        _ => ed.update_align_preview(),
    }
}
