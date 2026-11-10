//! Planet Studio rollouts: Planet (radius, subdivisions, seed), then one
//! rollout per concentric layer — Planet Surface, Weather & Cloud Shell,
//! Atmosphere Rim Halo and Planetary Rings — each with its own toggle.

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::{PlanetParams, PlanetPart, SurfacePreset};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, rollout};
use bevy_egui::egui::{self, RichText};
use std::path::PathBuf;

/// A custom-texture button press: the layer and the picked image (None =
/// back to procedural).
pub type TexRequest = (PlanetPart, Option<PathBuf>);

const IMAGE_EXTS: [&str; 6] = ["png", "jpg", "jpeg", "exr", "hdr", "tga"];

fn pick_image(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).add_filter("Image", &IMAGE_EXTS).pick_file()
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or(path.to_string(), |n| n.to_string_lossy().to_string())
}

/// Load / clear buttons for one layer's custom texture.
fn texture_row(ui: &mut egui::Ui, part: PlanetPart, current: &Option<String>, label: &str, req: &mut Option<TexRequest>) {
    ui.horizontal(|ui| {
        if ui.button(label).on_hover_text("Lat-long (equirectangular) image, 2:1").clicked()
            && let Some(p) = pick_image(label)
        {
            *req = Some((part, Some(p)));
        }
        if current.is_some() && ui.button("✖ Procedural").clicked() {
            *req = Some((part, None));
        }
    });
    if let Some(c) = current {
        ui.label(dim(&format!("Texture: {}", file_name(c))));
    }
}

fn surface(ui: &mut egui::Ui, t: &mut Track, p: &mut PlanetParams, req: &mut Option<TexRequest>) {
    t.r(ui.checkbox(&mut p.show_surface, "Show Planet Surface"));
    ui.label(RichText::new("Surface Preset").strong());
    egui::ComboBox::from_id_salt("planet_surface").width(CONTENT_W - 12.0).selected_text(p.surface.label()).show_ui(ui, |ui| {
        for s in SurfacePreset::ALL {
            if ui.selectable_label(p.surface == s, s.label()).clicked() && p.surface != s {
                *p = p.with_preset(s);
                (t.started, t.changed) = (true, true);
            }
        }
    });
    grid(ui, "planet_surface_grid", |ui| {
        row(ui, t, "Sea Level:", &mut p.sea_level, 0.0..=1.0, 0.52, 0.005);
        row(ui, t, "Mountain Relief:", &mut p.relief, 0.0..=0.5, 0.03, 0.001);
    });
    texture_row(ui, PlanetPart::Surface, &p.custom_surface, "Load Custom Surface Texture", req);
    ui.label(dim("IDs 1 Ocean / Lowland · 2 Land · 3 Highland · 4 Ice (Planet Studio Pack)."));
}

fn clouds(ui: &mut egui::Ui, t: &mut Track, p: &mut PlanetParams, req: &mut Option<TexRequest>) {
    let c = &mut p.clouds;
    t.r(ui.checkbox(&mut c.enabled, "Weather & Cloud Shell"));
    if !c.enabled {
        return;
    }
    grid(ui, "planet_clouds", |ui| {
        row(ui, t, "Cloud Coverage:", &mut c.coverage, 0.0..=1.0, 0.5, 0.005);
        row(ui, t, "Swirl Intensity:", &mut c.swirl, 0.0..=1.0, 0.6, 0.005);
        row(ui, t, "Storm Eye Count:", &mut c.storm_eyes, 0..=12, 3, 0.05);
        row(ui, t, "Opacity:", &mut c.opacity, 0.0..=1.0, 0.85, 0.005);
    });
    texture_row(ui, PlanetPart::Clouds, &c.custom_map, "Load Custom Cloud Map", req);
    ui.label(dim("Shell at radius × 1.015 (ID 5); its alpha casts soft shadows on the surface in F9."));
}

fn atmosphere(ui: &mut egui::Ui, t: &mut Track, p: &mut PlanetParams) {
    let a = &mut p.atmosphere;
    t.r(ui.checkbox(&mut a.enabled, "Atmosphere Rim Halo"));
    if !a.enabled {
        return;
    }
    ui.horizontal(|ui| {
        ui.label("Atmosphere Color:");
        t.r(ui.color_edit_button_rgb(&mut a.color));
    });
    grid(ui, "planet_atmo", |ui| {
        row(ui, t, "Density:", &mut a.density, 0.0..=2.0, 0.5, 0.005);
        row(ui, t, "Sunset Terminator Glow:", &mut a.sunset_glow, 0.0..=1.0, 0.6, 0.005);
        row(ui, t, "Sun Azimuth:", &mut p.sun_azimuth, -360.0..=360.0, 30.0, 0.5);
        row(ui, t, "Sun Elevation:", &mut p.sun_elevation, -90.0..=90.0, 10.0, 0.5);
    });
    ui.label(dim("Stacked see-through shells (ID 6) glow brightest at the limb; aim the sun to match your key light."));
}

fn rings(ui: &mut egui::Ui, t: &mut Track, p: &mut PlanetParams) {
    let r = &mut p.rings;
    t.r(ui.checkbox(&mut r.enabled, "Planetary Rings"));
    if !r.enabled {
        return;
    }
    grid(ui, "planet_rings", |ui| {
        row(ui, t, "Inner Radius (× R):", &mut r.inner, 1.0..=10.0, 1.35, 0.005);
        row(ui, t, "Outer Radius (× R):", &mut r.outer, 1.05..=12.0, 2.3, 0.005);
        row(ui, t, "Tilt Angle:", &mut r.tilt, -90.0..=90.0, 20.0, 0.5);
        row(ui, t, "Radial Band Noise:", &mut r.band_noise, 0.0..=1.0, 0.7, 0.005);
        row(ui, t, "Opacity:", &mut r.opacity, 0.0..=1.0, 0.8, 0.005);
    });
    if r.outer < r.inner + 0.05 {
        r.outer = r.inner + 0.05;
    }
}

/// All Planet Studio rollouts; returns a pending custom-texture request.
pub fn params(ui: &mut egui::Ui, t: &mut Track, p: &mut PlanetParams) -> Option<TexRequest> {
    let mut req = None;
    rollout(ui, "Planet", true, |ui| {
        grid(ui, "planet_main", |ui| {
            row(ui, t, "Radius:", &mut p.radius, 0.01..=1_000_000.0, 50.0, 0.2);
            row(ui, t, "Subdivisions:", &mut p.subdivisions, 4..=128, 32, 0.1);
        });
        seed_row(ui, t, &mut p.seed);
    });
    rollout(ui, "1. Planet Surface", true, |ui| surface(ui, t, p, &mut req));
    rollout(ui, "2. Weather & Cloud Shell", true, |ui| clouds(ui, t, p, &mut req));
    rollout(ui, "3. Atmosphere Rim Halo", false, |ui| atmosphere(ui, t, p));
    rollout(ui, "4. Planetary Rings", false, |ui| rings(ui, t, p));
    req
}
