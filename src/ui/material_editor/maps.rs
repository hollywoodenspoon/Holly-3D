//! Maps rollout (channel slots with amount spinners) and the shared map-node
//! parameter editor used by both Compact and Slate views.

use super::params::{color, num};
use super::{craft_params, edit};
use crate::editor::Editor;
use crate::materials::maps::NoiseKind;
use crate::materials::patterns::{BrickKind, GrungeKind};
use crate::materials::{MapChannel, MapNode, import};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, max_spinner_range, rollout as card};
use bevy_egui::egui::{self, RichText};

/// Combo for a map node kind; `None` clears the slot. Returns true on change.
pub(super) fn kind_combo(ui: &mut egui::Ui, id: egui::Id, node: &mut Option<MapNode>, t: &mut Track) {
    let cur = node.as_ref().map_or("None", |n| n.kind_name());
    egui::ComboBox::from_id_salt(id).width(118.0).selected_text(cur).show_ui(ui, |ui| {
        if ui.selectable_label(node.is_none(), "None").clicked() && node.is_some() {
            *node = None;
            (t.started, t.changed) = (true, true);
        }
        for (i, k) in MapNode::KINDS.iter().enumerate() {
            if ui.selectable_label(cur == *k, *k).clicked() && cur != *k {
                // Color Correction wraps whatever was there.
                *node = Some(MapNode::new_kind(i, node.take()));
                (t.started, t.changed) = (true, true);
            }
        }
    });
}

/// Parameter grid for one node (recursing into Color Correction's input).
/// Returns a status message (bitmap load result).
pub(super) fn node_params(ui: &mut egui::Ui, salt: &str, node: &mut MapNode, t: &mut Track) -> Option<String> {
    let mut msg = None;
    egui::Grid::new(("node", salt)).num_columns(2).spacing([10.0, 5.0]).min_col_width(96.0).show(ui, |ui| match &mut *node {
        MapNode::Bitmap { path, tex } => {
            ui.label("File");
            t.r(ui.add(egui::TextEdit::singleline(path).desired_width(150.0).hint_text("/path/to/image.png")));
            ui.end_row();
            ui.label(match tex {
                Some(h) => dim(&format!("{} x {}", h.tex.width, h.tex.height)),
                None => dim("not loaded"),
            });
            if ui.button("Load Bitmap").clicked() {
                match import::bitmap_node(std::path::Path::new(path.trim())) {
                    Ok(MapNode::Bitmap { tex: loaded, .. }) => {
                        *tex = loaded;
                        (t.started, t.changed) = (true, true);
                        msg = Some("Bitmap loaded".into());
                    }
                    Ok(_) => {}
                    Err(e) => msg = Some(e),
                }
            }
            ui.end_row();
        }
        MapNode::Noise { kind, scale, seed, color_a, color_b } => {
            ui.label("Type");
            ui.horizontal(|ui| {
                for (k, l) in [(NoiseKind::Perlin, "Perlin"), (NoiseKind::Fractal, "Fractal"), (NoiseKind::Cellular, "Cellular")] {
                    t.r(ui.selectable_value(kind, k, l));
                }
            });
            ui.end_row();
            num(ui, t, "Size", scale, 0.1..=50.0, 4.0);
            seed_row(ui, t, seed);
            color(ui, t, "Color #1", color_a);
            color(ui, t, "Color #2", color_b);
        }
        MapNode::Checker { tiles, color_a, color_b } => {
            num(ui, t, "Tiling", tiles, 1.0..=64.0, 8.0);
            color(ui, t, "Color #1", color_a);
            color(ui, t, "Color #2", color_b);
        }
        MapNode::Gradient { color_a, color_b, color_c, mid, radial } => {
            color(ui, t, "Color #1", color_a);
            color(ui, t, "Color #2", color_b);
            color(ui, t, "Color #3", color_c);
            num(ui, t, "Color 2 Position", mid, 0.01..=0.99, 0.5);
            ui.label("Type");
            t.r(ui.checkbox(radial, "Radial"));
            ui.end_row();
        }
        MapNode::Wood { scale, rings, grain, light, dark } => {
            num(ui, t, "Size", scale, 0.05..=20.0, 1.0);
            num(ui, t, "Rings", rings, 1.0..=60.0, 12.0);
            num(ui, t, "Grain Noise", grain, 0.0..=3.0, 0.6);
            color(ui, t, "Light Wood", light);
            color(ui, t, "Dark Wood", dark);
        }
        MapNode::ColorCorrect { brightness, contrast, saturation, hue, tint, .. } => {
            num(ui, t, "Brightness", brightness, -1.0..=1.0, 0.0);
            num(ui, t, "Contrast", contrast, 0.0..=4.0, 1.0);
            num(ui, t, "Saturation", saturation, 0.0..=4.0, 1.0);
            num(ui, t, "Hue Shift", hue, -180.0..=180.0, 0.0);
            color(ui, t, "Tint", tint);
        }
        MapNode::Bricks { kind, cols, rows, gap, vary, seed, color_a, color_b } => {
            ui.label("Pattern");
            egui::ComboBox::from_id_salt(("brick_kind", salt)).selected_text(kind.label()).show_ui(ui, |ui| {
                for k in BrickKind::ALL {
                    t.r(ui.selectable_value(kind, k, k.label()));
                }
            });
            ui.end_row();
            num(ui, t, "Columns", cols, 0.5..=64.0, 4.0);
            num(ui, t, "Rows", rows, 0.5..=64.0, 8.0);
            num(ui, t, "Mortar Gap", gap, 0.0..=0.6, 0.1);
            num(ui, t, "Color Variation", vary, 0.0..=1.0, 0.3);
            seed_row(ui, t, seed);
            color(ui, t, "Brick", color_a);
            color(ui, t, "Mortar", color_b);
        }
        MapNode::Stripes { count, width, dash, soft, along_u, color_a, color_b } => {
            num(ui, t, "Count", count, 0.1..=100.0, 1.0);
            num(ui, t, "Line Width", width, 0.0..=1.0, 0.06);
            num(ui, t, "Dashes (0 = solid)", dash, 0.0..=50.0, 0.0);
            ui.label("Options");
            ui.horizontal(|ui| {
                t.r(ui.checkbox(soft, "Corrugated"));
                t.r(ui.checkbox(along_u, "Along U"));
            });
            ui.end_row();
            color(ui, t, "Line", color_a);
            color(ui, t, "Ground", color_b);
        }
        MapNode::Cracks { scale, width, vary, seed, color_a, color_b } => {
            num(ui, t, "Size", scale, 0.1..=50.0, 3.0);
            num(ui, t, "Crack Width", width, 0.0..=0.8, 0.06);
            num(ui, t, "Color Variation", vary, 0.0..=1.0, 0.2);
            seed_row(ui, t, seed);
            color(ui, t, "Surface", color_a);
            color(ui, t, "Crack", color_b);
        }
        MapNode::Grunge { kind, scale, amount, seed, color_a, color_b } => {
            ui.label("Type");
            egui::ComboBox::from_id_salt(("grunge_kind", salt)).selected_text(kind.label()).show_ui(ui, |ui| {
                for k in GrungeKind::ALL {
                    t.r(ui.selectable_value(kind, k, k.label()));
                }
            });
            ui.end_row();
            num(ui, t, "Size", scale, 0.1..=50.0, 2.0);
            num(ui, t, "Amount", amount, 0.0..=1.0, 0.4);
            seed_row(ui, t, seed);
            color(ui, t, "Clean", color_a);
            color(ui, t, "Dirt", color_b);
        }
        other => _ = craft_params::rows(ui, salt, other, t),
    });
    // Input sockets: pick what feeds each one and edit it in place.
    let many = node.input_count() > 1;
    for (i, (name, input)) in node.inputs_mut().into_iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{name} Map"));
            let mut inner = Some(input.clone());
            kind_combo(ui, egui::Id::new((salt, "in", i)), &mut inner, t);
            // A socket always needs a map; "None" falls back to Noise.
            *input = inner.unwrap_or_else(|| MapNode::new_kind(1, None));
        });
        let sub = format!("{salt}/in{i}");
        let title = dim(&format!("{name}: {}", input.kind_name()));
        let mut body = |ui: &mut egui::Ui| {
            if let Some(m) = node_params(ui, &sub, input, t) {
                msg = Some(m);
            }
        };
        if many {
            egui::CollapsingHeader::new(title).id_salt((salt, "in_body", i)).show(ui, body);
        } else {
            ui.indent((salt, "in_body", i), &mut body);
        }
    }
    msg
}

