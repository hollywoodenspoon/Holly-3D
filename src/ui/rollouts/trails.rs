//! Clouds PR 3 rollout: one Trail object.

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::trails::{TrailKind, TrailParams};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui;

/// The Trail rollout; returns true when `[ Pick Path Spline ]` is clicked.
pub fn trail(ui: &mut egui::Ui, t: &mut Track, p: &mut TrailParams) -> bool {
    ui.horizontal(|ui| {
        ui.label("Type:");
        let mut k = p.kind;
        egui::ComboBox::from_id_salt("trail_type").width(content_w() - 60.0).selected_text(k.label()).show_ui(ui, |ui| {
            for c in TrailKind::ALL {
                t.r(ui.selectable_value(&mut k, c, c.label()).on_hover_text(c.tip()));
            }
        });
        if k != p.kind {
            *p = TrailParams { seed: p.seed, path: std::mem::take(&mut p.path), ..TrailParams::of(k) };
        }
    });
    seed_row(ui, t, &mut p.seed);
    let d = TrailParams::of(p.kind);
    ui.label("Path:");
    if p.path.len() >= 2 {
        ui.horizontal(|ui| {
            ui.label(dim(&format!("Picked spline, {} points", p.path.len())));
            if ui.button("Reverse").on_hover_text("Put the source at the other end").clicked() {
                p.path.reverse();
                (t.started, t.changed) = (true, true);
            }
            if ui.button("Clear").on_hover_text("Go back to the built-in arc").clicked() {
                p.path.clear();
                (t.started, t.changed) = (true, true);
            }
        });
    } else {
        grid(ui, "trail_arc", |ui| {
            row(ui, t, "Length:", &mut p.length, 0.1..=100_000.0, d.length, 0.5);
            row(ui, t, "Turn:", &mut p.turn, -360.0..=360.0, 0.0, 0.5);
            row(ui, t, "Climb:", &mut p.climb, -20_000.0..=20_000.0, 0.0, 0.5);
        });
    }
    let pick = ui
            .add_sized([content_w(), 24.0], egui::Button::new("Pick Path Spline"))
            .on_hover_text("Click a spline in a viewport: the trail follows it from its first point")
            .clicked();
    let flame_only = p.kind == TrailKind::Afterburner;
    grid(ui, "trail_params", |ui| {
        if p.kind != TrailKind::VaporCone {
            let label = if p.kind == TrailKind::WingtipVortices { "Wingspan:" } else { "Source Spacing:" };
            if p.kind != TrailKind::WingtipVortices {
                row(ui, t, "Sources:", &mut p.sources, 1..=8, d.sources, 0.05);
            }
            row(ui, t, label, &mut p.spacing, 0.0..=500.0, d.spacing, 0.05);
        }
        row(ui, t, "Start Gap:", &mut p.start_gap, 0.0..=10_000.0, d.start_gap, 0.1);
        row(ui, t, "Start Radius:", &mut p.start_radius, 0.01..=1000.0, d.start_radius, 0.01);
        row(ui, t, "End Radius:", &mut p.end_radius, 0.01..=1000.0, d.end_radius, 0.01);
        if !flame_only {
            row(ui, t, "Spread:", &mut p.spread, 0.0..=1.0, d.spread, 0.005);
        }
        row(ui, t, "Breakup:", &mut p.breakup, 0.0..=1.0, d.breakup, 0.005);
        row(ui, t, "Fade:", &mut p.fade, 0.0..=1.0, d.fade, 0.005);
        row(ui, t, "Dissipation Length:", &mut p.dissipation, 0.01..=100_000.0, d.dissipation, 0.5);
        match p.kind {
            TrailKind::Afterburner => {
                row(ui, t, "Throttle:", &mut p.throttle, 0.0..=1.0, d.throttle, 0.005);
                row(ui, t, "Shock Diamonds:", &mut p.diamonds, 0..=12, d.diamonds, 0.05);
            }
            TrailKind::MissileTrail => row(ui, t, "Throttle:", &mut p.throttle, 0.0..=1.0, d.throttle, 0.005),
            TrailKind::WingtipVortices => row(ui, t, "Spiral Turns / 100 m:", &mut p.twist, 0.0..=50.0, d.twist, 0.05),
            _ => {}
        }
    });
    if p.kind == TrailKind::DamageSmoke {
        t.r(ui.checkbox(&mut p.fire, "Fire at the Start"));
        t.r(ui.checkbox(&mut p.black, "Black Smoke (off: grey)"));
    }
    super::clouds::look(ui, t, &mut p.look, "trail_look");
    let n = p.puffs().len();
    ui.label(dim(&format!("{n} puffs. The source sits at the object's pivot; the trail runs back along local -X (or along the picked path).")));
    pick
}
