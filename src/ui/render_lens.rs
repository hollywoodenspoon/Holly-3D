//! Render Setup (F10) "Lens Effects" rollout: aperture blades for polygonal
//! bokeh, cat's-eye vignetting, anamorphic streak flares and starburst
//! glare (lens model after EffectCraft, MIT / Apache-2.0).

use crate::editor::Editor;
use crate::render::lens_fx::LensFx;
use crate::ui::widgets::{dim, max_spinner_range, rollout};
use bevy_egui::egui;

pub fn rollout_ui(ui: &mut egui::Ui, ed: &mut Editor) {
    let l = &mut ed.render.post.lens;
    rollout(ui, "Lens Effects", false, |ui| {
        egui::Grid::new("fx_lens").num_columns(2).spacing([10.0, 6.0]).min_col_width(130.0).show(ui, |ui| {
            ui.label("Aperture Blades");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut l.blades, 0, "Circular");
                for n in 5..=9 {
                    ui.selectable_value(&mut l.blades, n, n.to_string());
                }
            });
            ui.end_row();
            ui.label("Blade Rotation");
            max_spinner_range(ui, &mut l.blade_rotation, 0.5, 0.0, -180.0..=180.0);
            ui.end_row();
            ui.label("Cat's-Eye Vignetting");
            max_spinner_range(ui, &mut l.cats_eye, 0.01, 0.0, 0.0..=1.0).on_hover_text("Off-centre bokeh becomes lemon-shaped");
            ui.end_row();
        });
        ui.label(dim("Blades and cat's-eye shape the Depth of Field bokeh."));
        ui.separator();
        ui.checkbox(&mut l.anamorphic, "Anamorphic Streak Flare");
        ui.add_enabled_ui(l.anamorphic, |ui| {
            ui.horizontal(|ui| {
                ui.label("Strength");
                max_spinner_range(ui, &mut l.anamorphic_strength, 0.01, 0.5, 0.0..=5.0);
                ui.color_edit_button_rgb(&mut l.anamorphic_tint).on_hover_text("Streak tint");
            });
        });
        ui.horizontal(|ui| {
            ui.label("Starburst");
            for n in LensFx::STAR_POINTS {
                ui.selectable_value(&mut l.star_points, n, if n == 0 { "Off".to_string() } else { format!("{n} pt") });
            }
        });
        ui.add_enabled_ui(l.star_points > 0, |ui| {
            ui.horizontal(|ui| {
                ui.label("Strength");
                max_spinner_range(ui, &mut l.star_strength, 0.01, 0.5, 0.0..=5.0);
            });
        });
        ui.horizontal(|ui| {
            ui.label("Flare Threshold");
            max_spinner_range(ui, &mut l.threshold, 0.01, 1.5, 0.0..=100.0).on_hover_text("Linear luminance above which highlights streak");
        });
    });
}
