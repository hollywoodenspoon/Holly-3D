//! Environment (`8`) "Dynamic Sky" section: Sky Mode, Time of Day slider
//! (00:00..24:00), sun path / link options, the Procedural Clouds rollout
//! and the Deep Space rollout (starfield, nebula bands, single star).

use crate::render::Environment;
use crate::render::sky::SkyMode;
use crate::ui::widgets::{Track, color_row, dim, num_row, rollout};
use bevy_egui::egui::{self, RichText};

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(120.0).show(ui, body);
}

/// `14.5` -> `"14:30"`.
pub fn hhmm(hours: f32) -> String {
    let m = (hours.clamp(0.0, 24.0) * 60.0).round() as u32;
    format!("{:02}:{:02}", m / 60, m % 60)
}

fn earth(ui: &mut egui::Ui, env: &mut Environment, t: &mut Track) {
    let s = &mut env.sky;
    ui.horizontal(|ui| {
        ui.label("Time of Day");
        let slider = egui::Slider::new(&mut s.time_of_day, 0.0..=24.0).step_by(0.25).custom_formatter(|v, _| hhmm(v as f32));
        t.r(ui.add(slider));
    });
    grid(ui, "sky_earth", |ui| {
        num_row(ui, t, "Sun Path Azimuth", &mut s.sun_azimuth, -180.0..=180.0, 0.0);
        num_row(ui, t, "Sky Intensity", &mut s.intensity, 0.0..=10.0, 1.0);
    });
    let c = &mut s.clouds;
    rollout(ui, "Procedural Clouds", true, |ui| {
        t.r(ui.checkbox(&mut c.enabled, "Enable Clouds"));
        ui.add_enabled_ui(c.enabled, |ui| {
            grid(ui, "sky_clouds", |ui| {
                num_row(ui, t, "Cloud Coverage", &mut c.coverage, 0.0..=1.0, 0.4);
                num_row(ui, t, "Density", &mut c.density, 0.0..=1.0, 0.8);
                num_row(ui, t, "Altitude (km)", &mut c.altitude, 0.2..=12.0, 2.0);
                num_row(ui, t, "Wind Drift Speed", &mut c.wind_speed, 0.0..=10.0, 0.5);
            });
        });
        ui.label(dim("Clouds pick up the sunset glow and moonlight automatically."));
    });
    crate::ui::weather_rollout::rows(ui, env, t);
}

fn space(ui: &mut egui::Ui, env: &mut Environment, t: &mut Track) {
    let s = &mut env.sky;
    grid(ui, "sky_space_i", |ui| num_row(ui, t, "Sky Intensity", &mut s.intensity, 0.0..=10.0, 1.0));
    let sp = &mut s.space;
    rollout(ui, "Starfield & Star", true, |ui| {
        grid(ui, "sky_stars", |ui| {
            num_row(ui, t, "Star Density", &mut sp.star_density, 0.0..=0.1, 0.02);
            num_row(ui, t, "Star Brightness", &mut sp.star_brightness, 0.0..=10.0, 1.0);
            num_row(ui, t, "Sun Azimuth", &mut sp.star_azimuth, -180.0..=180.0, 40.0);
            num_row(ui, t, "Sun Elevation", &mut sp.star_elevation, -90.0..=90.0, 25.0);
            color_row(ui, t, "Sun Color", &mut sp.star_color);
        });
    });
    rollout(ui, "Nebula Clouds", true, |ui| {
        t.r(ui.checkbox(&mut sp.nebula, "Enable Nebula"));
        ui.add_enabled_ui(sp.nebula, |ui| {
            grid(ui, "sky_nebula", |ui| {
                color_row(ui, t, "Band Color A", &mut sp.nebula_a);
                color_row(ui, t, "Band Color B", &mut sp.nebula_b);
                num_row(ui, t, "Nebula Density", &mut sp.nebula_density, 0.0..=2.0, 0.5);
            });
        });
    });
}

/// The whole section (edits `env`; changes are reported through `t`).
pub fn rows(ui: &mut egui::Ui, env: &mut Environment, t: &mut Track) {
    ui.separator();
    ui.label(RichText::new("Dynamic Sky").strong());
    let cur = env.sky.mode;
    egui::ComboBox::from_id_salt("sky_mode").width(300.0).selected_text(cur.label()).show_ui(ui, |ui| {
        for m in SkyMode::ALL {
            if ui.selectable_label(m == cur, m.label()).clicked() {
                env.sky.mode = m;
                (t.started, t.changed) = (true, true);
            }
        }
    });
    if env.sky.mode == SkyMode::Off {
        return;
    }
    let s = &mut env.sky;
    t.r(ui.checkbox(&mut s.link_sun, "Link Main Directional Sun to Sky Time"));
    t.r(ui.checkbox(&mut s.lighting, "Use for Lighting (sky ambient)"));
    t.r(ui.checkbox(&mut s.show_in_viewport, "Show in Perspective Viewports"));
    match env.sky.mode {
        SkyMode::Earth => earth(ui, env, t),
        SkyMode::Space => space(ui, env, t),
        SkyMode::Off => {}
    }
    ui.label(dim("The sky replaces the HDRI while on (F9 background, reflections and viewport)."));
}

#[cfg(test)]
mod tests {
    use super::hhmm;

    #[test]
    fn time_of_day_formats_as_clock() {
        assert_eq!(hhmm(0.0), "00:00");
        assert_eq!(hhmm(14.5), "14:30");
        assert_eq!(hhmm(18.25), "18:15");
        assert_eq!(hhmm(24.0), "24:00");
    }
}
