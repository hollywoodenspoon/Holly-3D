//! Clouds PR 1 rollouts: one Cloud object and the Cloud Field.

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::clouds::{CloudFieldParams, CloudLook, CloudParams, CloudType, RenderAs};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui;

/// The Cloud rollout; picking a preset resets the shape to that type.
pub fn cloud(ui: &mut egui::Ui, t: &mut Track, p: &mut CloudParams) {
    ui.horizontal(|ui| {
        ui.label("Type:");
        let mut k = p.kind;
        egui::ComboBox::from_id_salt("cloud_type").width(content_w() - 60.0).selected_text(k.label()).show_ui(ui, |ui| {
            for c in CloudType::ALL {
                t.r(ui.selectable_value(&mut k, c, c.label()).on_hover_text(c.tip()));
            }
        });
        if k != p.kind {
            *p = CloudParams { seed: p.seed, ..CloudParams::of(k) };
        }
    });
    seed_row(ui, t, &mut p.seed);
    let (dw, dh) = p.kind.default_size();
    let d = CloudParams::of(p.kind);
    grid(ui, "cloud_params", |ui| {
        row(ui, t, "Size:", &mut p.size, 1.0..=20000.0, dw, 1.0);
        row(ui, t, "Height:", &mut p.height, 1.0..=20000.0, dh, 1.0);
        row(ui, t, "Puffiness:", &mut p.puffiness, 0.0..=1.0, d.puffiness, 0.005);
        row(ui, t, "Lumpiness:", &mut p.lumpiness, 0.0..=1.0, d.lumpiness, 0.005);
        row(ui, t, "Base Flatness:", &mut p.base_flatness, 0.0..=1.0, d.base_flatness, 0.005);
        row(ui, t, "Wind Shear:", &mut p.shear, 0.0..=1.0, d.shear, 0.005);
        row(ui, t, "Wind Direction:", &mut p.shear_dir, -360.0..=360.0, 0.0, 0.5);
        row(ui, t, "Detail Levels:", &mut p.detail, 1..=3, 3, 0.05);
    });
    let n = p.shape().puffs.len();
    look(ui, t, &mut p.look, "cloud_look");
    ui.label(dim(&format!("{n} puffs. The viewport shows a light preview mesh; F9 renders the same puffs as soft density.")));
}

/// The Cloud Volume rows shared by a Cloud and a Cloud Field.
pub(crate) fn look(ui: &mut egui::Ui, t: &mut Track, l: &mut CloudLook, id: &str) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Render As:");
        egui::ComboBox::from_id_salt(id).width(content_w() - 90.0).selected_text(l.render_as.label()).show_ui(ui, |ui| {
            for r in RenderAs::ALL {
                t.r(ui.selectable_value(&mut l.render_as, r, r.label()).on_hover_text(r.tip()));
            }
        });
    });
    if l.render_as == RenderAs::Mesh {
        return;
    }
    let d = CloudLook::default();
    grid(ui, &format!("{id}_grid"), |ui| {
        row(ui, t, "Edge Softness:", &mut l.edge_softness, 0.05..=1.0, d.edge_softness, 0.005);
        row(ui, t, "Erosion:", &mut l.erosion, 0.0..=1.0, d.erosion, 0.005);
        row(ui, t, "Wispiness:", &mut l.wispiness, 0.0..=1.0, d.wispiness, 0.005);
        row(ui, t, "Density:", &mut l.density, 0.0..=20.0, d.density, 0.01);
    });
}

/// The Cloud Field rollout; returns true when `[ Break Into Clouds ]` is clicked.
pub fn field(ui: &mut egui::Ui, t: &mut Track, p: &mut CloudFieldParams) -> bool {
    seed_row(ui, t, &mut p.seed);
    let d = CloudFieldParams::default();
    grid(ui, "cloud_field", |ui| {
        row(ui, t, "Base Height:", &mut p.base_height, 0.0..=20000.0, d.base_height, 1.0);
        row(ui, t, "Layer Thickness:", &mut p.thickness, 1.0..=20000.0, d.thickness, 1.0);
        row(ui, t, "Width:", &mut p.width, 10.0..=100000.0, d.width, 5.0);
        row(ui, t, "Depth:", &mut p.depth, 10.0..=100000.0, d.depth, 5.0);
        row(ui, t, "Coverage:", &mut p.coverage, 0.0..=1.0, d.coverage, 0.005);
        row(ui, t, "Min Cloud Size:", &mut p.min_size, 1.0..=10000.0, d.min_size, 1.0);
        row(ui, t, "Max Cloud Size:", &mut p.max_size, 1.0..=10000.0, d.max_size, 1.0);
        row(ui, t, "Detail Levels:", &mut p.detail, 1..=3, d.detail, 0.05);
    });
    ui.label("Type Mix:");
    grid(ui, "cloud_mix", |ui| {
        for (i, k) in CloudType::ALL.iter().enumerate() {
            row(ui, t, k.label(), &mut p.mix[i], 0.0..=1.0, d.mix[i], 0.005);
        }
    });
    look(ui, t, &mut p.look, "field_look");
    let n = p.placements().len();
    ui.label(dim(&format!("{n} clouds, spaced so they never overlap.")));
    ui.add_sized([content_w(), 24.0], egui::Button::new("Break Into Clouds"))
        .on_hover_text("Turn the field into separate Cloud objects you can move one by one")
        .clicked()
}
