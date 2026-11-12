//! PR 8 Smoke & Fire Plume rollout (the base object; the Battlefield
//! `[✓] Spawn Rising Smoke Plumes` section reuses it for its plumes).

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::smoke::{SmokeParams, SmokeStyle};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui;

pub fn params(ui: &mut egui::Ui, t: &mut Track, p: &mut SmokeParams, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("Smoke:");
        egui::ComboBox::from_id_salt(("smoke_style", salt)).width(CONTENT_W - 60.0).selected_text(p.style.label()).show_ui(ui, |ui| {
            for s in SmokeStyle::ALL {
                t.r(ui.selectable_value(&mut p.style, s, s.label()));
            }
        });
    });
    seed_row(ui, t, &mut p.seed);
    grid(ui, &format!("smoke_params_{salt}"), |ui| {
        row(ui, t, "Plume Height:", &mut p.height, 0.5..=2000.0, 30.0, 0.1);
        row(ui, t, "Base Radius:", &mut p.base_radius, 0.05..=100.0, 2.0, 0.01);
        row(ui, t, "Top Spread:", &mut p.top_spread, 1.0..=12.0, 4.0, 0.01);
        row(ui, t, "Billow Turbulence:", &mut p.turbulence, 0.0..=1.0, 0.5, 0.005);
        row(ui, t, "Wind Lean Angle:", &mut p.wind_lean, 0.0..=80.0, 18.0, 0.2);
        row(ui, t, "Wind Direction:", &mut p.wind_dir, -360.0..=360.0, 0.0, 0.5);
        row(ui, t, "Lobes:", &mut p.lobes, 3..=64, 16, 0.1);
    });
    ui.horizontal(|ui| {
        ui.label("Time / Rise Phase:");
        t.r(ui.add(egui::Slider::new(&mut p.phase, 0.0..=1.0)));
    });
    ui.horizontal(|ui| {
        ui.label("Fire Base Intensity:");
        t.r(ui.add(egui::Slider::new(&mut p.fire, 0.0..=100.0).suffix(" %")));
    });
    ui.horizontal(|ui| {
        t.r(ui.checkbox(&mut p.cards, "Turbulence Cards"));
        t.r(ui.checkbox(&mut p.mushroom, "Mushroom Cap"));
    });
    ui.label(dim("Puffs rise, slow and widen like a buoyant plume, bend downwind and sway as vortices shed. Density is in the vertex alpha (Smoke & Fire Pack: 1 fire, 2 smoke, 3 wisps)."));
}