fn seed_row(ui: &mut egui::Ui, t: &mut Track, seed: &mut u32) {
    ui.label("Seed");
    t.r(max_spinner_range(ui, seed, 0.2, 0, 0..=9999));
    ui.end_row();
}

pub fn rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    card(ui, "Maps", true, |ui| {
        edit(ed, |m, t| {
            let mut msg = None;
            for ch in MapChannel::ALL {
                let slot = m.map_mut(ch);
                ui.horizontal(|ui| {
                    t.r(ui.checkbox(&mut slot.enabled, ""));
                    ui.add_sized([104.0, 20.0], egui::Label::new(ch.label()).truncate());
                    t.r(max_spinner_range(ui, &mut slot.amount, 0.5, 100.0, 0.0..=100.0));
                    kind_combo(ui, egui::Id::new(("map_kind", ch as usize)), &mut slot.node, t);
                });
                if let Some(node) = slot.node.as_mut() {
                    let title = RichText::new(format!("{} — {}", ch.label(), node.kind_name())).small();
                    egui::CollapsingHeader::new(title).id_salt(("map_body", ch as usize)).default_open(false).show(ui, |ui| {
                        if let Some(s) = node_params(ui, &format!("ch{}", ch as usize), node, t) {
                            msg = Some(s);
                        }
                    });
                }
            }
            msg
        });
        ui.separator();
        ui.label(dim("Import PBR Texture Folder (albedo / metallic / roughness / normal / opacity / emissive):"));
        ui.add(egui::TextEdit::singleline(&mut ed.mat_editor.pbr_folder).desired_width(CONTENT_W).hint_text("/path/to/textures"));
        if ui.add_sized([CONTENT_W, 24.0], egui::Button::new("📁 Import PBR Texture Folder")).clicked() {
            ed.import_pbr_folder();
        }
    });
}
