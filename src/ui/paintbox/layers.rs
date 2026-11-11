//! PaintBox layer list: the base material at the bottom (never painted
//! on), paint and fill layers above it with a visibility toggle, blend
//! mode and opacity each.

use super::W;
use crate::editor::Editor;
use crate::paint::{BlendMode, LayerKind, PaintStack};
use crate::ui::theme;
use crate::ui::widgets::{Track, action, dim, max_spinner_range};
use bevy_egui::egui::{self, RichText};

const SIZES: [u32; 4] = [512, 1024, 2048, 4096];

/// What a click in the list asked for.
enum Ask {
    Select(usize),
    Remove(usize),
    Shift(usize, bool),
}

/// One layer row; edits to `visible` go through `edit`.
fn row(ui: &mut egui::Ui, s: &PaintStack, i: usize, ask: &mut Option<Ask>, visible: &mut Option<(usize, bool)>) {
    let l = &s.layers[i];
    let active = i == s.active;
    ui.horizontal(|ui| {
        if ui.selectable_label(l.visible, "👁").on_hover_text("Show / hide this layer").clicked() {
            *visible = Some((i, !l.visible));
        }
        let icon = match (&l.kind, &l.smart) {
            (_, Some(_)) => "✨",
            (LayerKind::Paint(_), _) => "✏",
            (LayerKind::Fill { .. }, _) => "◼",
        };
        let text = RichText::new(format!("{icon} {}", l.name));
        let text = if active { text.color(theme::on_accent()).background_color(theme::amber()) } else { text };
        if ui.add(egui::Label::new(text).sense(egui::Sense::click()).truncate()).on_hover_text("Click to paint on this layer").clicked() {
            *ask = Some(Ask::Select(i));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("🗑").on_hover_text("Delete layer").clicked() {
                *ask = Some(Ask::Remove(i));
            }
            if ui.add_enabled(i > 0, egui::Button::new("⏷").small()).on_hover_text("Move down").clicked() {
                *ask = Some(Ask::Shift(i, false));
            }
            if ui.add_enabled(i + 1 < s.layers.len(), egui::Button::new("⏶").small()).on_hover_text("Move up").clicked() {
                *ask = Some(Ask::Shift(i, true));
            }
        });
    });
}

/// Blend mode and opacity of the active layer.
fn active_settings(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(s) = ed.paint_stack() else { return };
    let Some(l) = s.active_layer() else { return };
    let (i, mut blend, mut pct) = (s.active, l.blend, l.opacity * 100.0);
    let mut t = Track::default();
    egui::Grid::new("paint_layer").num_columns(2).spacing([10.0, 6.0]).min_col_width(90.0).show(ui, |ui| {
        ui.label("Blend");
        egui::ComboBox::from_id_salt("paint_blend").width(150.0).selected_text(blend.label()).show_ui(ui, |ui| {
            for b in BlendMode::ALL {
                ui.selectable_value(&mut blend, b, b.label());
            }
        });
        ui.end_row();
        let label = if blend == BlendMode::HeightBlend { "Fill Level %" } else { "Opacity %" };
        ui.label(label).on_hover_text("Height-Blend: how high the layer has risen out of the crevices");
        t.r(max_spinner_range(ui, &mut pct, 0.5, 100.0, 0.0..=100.0));
        ui.end_row();
    });
    if blend != l.blend {
        ed.edit_paint_stack("Layer Blend Mode", |s| s.layers[i].blend = blend);
    }
    if t.started {
        ed.checkpoint("Layer Opacity");
    }
    if t.changed
        && let Some(m) = ed.current_obj().and_then(|o| o.material)
        && let Some(s) = ed.scene.materials.mats[m].paint.as_mut()
    {
        s.layers[i].opacity = pct / 100.0;
        s.touch();
    }
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let has_obj = ed.current_obj().is_some_and(|o| !o.is_helper());
    ui.horizontal(|ui| {
        ui.label("Layer Size");
        egui::ComboBox::from_id_salt("canvas_size").width(110.0).selected_text(format!("{0}x{0}", ed.paint.canvas_size)).show_ui(ui, |ui| {
            for s in SIZES {
                ui.selectable_value(&mut ed.paint.canvas_size, s, format!("{s}x{s}"));
            }
        });
    });
    let (mut ask, mut visible) = (None, None);
    match ed.paint_stack() {
        Some(s) => {
            egui::Frame::new().fill(theme::input()).corner_radius(4).inner_margin(4).show(ui, |ui| {
                ui.set_width(W - 8.0);
                for i in (0..s.layers.len()).rev() {
                    row(ui, s, i, &mut ask, &mut visible);
                }
                let base = ed.current_obj().and_then(|o| o.material).map_or("Material".to_string(), |m| ed.scene.materials.mats[m].name.clone());
                ui.label(dim(&format!("🔒 Layer 0: Base Material ({base})"))).on_hover_text("The object's own material. Painting never changes it; erasing a layer shows it again.");
            });
            let mb = s.bytes() as f32 / (1024.0 * 1024.0);
            ui.label(dim(&format!("{0}x{0} · {1} layers · {mb:.1} MB painted", s.size, s.layers.len())));
        }
        None => {
            ui.label(dim("No paint layers on the selected object yet. The first stroke keeps its material as Layer 0 and adds a Paint Layer."));
            if action(ui, "Auto-Setup Paint Canvas", has_obj, W) {
                ed.auto_setup_canvas();
            }
            ui.label(dim("Existing UVs are kept. Only an object without UVs gets a Smart UV Project."));
        }
    }
    match ask {
        Some(Ask::Select(i)) => ed.set_active_paint_layer(i),
        Some(Ask::Remove(i)) => ed.remove_paint_layer(i),
        Some(Ask::Shift(i, up)) => ed.shift_paint_layer(i, up),
        None => {}
    }
    if let Some((i, on)) = visible {
        ed.edit_paint_stack("Layer Visibility", |s| s.layers[i].visible = on);
    }
    ui.horizontal(|ui| {
        let w = (W - 6.0) / 2.0;
        if action(ui, "➕ Paint Layer", has_obj, w) {
            ed.add_paint_layer();
        }
        if action(ui, "➕ Fill Layer", has_obj, w) {
            ed.add_fill_layer();
        }
    });
    active_settings(ui, ed);
    if ed.paint_stack().is_some() {
        if ed.paint.save_path.is_empty() {
            ed.paint.save_path = "textures".into();
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut ed.paint.save_path).desired_width(150.0)).on_hover_text("Folder the PNGs are written to");
            if ui.button("💾 Export Textures").on_hover_text("Base colour, ORM, normal and emission PNGs of the painted material").clicked() {
                let p = ed.paint.save_path.clone();
                ed.export_paint_textures(&p);
            }
        });
    }
}
