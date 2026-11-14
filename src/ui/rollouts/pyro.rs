//! Clouds PR 4 rollout: one explosion (Pyro) object.

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::pyro::{PyroKind, PyroParams, SparkColor};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui;

/// The Pyro rollout.
pub fn pyro(ui: &mut egui::Ui, t: &mut Track, p: &mut PyroParams) {
    ui.horizontal(|ui| {
        ui.label("Type:");
        let mut k = p.kind;
        egui::ComboBox::from_id_salt("pyro_type").width(content_w() - 60.0).selected_text(k.label()).show_ui(ui, |ui| {
            for c in PyroKind::ALL {
                t.r(ui.selectable_value(&mut k, c, c.label()).on_hover_text(c.tip()));
            }
        });
        if k != p.kind {
            *p = PyroParams { seed: p.seed, stage: p.stage, ..PyroParams::of(k) };
        }
    });
    seed_row(ui, t, &mut p.seed);
    let d = PyroParams::of(p.kind);
    let secs = p.stage * p.kind.duration();
    ui.horizontal(|ui| {
        ui.label("Stage:");
        let mut pct = p.stage * 100.0;
        let r = ui
            .add_sized([content_w() - 60.0, 18.0], egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %"))
            .on_hover_text("The moment of the blast: 0 % is the first flash, 100 % the last of the smoke");
        if r.secondary_clicked() {
            pct = d.stage * 100.0;
            (t.started, t.changed) = (true, true);
        }
        t.r(r);
        p.stage = pct / 100.0;
    });
    ui.label(dim(&format!("{secs:.1} s after the blast")));
    grid(ui, "pyro_params", |ui| {
        row(ui, t, "Size:", &mut p.size, 0.1..=5000.0, d.size, 0.05);
        row(ui, t, "Billow:", &mut p.billow, 0.0..=1.0, d.billow, 0.005);
        row(ui, t, "Heat:", &mut p.heat, 0.0..=2.0, d.heat, 0.005);
        row(ui, t, "Smoke Darkness:", &mut p.darkness, 0.0..=1.0, d.darkness, 0.005);
    });
    ui.label("Sparks:");
    grid(ui, "pyro_sparks", |ui| {
        row(ui, t, "Count:", &mut p.sparks, 0..=1500, d.sparks, 1.0);
        row(ui, t, "Speed (m/s):", &mut p.spark_speed, 0.0..=500.0, d.spark_speed, 0.2);
        row(ui, t, "Speed Spread:", &mut p.speed_spread, 0.0..=1.0, d.speed_spread, 0.005);
        row(ui, t, "Streak Length:", &mut p.streak, 0.0..=5.0, d.streak, 0.01);
        row(ui, t, "Cooling:", &mut p.spark_cooling, 0.0..=1.0, d.spark_cooling, 0.005);
        row(ui, t, "Burning Debris:", &mut p.debris, 0..=64, d.debris, 0.1);
    });
    ui.horizontal(|ui| {
        ui.label("Spark Colour:");
        for c in SparkColor::ALL {
            t.r(ui.selectable_value(&mut p.spark_color, c, c.label()));
        }
    });
    t.r(ui.checkbox(&mut p.light, "Explosion Light").on_hover_text("A point light at the fire's centre, sized and coloured from the fire (in its own LightMix group, Explosions)"));
    if p.light {
        grid(ui, "pyro_light", |ui| row(ui, t, "Light Strength:", &mut p.light_strength, 0.0..=20.0, d.light_strength, 0.01));
    }
    super::clouds::look(ui, t, &mut p.look, "pyro_look");
    let n = p.puffs().len();
    ui.label(dim(&format!(
        "{n} puffs. Fire, smoke, dust, vapour and sparks render as separate volumes; the AgX tonemap (Render Setup) keeps bright fire from clipping to flat yellow."
    )));
}
