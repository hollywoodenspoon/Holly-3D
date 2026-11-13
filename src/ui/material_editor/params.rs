//! Basic Parameters (per shading model) and the universal Ink Outline rollout.

use super::edit;
use crate::editor::Editor;
use crate::materials::{Mapping, Material, ShadingModel};
use crate::ui::widgets::{Track, dim, max_spinner_range, rollout, toggle_button};
pub(super) use crate::ui::widgets::{color_row as color, num_row as num};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, body);
}

/// Luminance / Emission: above 0 the surface lights the scene in F9.
fn emission(ui: &mut egui::Ui, t: &mut Track, m: &mut Material) {
    color(ui, t, "Luminance / Emission", &mut m.emissive);
    num(ui, t, "Emission Multiplier", &mut m.emissive_strength, 0.0..=100.0, 0.0);
    t.r(ui.checkbox(&mut m.preserve_emission_texture, "Preserve Texture Appearance")).on_hover_text(
        "Decouple Luminance: the Emission Multiplier still lights the scene, but the camera sees this material in its own colours, never washed out",
    );
    ui.label(dim("(Decouple Luminance)"));
    ui.end_row();
    ui.add_enabled_ui(m.preserve_emission_texture, |ui| ui.label("Direct Camera Brightness"));
    ui.add_enabled_ui(m.preserve_emission_texture, |ui| t.r(max_spinner_range(ui, &mut m.direct_camera_brightness, 0.01, 1.0, 0.0..=2.0)));
    ui.end_row();
}

/// "Alpha: Blend / Cut-out": how texture alpha and Opacity show.
fn alpha_mode(ui: &mut egui::Ui, t: &mut Track, m: &mut Material) {
    ui.label("Alpha");
    let label = if m.alpha_cutout { "Cut-out" } else { "Blend" };
    egui::ComboBox::from_id_salt("mat_alpha_mode").selected_text(label).show_ui(ui, |ui| {
        t.r(ui.selectable_value(&mut m.alpha_cutout, false, "Blend")).on_hover_text("See-through by the alpha amount (glass decals, smoke)");
        t.r(ui.selectable_value(&mut m.alpha_cutout, true, "Cut-out"))
            .on_hover_text("Alpha is either solid or a hole, at 50 %: hair cards, lashes, leaves, fences");
    });
    ui.end_row();
}

fn physical(ui: &mut egui::Ui, t: &mut Track, m: &mut Material) {
    color(ui, t, "Base Color", &mut m.base_color);
    num(ui, t, "Metallic", &mut m.metallic, 0.0..=1.0, 0.0);
    num(ui, t, "Roughness", &mut m.roughness, 0.0..=1.0, 0.6);
    num(ui, t, "Opacity", &mut m.opacity, 0.0..=1.0, 1.0);
    alpha_mode(ui, t, m);
    ui.label("Catcher");
    t.r(ui.checkbox(&mut m.shadow_catcher, "Shadow & Reflection Catcher"))
        .on_hover_text("Transparent in the render's alpha except for shadows, AO and reflections (CPU Path Tracer)");
    ui.end_row();
    emission(ui, t, m);
}

fn toon(ui: &mut egui::Ui, t: &mut Track, m: &mut Material) {
    color(ui, t, "Lit Color", &mut m.base_color);
    color(ui, t, "Shadow Tint", &mut m.toon.shadow_tint);
    ui.label("Ramp Steps");
    t.r(max_spinner_range(ui, &mut m.toon.steps, 0.05, 2, 2..=5));
    ui.end_row();
    num(ui, t, "Rim Highlight", &mut m.toon.rim, 0.0..=1.0, 0.0);
    color(ui, t, "Rim Color", &mut m.toon.rim_color);
    num(ui, t, "Specular", &mut m.toon.specular, 0.0..=1.0, 0.0);
    ui.label("Screentone");
    t.r(ui.checkbox(&mut m.toon.screentone, "Manga Halftone Shadows"));
    ui.end_row();
    num(ui, t, "Screentone Cell (px)", &mut m.toon.screentone_scale, 2.0..=32.0, 6.0);
    num(ui, t, "Highlight Size", &mut m.roughness, 0.02..=1.0, 0.5);
    num(ui, t, "Opacity", &mut m.opacity, 0.0..=1.0, 1.0);
    alpha_mode(ui, t, m);
    emission(ui, t, m);
}

