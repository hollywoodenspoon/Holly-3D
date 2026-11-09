//! Modify panel for lights: General, Intensity / Color / Attenuation,
//! Spotlight / Directional, and Shadow Parameters rollouts.

use super::CONTENT_W;
use crate::editor::Editor;
use crate::render::lighting::{KELVIN_PRESETS, kelvin_to_rgb};
use crate::render::{Light, LightKind};
use crate::ui::widgets::{Track, color_row, dim, num_row, rollout};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, body);
}

/// Edits a clone of the current light; one undo step per gesture.
fn edit(ed: &mut Editor, body: impl FnOnce(&mut Light, &mut Track)) {
    let Some(mut l) = ed.current_obj().and_then(|o| o.light()).cloned() else { return };
    let mut t = Track::default();
    body(&mut l, &mut t);
    if t.started {
        ed.checkpoint("Light Parameters");
    }
    if t.changed {
        ed.edit_light(None, |dst| *dst = l);
    }
}

fn general(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(kind) = ed.current_obj().and_then(|o| o.light()).map(|l| l.kind) else { return };
    let mut pick = None;
    rollout(ui, "General Parameters", true, |ui| {
        edit(ed, |l, t| {
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut l.on, "Light On"));
                if !matches!(l.kind, LightKind::Skylight) {
                    t.r(ui.checkbox(&mut l.cast_shadows, "Cast Shadows"));
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label("Type");
            egui::ComboBox::from_id_salt("light_type").width(CONTENT_W - 50.0).selected_text(kind.label()).show_ui(ui, |ui| {
                for k in LightKind::ALL {
                    if ui.selectable_label(k == kind, k.label()).clicked() {
                        pick = Some(k);
                    }
                }
            });
        });
    });
    if let Some(k) = pick {
        ed.set_light_kind(k);
    }
}

fn intensity(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Intensity / Color / Attenuation", true, |ui| {
        edit(ed, |l, t| {
            grid(ui, "light_int", |ui| {
                num_row(ui, t, "Multiplier", &mut l.multiplier, 0.0..=20.0, 1.0);
                color_row(ui, t, "Color", &mut l.color);
                ui.label("Kelvin Preset");
                egui::ComboBox::from_id_salt("light_kelvin").width(130.0).selected_text("Choose…").show_ui(ui, |ui| {
                    for (name, k) in KELVIN_PRESETS {
                        if ui.selectable_label(false, name).clicked() {
                            l.color = kelvin_to_rgb(k);
                            (t.started, t.changed) = (true, true);
                        }
                    }
                });
                ui.end_row();
            });
            if !l.kind.has_range() {
                return;
            }
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Decay / Range").strong());
            t.r(ui.checkbox(&mut l.use_atten, "Use Far Attenuation"));
            ui.add_enabled_ui(l.use_atten, |ui| {
                grid(ui, "light_atten", |ui| {
                    num_row(ui, t, "Start", &mut l.atten_start, 0.0..=500.0, 0.0);
                    num_row(ui, t, "End / Range", &mut l.atten_end, 0.01..=500.0, 20.0);
                });
                t.r(ui.checkbox(&mut l.show_atten, "Show Attenuation Wireframe"));
            });
            l.atten_start = l.atten_start.min(l.atten_end);
        });
    });
}

fn shape(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(kind) = ed.current_obj().and_then(|o| o.light()).map(|l| l.kind) else { return };
    if kind.is_spot() {
        rollout(ui, "Spotlight Parameters", true, |ui| {
            edit(ed, |l, t| {
                grid(ui, "light_spot", |ui| {
                    num_row(ui, t, "Hotspot / Beam", &mut l.hotspot, 0.5..=178.5, 43.0);
                    num_row(ui, t, "Falloff / Field", &mut l.falloff, 1.0..=179.0, 45.0);
                });
                // Dragging hotspot past falloff pushes falloff along (3ds Max).
                if l.hotspot > l.falloff - 0.5 {
                    l.falloff = (l.hotspot + 0.5).min(179.0);
                }
            });
        });
    } else if kind.is_directional() {
        rollout(ui, "Directional Parameters", true, |ui| {
            edit(ed, |l, t| grid(ui, "light_dir", |ui| num_row(ui, t, "Beam Radius", &mut l.beam_radius, 0.01..=500.0, 2.0)));
            ui.label(dim("Parallel rays: only the direction matters for shading."));
        });
    } else if kind == LightKind::Skylight {
        ui.label(dim("Skylight adds soft hemisphere light to every surface (ambient in the viewport, sky dome in F9)."));
    }
}

fn shadows(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(l) = ed.current_obj().and_then(|o| o.light()) else { return };
    if l.kind == LightKind::Skylight {
        return;
    }
    rollout(ui, "Shadow Parameters", false, |ui| {
        edit(ed, |l, t| {
            grid(ui, "light_shadow", |ui| {
                num_row(ui, t, "Bias", &mut l.shadow_bias, 0.0..=1.0, 0.02);
                num_row(ui, t, "Softness / Radius", &mut l.softness, 0.0..=5.0, 0.0);
                color_row(ui, t, "Shadow Color", &mut l.shadow_color);
                num_row(ui, t, "Density", &mut l.shadow_density, 0.0..=1.0, 1.0);
            });
        });
        ui.label(dim("Shadow color and density are applied by the F9 renderer."));
    });
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    general(ui, ed);
    intensity(ui, ed);
    shape(ui, ed);
    shadows(ui, ed);
    if ed.uses_default_lights() {
        let why = if ed.default_lights { "Default Lights are forced on (Ctrl+L)" } else { "no scene light is on" };
        ui.label(dim(&format!("Viewport uses the Default Lights rig: {why}.")));
    }
}
