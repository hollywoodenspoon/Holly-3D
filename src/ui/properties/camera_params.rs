//! Modify panel for cameras: Lens / FOV, stock lenses, type, clipping
//! planes and display options (Parameters rollout).

use super::content_w;
use crate::editor::Editor;
use crate::scene::camera::{Camera, CameraKind, STOCK_LENSES};
use crate::ui::widgets::{Track, dim, num_row, rollout};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, body);
}

/// Edits a clone of the current camera; one undo step per gesture.
fn edit(ed: &mut Editor, body: impl FnOnce(&mut Camera, &mut Track)) {
    let Some(mut c) = ed.current_obj().and_then(|o| o.camera()).cloned() else { return };
    let mut t = Track::default();
    body(&mut c, &mut t);
    if t.started {
        ed.checkpoint("Camera Parameters");
    }
    if t.changed {
        ed.edit_camera(None, |dst| *dst = c);
    }
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(kind) = ed.current_obj().and_then(|o| o.camera()).map(|c| c.kind) else { return };
    let mut pick = None;
    rollout(ui, "Parameters", true, |ui| {
        edit(ed, |c, t| {
            grid(ui, "cam_lens", |ui| {
                let mut lens = c.lens_mm();
                num_row(ui, t, "Lens (mm)", &mut lens, 1.0..=1000.0, 43.456);
                if (lens - c.lens_mm()).abs() > 1e-4 {
                    c.set_lens_mm(lens);
                }
                num_row(ui, t, "FOV (°)", &mut c.fov, 0.5..=175.0, 45.0);
            });
            ui.label(dim("Stock Lenses"));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for mm in STOCK_LENSES {
                    let on = (c.lens_mm() - mm).abs() < 0.05;
                    if ui.selectable_label(on, format!("{mm}")).clicked() {
                        c.set_lens_mm(mm);
                        (t.started, t.changed) = (true, true);
                    }
                }
            });
            t.r(ui.checkbox(&mut c.show_cone, "Show Cone"));
        });
        ui.horizontal(|ui| {
            ui.label("Type");
            egui::ComboBox::from_id_salt("cam_type").width(content_w() - 50.0).selected_text(kind.label()).show_ui(ui, |ui| {
                for k in CameraKind::ALL {
                    if ui.selectable_label(k == kind, k.label()).clicked() {
                        pick = Some(k);
                    }
                }
            });
        });
        edit(ed, |c, t| {
            if c.kind == CameraKind::Free {
                grid(ui, "cam_free", |ui| num_row(ui, t, "Target Distance", &mut c.target_distance, 0.01..=10_000.0, 10.0));
            }
        });
    });
    rollout(ui, "Clipping Planes", true, |ui| {
        edit(ed, |c, t| {
            t.r(ui.checkbox(&mut c.clip_manually, "Clip Manually"));
            ui.add_enabled_ui(c.clip_manually, |ui| {
                grid(ui, "cam_clip", |ui| {
                    num_row(ui, t, "Near Clip", &mut c.near, 0.001..=10_000.0, 1.0);
                    num_row(ui, t, "Far Clip", &mut c.far, 0.01..=100_000.0, 1000.0);
                });
            });
        });
    });
    if let Some(k) = pick {
        ed.set_camera_kind(k);
    }
    let vi = ed.viewports.active;
    let id = ed.current();
    if ui.add_sized([content_w(), 26.0], egui::Button::new("Look Through Camera   C")).clicked() {
        ed.look_through(vi, id);
    }
}
