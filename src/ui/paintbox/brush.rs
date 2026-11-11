//! PaintBox brush section: tool, colour or material preset, PBR channels,
//! size / opacity / falloff, and the brush alphas and damage stamps.

use super::W;
use crate::editor::Editor;
use crate::materials::library;
use crate::paint::layers::{CH_COLOR, CH_EMISSION, CH_HEIGHT, CH_METAL, CH_ROUGH};
use crate::paint::{Alpha, BrushTool, PALETTE, PaintMode};
use crate::ui::widgets::{action, color32, dim, max_spinner_range, toggle_button};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(90.0).show(ui, body);
}

/// `Paint with: [ Color | Material preset ]` and the preset list.
fn source(ui: &mut egui::Ui, ed: &mut Editor) {
    let name = ed.paint.material.as_ref().map_or("Color".to_string(), |m| m.name.clone());
    ui.horizontal(|ui| {
        ui.label("Paint with");
        egui::ComboBox::from_id_salt("paint_source").width(190.0).selected_text(name).height(320.0).show_ui(ui, |ui| {
            if ui.selectable_label(ed.paint.material.is_none(), "Color (the swatch below)").clicked() {
                ed.paint.material = None;
            }
            if ui.selectable_label(false, "Active Material Editor slot").on_hover_text("Paint with the material in the active sample slot").clicked() {
                ed.paint.material = Some(Box::new(ed.editing_material().clone()));
            }
            let mut last = None;
            for (cat, item) in library::all() {
                if last != Some(cat) {
                    last = Some(cat);
                    ui.separator();
                    ui.label(dim(cat.label()));
                }
                let on = ed.paint.material.as_ref().is_some_and(|m| m.name == item.name());
                if ui.selectable_label(on, item.name()).clicked() {
                    ed.paint.material = Some(Box::new(item.material()));
                }
            }
        });
    });
    if ed.paint.material.is_some() {
        grid(ui, "paint_mat", |ui| {
            ui.label("Tiling");
            max_spinner_range(ui, &mut ed.paint.material_tiling, 0.02, 1.0, 0.05..=64.0);
            ui.end_row();
        });
        ui.label(dim("One stroke paints the preset's colour, relief, roughness, metallic and glow."));
        return;
    }
    // Flat colour: the PBR channels it writes besides colour.
    let look = &mut ed.paint.look;
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [(CH_COLOR, "Color"), (CH_ROUGH, "Rough"), (CH_METAL, "Metal"), (CH_HEIGHT, "Height"), (CH_EMISSION, "Glow")] {
            ui.checkbox(&mut look.channels[i], label).on_hover_text("Channels this brush writes");
        }
    });
    grid(ui, "paint_look", |ui| {
        if look.channels[CH_ROUGH] {
            ui.label("Roughness");
            max_spinner_range(ui, &mut look.rough, 0.005, 0.6, 0.0..=1.0);
            ui.end_row();
        }
        if look.channels[CH_METAL] {
            ui.label("Metallic");
            max_spinner_range(ui, &mut look.metal, 0.005, 0.0, 0.0..=1.0);
            ui.end_row();
        }
        if look.channels[CH_HEIGHT] {
            ui.label("Height").on_hover_text("0.5 is level; lower digs in, higher stands out");
            max_spinner_range(ui, &mut look.height, 0.005, 0.5, 0.0..=1.0);
            ui.end_row();
        }
        if look.channels[CH_EMISSION] {
            ui.label("Glow");
            max_spinner_range(ui, &mut look.emission, 0.005, 0.0, 0.0..=1.0);
            ui.end_row();
        }
    });
}

/// The brush alpha / damage stamp buttons (three per row).
fn alphas(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.label("Brush Alphas & Damage Stamps");
    egui::Grid::new("paint_alphas").num_columns(3).spacing([4.0, 4.0]).show(ui, |ui| {
        for (i, a) in Alpha::ALL.iter().enumerate() {
            let short = a.label().split(" / ").next().unwrap_or(a.label());
            if toggle_button(ui, ed.paint.brush.alpha == *a, short).on_hover_text(a.label()).clicked() {
                ed.paint.brush.alpha = *a;
                ed.paint.brush.tool = BrushTool::Paint;
            }
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
    if ed.paint.brush.alpha.is_stamp() {
        ui.label(dim("Stamps bring their own colour, depth, roughness and glow. Click to place one, drag to scatter."));
    }
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let texture = ed.paint.mode == PaintMode::Texture;
    ui.horizontal(|ui| {
        for tool in [BrushTool::Paint, BrushTool::Blur, BrushTool::Erase] {
            let tip = match tool {
                BrushTool::Paint => "Paint on the active layer",
                BrushTool::Blur => "Soften what is painted",
                BrushTool::Erase => "Erase the active layer: the layers and base material below show again",
            };
            if toggle_button(ui, ed.paint.brush.tool == tool, tool.label()).on_hover_text(tip).clicked() {
                ed.paint.brush.tool = tool;
            }
        }
    });
    if texture {
        source(ui, ed);
    }
    if !texture || ed.paint.material.is_none() {
        ui.horizontal(|ui| {
            ui.color_edit_button_rgb(&mut ed.paint.brush.color);
            ui.spacing_mut().item_spacing.x = 3.0;
            for c in PALETTE {
                let (rect, r) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 2.0, color32(c));
                if r.on_hover_text("Pick swatch").clicked() {
                    ed.paint.brush.color = c;
                }
            }
        });
    }
    let b = &mut ed.paint.brush;
    let mut opacity = b.opacity * 100.0;
    grid(ui, "paint_brush", |ui| {
        ui.label("Size");
        max_spinner_range(ui, &mut b.size, 0.005, 0.35, 0.005..=100.0);
        ui.end_row();
        ui.label("Opacity %");
        max_spinner_range(ui, &mut opacity, 0.5, 100.0, 0.0..=100.0);
        ui.end_row();
        ui.label("Falloff");
        max_spinner_range(ui, &mut b.falloff, 0.005, 0.5, 0.0..=1.0);
        ui.end_row();
    });
    b.opacity = opacity / 100.0;
    if texture {
        alphas(ui, ed);
    }
    ui.checkbox(&mut ed.paint.selected_only, "Selected Faces Only");
    let fill = if texture { "Fill Layer" } else { "Fill Object" };
    if action(ui, fill, ed.current().is_some(), W) {
        ed.paint_fill();
    }
}
