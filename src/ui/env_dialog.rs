//! Environment & Effects (`8`): 1-click lighting presets, background,
//! ambient tint, HDRI environment map, dynamic sky and exponential fog.

use crate::editor::Editor;
use crate::render::hdri::Hdri;
use crate::render::{Environment, LightingPreset};
use crate::ui::widgets::{Track, color_row, dim, num_row};
use bevy_egui::egui::{self, RichText};

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(120.0).show(ui, body);
}

/// Edits a clone of the environment; one undo step per gesture. True when
/// anything changed.
fn edit(ed: &mut Editor, body: impl FnOnce(&mut Environment, &mut Track)) -> bool {
    let mut env = ed.scene.environment.clone();
    let mut t = Track::default();
    body(&mut env, &mut t);
    if t.started {
        ed.checkpoint("Environment");
    }
    if t.changed {
        ed.scene.environment = env;
    }
    t.changed
}

/// Environment Map: an equirectangular `.hdr` / `.exr` HDRI.
fn hdri_rows(ui: &mut egui::Ui, env: &mut Environment, t: &mut Track) {
    ui.separator();
    ui.label(RichText::new("Environment Map (HDRI)").strong());
    ui.horizontal(|ui| {
        let name = env.hdri.as_ref().and_then(|h| h.path.file_name()).map_or("None".into(), |n| n.to_string_lossy().to_string());
        let pick = ui.button(format!("Map: {name}")).on_hover_text("Pick an equirectangular .hdr / .exr image");
        if pick.clicked()
            && let Some(path) = rfd::FileDialog::new().add_filter("HDR image", &["hdr", "exr"]).pick_file()
        {
            t.started = true;
            match Hdri::open(&path) {
                Ok(h) => {
                    ui.data_mut(|d| {
                        d.remove::<String>(egui::Id::new("hdri_err"));
                    });
                    env.hdri = Some(h);
                    t.changed = true;
                }
                Err(e) => {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("hdri_err"), e));
                }
            }
        }
        if env.hdri.is_some() && ui.button("Remove").clicked() {
            (t.started, t.changed) = (true, true);
            env.hdri = None;
        }
    });
    if let Some(e) = ui.data(|d| d.get_temp::<String>(egui::Id::new("hdri_err"))) {
        ui.label(dim(&format!("Could not load: {e}")));
    }
    let Some(h) = env.hdri.as_mut() else { return };
    if h.image.is_none() {
        ui.label(dim("Image not found on disk; pick it again."));
    }
    grid(ui, "env_hdri", |ui| {
        num_row(ui, t, "Strength", &mut h.strength, 0.0..=20.0, 1.0);
        num_row(ui, t, "Rotation", &mut h.rotation, -360.0..=360.0, 0.0);
    });
    t.r(ui.checkbox(&mut h.lighting, "Use for Lighting (image-based ambient)"));
    t.r(ui.checkbox(&mut h.show_in_viewport, "Show in Perspective Viewports"));
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
            egui::ScrollArea::vertical().max_height(680.0).min_scrolled_height(620.0).show(ui, |ui| {
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
                let changed = edit(ed, |env, t| {
                    ui.label(RichText::new("Common Parameters").strong());
                    grid(ui, "env_common", |ui| {
                        color_row(ui, t, "Background Color", &mut env.background);
                        color_row(ui, t, "Ambient Tint", &mut env.ambient);
                        num_row(ui, t, "Ambient Level", &mut env.ambient_level, 0.0..=4.0, 1.0);
                    });
                    t.r(ui.checkbox(&mut env.auto_default_lights, "Default Lights when the scene has none"));
                    hdri_rows(ui, env, t);
                    crate::ui::sky_dialog::rows(ui, env, t);
                    ui.separator();
                    ui.label(RichText::new("Atmosphere").strong());
                    t.r(ui.checkbox(&mut env.fog, "Exponential Fog"));
                    ui.add_enabled_ui(env.fog, |ui| {
                        grid(ui, "env_fog", |ui| {
                            color_row(ui, t, "Fog Color", &mut env.fog_color);
                            num_row(ui, t, "Density", &mut env.fog_density, 0.0..=2.0, 0.05);
                        });
                    });
                    crate::ui::volume_dialog::rows(ui, env, t);
                });
                if changed {
                    ed.link_sun_to_sky();
                }
                ui.separator();
                let lights = if ed.uses_default_lights() { "Default Lights (headlight rig)" } else { "Scene Lights" };
                ui.label(dim(&format!("Viewport lighting: {lights} · Ctrl+L toggles Default Lights")));
            });
        });
    if let Some(p) = preset {
        ed.apply_lighting_preset(p);
    }
    if !open {
        ed.env_open = false;
    }
}
