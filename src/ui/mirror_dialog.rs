//! Tools -> Mirror (3ds Max Mirror dialog): Mirror Axis, Offset and
//! Clone Selection, previewed live in the viewports.

use super::widgets::{dim, max_spinner, toggle_button};
use crate::editor::Editor;
use crate::editor::mirror::MirrorAxis;
use crate::scene::CloneKind;
use bevy_egui::egui::{self, RichText};

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    let Some(mut s) = ed.mirror_dialog.as_ref().map(|d| d.settings.clone()) else { return };
    let mut result = None;
    egui::Window::new("Mirror: World Coordinates")
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center() - egui::vec2(120.0, 160.0))
        .show(ctx, |ui| {
            ui.set_width(230.0);
            ui.label(RichText::new("Mirror Axis").strong());
            egui::Grid::new("mirror_axes").num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
                for row in MirrorAxis::ALL.chunks(3) {
                    for &a in row {
                        if toggle_button(ui, s.axis == a, a.label()).clicked() {
                            s.axis = a;
                        }
                    }
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset:");
                max_spinner(ui, &mut s.offset, 0.01, 0.0);
            });
            ui.separator();
            ui.label(RichText::new("Clone Selection").strong());
            ui.radio_value(&mut s.clone, None, "No Clone");
            for k in CloneKind::ALL {
                ui.radio_value(&mut s.clone, Some(k), k.label());
            }
            ui.label(dim("Mirrors about the toolbar's transform center."));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("  OK  ").clicked() {
                    result = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    result = Some(false);
                }
            });
        });
    if let Some(d) = &mut ed.mirror_dialog {
        d.settings = s;
    }
    match result {
        Some(true) => ed.confirm_mirror(),
        Some(false) => ed.cancel_mirror(),
        None => ed.update_mirror_preview(),
    }
}