fn crystal(ui: &mut egui::Ui, t: &mut Track, m: &mut Material) {
    color(ui, t, "Surface Tint", &mut m.base_color);
    num(ui, t, "Specular Transmission", &mut m.transmission, 0.0..=1.0, 0.92);
    num(ui, t, "IOR", &mut m.ior, 1.0..=3.0, 1.55);
    num(ui, t, "Roughness", &mut m.roughness, 0.0..=1.0, 0.06);
    num(ui, t, "Thickness", &mut m.thickness, 0.0..=10.0, 0.35);
    color(ui, t, "Attenuation Color", &mut m.attenuation_color);
    emission(ui, t, m);
}

fn unlit(ui: &mut egui::Ui, t: &mut Track, m: &mut Material) {
    color(ui, t, "Color", &mut m.base_color);
    num(ui, t, "Opacity", &mut m.opacity, 0.0..=1.0, 1.0);
    emission(ui, t, m);
}

pub fn basic(ui: &mut egui::Ui, ed: &mut Editor) {
    let model = ed.editing_material().model;
    let title = match model {
        ShadingModel::Physical => "Physical Basic Parameters",
        ShadingModel::Toon => "Cel / Toon Ramp Parameters",
        ShadingModel::Crystal => "Crystal / Glass Parameters",
        ShadingModel::Unlit => "Unlit Parameters",
    };
    rollout(ui, title, true, |ui| {
        edit(ed, |m, t| {
            grid(ui, "mat_basic", |ui| match m.model {
                ShadingModel::Physical => physical(ui, t, m),
                ShadingModel::Toon => toon(ui, t, m),
                ShadingModel::Crystal => crystal(ui, t, m),
                ShadingModel::Unlit => unlit(ui, t, m),
            });
            None
        });
    });
}

/// Mapping: the mesh's UVs, or World / Triplanar projection for meshes
/// without UVs (Settlement, Terrain, GeoGen rocks, Battlefield rubble).
pub fn mapping(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Mapping", ed.editing_material().projection.on(), |ui| {
        edit(ed, |m, t| {
            let p = &mut m.projection;
            ui.horizontal(|ui| {
                for mode in Mapping::ALL {
                    if toggle_button(ui, p.mapping == mode, mode.label()).clicked() && p.mapping != mode {
                        p.mapping = mode;
                        t.started = true;
                        t.changed = true;
                    }
                }
            });
            ui.add_enabled_ui(p.on(), |ui| {
                grid(ui, "mat_mapping", |ui| {
                    num(ui, t, "Tile Size (units)", &mut p.size, 0.01..=1000.0, 1.0);
                    num(ui, t, "Edge Blend", &mut p.blend, 0.0..=1.0, 0.3);
                });
            });
            ui.label(dim("World / Triplanar projects maps and patterns from three sides, so they tile seamlessly without UVs. Paint layers always use the UVs."));
            None
        });
    });
}

pub fn outline(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Ink / Toon Outline", true, |ui| {
        edit(ed, |m, t| {
            let o = &mut m.outline;
            t.r(ui.checkbox(&mut o.enabled, "Render Ink Outline"));
            ui.add_enabled_ui(o.enabled, |ui| {
                grid(ui, "mat_outline", |ui| {
                    num(ui, t, "Outline Width", &mut o.width, 0.0..=10.0, 2.0);
                    color(ui, t, "Outline Color", &mut o.color);
                });
                t.r(ui.checkbox(&mut o.crease_lines, "Draw Crease / Sharp Angle Lines"));
                ui.add_enabled_ui(o.crease_lines, |ui| {
                    grid(ui, "mat_crease", |ui| num(ui, t, "Crease Angle", &mut o.crease_angle, 1.0..=179.0, 40.0));
                });
                t.r(ui.checkbox(&mut o.pen_lines, "Interior Pen Lines (Curvature + Panel Grooves)"));
            });
            ui.label(dim("Inverted-hull outline + sharp-edge lines; works on every shading model."));
            None
        });
    });
}
