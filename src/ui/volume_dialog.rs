//! Environment (`8`) -> Volumetric Godrays & Mist rollout.

use crate::render::environment::Environment;
use crate::render::volumetrics::VolumeSettings;
use crate::ui::widgets::{Track, color_row, dim, max_spinner_range, num_row};
use bevy_egui::egui::{self, RichText};

pub fn rows(ui: &mut egui::Ui, env: &mut Environment, t: &mut Track) {
    ui.separator();
    ui.label(RichText::new("Volumetric Godrays & Mist").strong());
    let v = &mut env.volume;
    t.r(ui.checkbox(&mut v.enabled, "Enable Volumetric Godrays"));
    ui.add_enabled_ui(v.enabled, |ui| {
        egui::Grid::new("env_volume").num_columns(2).spacing([10.0, 6.0]).min_col_width(120.0).show(ui, |ui| {
            num_row(ui, t, "Godray Intensity", &mut v.intensity, 0.0..=10.0, 1.0);
            num_row(ui, t, "Anisotropy g", &mut v.anisotropy, -0.9..=0.95, 0.6);
            ui.label("Volumetric Steps");
            t.r(max_spinner_range(ui, &mut v.steps, 1.0, 48, 16..=128));
            ui.end_row();
            color_row(ui, t, "Fog Scatter Color", &mut v.color);
            num_row(ui, t, "Density", &mut v.density, 0.0..=2.0, 0.04);
            num_row(ui, t, "Mist Base Height", &mut v.base_height, -100.0..=100.0, 0.0);
            num_row(ui, t, "Height Falloff", &mut v.falloff, 0.0..=5.0, 0.0);
        });
        ui.label(RichText::new("3D Noise Wisps").strong());
        egui::Grid::new("env_wisps").num_columns(2).spacing([10.0, 6.0]).min_col_width(120.0).show(ui, |ui| {
            num_row(ui, t, "Wisp Density", &mut v.wisps, 0.0..=1.0, 0.0);
            num_row(ui, t, "Wisp Scale", &mut v.wisp_scale, 0.1..=50.0, 4.0);
            num_row(ui, t, "Wind Drift X", &mut v.wind[0], -100.0..=100.0, 0.0);
            num_row(ui, t, "Wind Drift Y", &mut v.wind[1], -100.0..=100.0, 0.0);
        });
        ui.horizontal(|ui| {
            for (label, g, falloff, wisps) in [("Cathedral Shafts", 0.8, 0.0, 0.0), ("Valley Mist", 0.3, 0.6, 0.6), ("Dusty Hangar", 0.55, 0.1, 0.3)] {
                if ui.button(label).on_hover_text("Godray preset").clicked() {
                    (t.started, t.changed) = (true, true);
                    *v = VolumeSettings { enabled: true, anisotropy: g, falloff, wisps, ..VolumeSettings::default() };
                }
            }
        });
        ui.label(dim("Light shafts come from scene lights (link the Sun to the sky for sun shafts). Perspective viewports preview them live."));
    });
}
