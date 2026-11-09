//! Environment & Effects (`8`): 1-click lighting presets, background,
//! ambient tint and exponential fog.

use crate::editor::Editor;
use crate::render::{Environment, LightingPreset};
use crate::ui::widgets::{Track, color_row, dim, num_row};
use bevy_egui::egui::{self, RichText};

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(120.0).show(ui, body);
}

/// Edits a clone of the environment; one undo step per gesture.
fn edit(ed: &mut Editor, body: impl FnOnce(&mut Environment, &mut Track)) {
    let mut env = ed.scene.environment.clone();
    let mut t = Track::default();
    body(&mut env, &mut t);
    if t.started {
        ed.checkpoint("Environment");
    }
    if t.changed {
        ed.scene.environment = env;
    }
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.env_open {
        return;
    }
    let mut open = true;
    let mut preset = None;
    egui::Window::new(RichText::new("Environment and Effects").strong())
        .id(egui::Id::new("env_dialog"))
        .open(&mut open)
        .default_pos([420.0, 120.0])
        .default_width(330.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(RichText::new("Lighting Preset").strong());
            let cur = ed.scene.environment.preset;
            egui::ComboBox::from_id_salt("env_preset").width(300.0).selected_text(cur.label()).show_ui(ui, |ui| {
                for p in LightingPreset::ALL {
                    if ui.selectable_label(p == cur, p.label()).clicked() {
                        preset = Some(p);
                    }
                }
            });
            ui.label(dim("Presets place editable lights around the scene (and replace the previous preset's lights)."));
            ui.separator();
            edit(ed, |env, t| {
                ui.label(RichText::new("Common Parameters").strong());
                grid(ui, "env_common", |ui| {
                    color_row(ui, t, "Background Color", &mut env.background);
                    color_row(ui, t, "Ambient Tint", &mut env.ambient);
                    num_row(ui, t, "Ambient Level", &mut env.ambient_level, 0.0..=4.0, 1.0);
                });
                t.r(ui.checkbox(&mut env.auto_default_lights, "Default Lights when the scene has none"));
                ui.separator();
                ui.label(RichText::new("Atmosphere").strong());
                t.r(ui.checkbox(&mut env.fog, "Exponential Fog"));
                ui.add_enabled_ui(env.fog, |ui| {
                    grid(ui, "env_fog", |ui| {
                        color_row(ui, t, "Fog Color", &mut env.fog_color);
                        num_row(ui, t, "Density", &mut env.fog_density, 0.0..=2.0, 0.05);
                    });
                });
            });
            ui.separator();
            let lights = if ed.uses_default_lights() { "Default Lights (headlight rig)" } else { "Scene Lights" };
            ui.label(dim(&format!("Viewport lighting: {lights} · Ctrl+L toggles Default Lights")));
        });
    if let Some(p) = preset {
        ed.apply_lighting_preset(p);
    }
    if !open {
        ed.env_open = false;
    }
}
