//! PR 8: Environment (`8`) -> Dynamic Sky -> "Weather & Storm Clouds"
//! rollout: one-click Weather Preset, Cloud Thickness (optical depth),
//! Overcast Sun Dimming, Lightning Flash and the Precipitation controls.

use crate::render::Environment;
use crate::render::weather::{Precip, WeatherPreset};
use crate::ui::widgets::{Track, dim, num_row, rollout};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(120.0).show(ui, body);
}

/// Applies a preset from the dropdown (reported through `t`).
pub fn apply(env: &mut Environment, p: WeatherPreset, t: &mut Track) {
    p.apply(env);
    (t.started, t.changed) = (true, true);
}

pub fn rows(ui: &mut egui::Ui, env: &mut Environment, t: &mut Track) {
    rollout(ui, "Weather & Storm Clouds", true, |ui| {
        let cur = env.sky.weather.preset;
        ui.horizontal(|ui| {
            ui.label("Weather Preset");
            egui::ComboBox::from_id_salt("weather_preset").width(200.0).selected_text(cur.label()).show_ui(ui, |ui| {
                for p in WeatherPreset::ALL {
                    if ui.selectable_label(p == cur, p.label()).clicked() {
                        apply(env, p, t);
                    }
                }
            });
        });
        let w = &mut env.sky.weather;
        grid(ui, "weather_clouds", |ui| {
            let mut pct = w.thickness * 100.0;
            num_row(ui, t, "Cloud Thickness %", &mut pct, 0.0..=100.0, 20.0);
            w.thickness = pct / 100.0;
        });
        t.r(ui.checkbox(&mut w.sun_dimming, "Overcast Sun Dimming (dims + softens the linked sun)"));
        t.r(ui.checkbox(&mut w.lightning, "Lightning Flash"));
        ui.horizontal(|ui| {
            ui.label("Precipitation");
            egui::ComboBox::from_id_salt("weather_precip").width(180.0).selected_text(w.precip.label()).show_ui(ui, |ui| {
                for p in Precip::ALL {
                    t.r(ui.selectable_value(&mut w.precip, p, p.label()));
                }
            });
        });
        ui.add_enabled_ui(w.precip != Precip::None, |ui| {
            grid(ui, "weather_precip_g", |ui| {
                num_row(ui, t, "Amount", &mut w.amount, 0.0..=1.0, 0.6);
                num_row(ui, t, "Wind Slant (deg)", &mut w.slant, -60.0..=60.0, 12.0);
            });
        });
        ui.horizontal(|ui| {
            ui.label("Seed (bolt / particles)");
            t.r(ui.add(egui::DragValue::new(&mut w.seed).range(0..=9999)));
            if ui.button("🎲 New Seed").clicked() {
                w.seed = w.seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453) % 10_000;
                (t.started, t.changed) = (true, true);
            }
        });
        ui.label(dim("Thick clouds darken their bases (Beer-Lambert). Precipitation shows in the viewport and in F9, hidden behind nearer geometry."));
    });
}
