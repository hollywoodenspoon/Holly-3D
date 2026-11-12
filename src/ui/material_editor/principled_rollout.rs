//! Principled BSDF rollout: Subsurface Scattering, Anisotropy, Clearcoat,
//! Cloth Sheen, Volume Absorption and Dispersion, plus one-click looks.
//! Path-traced in F9 (Path Tracer engine).

use super::edit;
use super::params::{color, num};
use crate::editor::Editor;
use crate::materials::ShadingModel;
use crate::materials::principled::{PRESETS, Principled};
use crate::ui::widgets::{dim, rollout};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, body);
}

pub fn principled(ui: &mut egui::Ui, ed: &mut Editor) {
    let m = ed.editing_material();
    if matches!(m.model, ShadingModel::Toon | ShadingModel::Unlit) {
        return;
    }
    let open = m.principled.active();
    rollout(ui, "Principled BSDF (SSS / Coat / Sheen)", open, |ui| {
        edit(ed, |m, t| {
            ui.horizontal_wrapped(|ui| {
                for (name, apply) in PRESETS {
                    if ui.button(name).on_hover_text("Apply this look (other Principled settings reset)").clicked() {
                        let mut p = Principled::default();
                        apply(&mut p);
                        m.principled = p;
                        (t.started, t.changed) = (true, true);
                    }
                }
                if ui.button("Clear").on_hover_text("Turn every Principled lobe off").clicked() {
                    m.principled = Principled::default();
                    (t.started, t.changed) = (true, true);
                }
            });
            let p = &mut m.principled;
            ui.label(dim("Subsurface Scattering"));
            grid(ui, "pr_sss", |ui| {
                num(ui, t, "SSS Weight", &mut p.sss_weight, 0.0..=1.0, 0.0);
                color(ui, t, "Scatter Color", &mut p.sss_color);
                num(ui, t, "Radius R", &mut p.sss_radius[0], 0.0..=10.0, 1.0);
                num(ui, t, "Radius G", &mut p.sss_radius[1], 0.0..=10.0, 0.35);
                num(ui, t, "Radius B", &mut p.sss_radius[2], 0.0..=10.0, 0.2);
                num(ui, t, "Scale", &mut p.sss_scale, 0.001..=10.0, 0.1);
            });
            ui.label(dim("Specular Layers"));
            grid(ui, "pr_spec", |ui| {
                num(ui, t, "Anisotropy", &mut p.anisotropy, 0.0..=1.0, 0.0);
                num(ui, t, "Aniso Rotation", &mut p.aniso_rotation, 0.0..=360.0, 0.0);
                num(ui, t, "Clearcoat", &mut p.clearcoat, 0.0..=1.0, 0.0);
                num(ui, t, "Coat Roughness", &mut p.clearcoat_roughness, 0.0..=1.0, 0.03);
                num(ui, t, "Cloth Sheen", &mut p.sheen, 0.0..=1.0, 0.0);
                num(ui, t, "Sheen Tint", &mut p.sheen_tint, 0.0..=1.0, 0.5);
            });
            ui.label(dim("Glass / Crystal Volume"));
            grid(ui, "pr_vol", |ui| {
                color(ui, t, "Absorption Color", &mut p.absorption_color);
                num(ui, t, "Absorption Density", &mut p.absorption_density, 0.0..=20.0, 0.0);
                num(ui, t, "Dispersion (Abbe)", &mut p.abbe, 0.0..=90.0, 0.0);
            });
            ui.label(dim("Abbe 0 = off; 20 strong rainbow .. 90 faint. Rendered by the F9 Path Tracer (CPU)."));
            None
        });
    });
}
