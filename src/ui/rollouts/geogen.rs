//! GeoGen rollouts: the shared Parameters rollout (base object and modifier)
//! plus the modifier's Grow / Deform options.

use super::spline::{grid, mod_params, num_row, u_row};
use crate::editor::Editor;
use crate::modifiers::{GeoGenParams, GeoMode, ModifierKind};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

/// Generator Mode + Parameters, shared by the base object and the modifier.
fn params(ui: &mut egui::Ui, t: &mut Track, p: &mut GeoGenParams) {
    ui.label(RichText::new("Generator Mode").strong());
    egui::ComboBox::from_id_salt("geogen_mode").width(CONTENT_W - 12.0).selected_text(p.mode.label()).show_ui(ui, |ui| {
        for m in GeoMode::ALL {
            if ui.selectable_label(p.mode == m, m.label()).clicked() && p.mode != m {
                // A new mode starts from its own defaults, keeping the seed.
                *p = GeoGenParams { seed: p.seed, ..GeoGenParams::default_of(m) };
                t.started = true;
                t.changed = true;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Random Seed:");
        t.r(max_spinner_range(ui, &mut p.seed, 0.2, 1, 0..=99_999));
        if ui.button("🎲 New Seed").clicked() {
            p.seed = rand::random::<u32>() % 100_000;
            t.started = true;
            t.changed = true;
        }
    });
    let count = match p.mode {
        GeoMode::Crystal => "Cluster Spires:",
        GeoMode::Rock => "Boulders:",
        GeoMode::Cliff => "Strata:",
        GeoMode::Stalagmite => "Columns:",
    };
    let facets = match p.mode {
        GeoMode::Cliff => "Fracture Cracks:",
        GeoMode::Stalagmite => "Flutes:",
        _ => "Planar Cuts:",
    };
    grid(ui, "geogen_params", |ui| {
        u_row(ui, t, count, &mut p.count, 7, 1..=50);
        if matches!(p.mode, GeoMode::Crystal | GeoMode::Stalagmite) {
            num_row(ui, t, "Spread Angle:", &mut p.spread, 0.0..=90.0, 35.0);
        }
        num_row(ui, t, "Min Height:", &mut p.min_height, 0.01..=100.0, 0.4);
        num_row(ui, t, "Max Height:", &mut p.max_height, 0.01..=100.0, 1.2);
        num_row(ui, t, "Base Radius:", &mut p.base_radius, 0.01..=100.0, 0.5);
        if matches!(p.mode, GeoMode::Crystal | GeoMode::Stalagmite) {
            num_row(ui, t, "Tip Taper:", &mut p.tip_taper, 0.0..=1.0, 0.3);
        }
        u_row(ui, t, facets, &mut p.facets, 8, 0..=30);
        num_row(ui, t, "Noise Strength:", &mut p.noise_strength, 0.0..=2.0, 0.25);
        num_row(ui, t, "Noise Scale:", &mut p.noise_scale, 0.05..=20.0, 1.5);
    });
    if p.min_height > p.max_height {
        p.max_height = p.min_height;
    }
    t.r(ui.checkbox(&mut p.auto_smooth, "Auto-Assign Smoothing Groups"));
    match p.mode {
        GeoMode::Crystal => {
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut p.side_shards, "Side Shards"));
                t.r(ui.checkbox(&mut p.cluster_base, "Cluster Base"));
            });
        }
        GeoMode::Stalagmite => {
            t.r(ui.checkbox(&mut p.hanging, "Hang from Ceiling (Stalactite)"));
        }
        _ => {}
    }
}

/// GeoGen base object (Create -> Geometry -> GeoGen).
pub fn base(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(id) = ed.current() else { return };
    let Some(mut p) = ed.scene.objects[id].geogen_mut().cloned() else { return };
    let mut t = Track::default();
    rollout(ui, "GeoGen Parameters", true, |ui| params(ui, &mut t, &mut p));
    if t.started {
        ed.checkpoint("GeoGen Parameters");
    }
    if t.changed {
        let o = &mut ed.scene.objects[id];
        if let Some(g) = o.geogen_mut() {
            *g = p;
        }
        o.evaluate();
        ed.scene.propagate_from(id);
    }
}

/// GeoGen stack modifier: grow formations on polygons, or deform the mesh.
pub fn modifier(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "GeoGen Modifier", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::GeoGen(g) = k else { return };
            ui.horizontal(|ui| {
                t.r(ui.radio_value(&mut g.deform, false, "Grow on Polygons"));
                t.r(ui.radio_value(&mut g.deform, true, "Deform Mesh"));
            });
            if g.deform {
                ui.label(dim("Displaces the input with rock noise and planar cuts (Noise / Planar Cuts below)."));
            } else {
                grid(ui, "geogen_grow", |ui| {
                    num_row(ui, t, "Density:", &mut g.density, 0.0..=1.0, 1.0);
                    num_row(ui, t, "Scale:", &mut g.scale, 0.01..=10.0, 1.0);
                });
                ui.label(dim("Grows on the selected polygons passed up the stack, or on every polygon."));
            }
            ui.separator();
            params(ui, t, &mut g.params);
        });
    });
}
