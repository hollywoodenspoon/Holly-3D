//! PaintBox floater (3ds Max VertexPaint palette) for Viewport Paint, plus
//! the VertexPaint modifier rollout in the Modify panel.

use crate::editor::Editor;
use crate::modifiers::ModifierKind;
use crate::paint::{BrushTool, PALETTE, PaintMode};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, action, color32, dim, max_spinner_range, rollout, toggle_button};
use bevy_egui::egui::{self, RichText};

const SIZES: [u32; 4] = [512, 1024, 2048, 4096];

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(90.0).show(ui, body);
}

/// Layer Opacity spinner + Clear for the object's VertexPaint modifier.
fn layer_controls(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(i) = ed.vertex_paint_modifier(false) else {
        ui.label(dim("The first stroke adds a VertexPaint modifier on top of the stack."));
        return;
    };
    let Some(ModifierKind::VertexPaint(vp)) = ed.current_obj().map(|o| &o.modifiers[i].kind) else { return };
    let (mut pct, painted) = (vp.layer_opacity * 100.0, vp.layer.len());
    let mut t = Track::default();
    grid(ui, "vp_layer", |ui| {
        ui.label("Layer Opacity");
        t.r(max_spinner_range(ui, &mut pct, 0.5, 100.0, 0.0..=100.0));
        ui.end_row();
    });
    if t.started {
        ed.checkpoint("Layer Opacity");
    }
    if t.changed
        && let Some(o) = ed.current_obj_mut()
    {
        if let ModifierKind::VertexPaint(vp) = &mut o.modifiers[i].kind {
            vp.layer_opacity = pct / 100.0;
        }
        o.evaluate();
    }
    ui.horizontal(|ui| {
        ui.label(dim(&format!("{painted} painted vertices")));
        if ui.button("Clear Layer").clicked() {
            ed.clear_vertex_paint();
        }
    });
}

fn canvas_controls(ui: &mut egui::Ui, ed: &mut Editor) {
    let canvas = ed.current().and_then(|id| ed.canvas_of(id));
    let info = canvas.and_then(|m| crate::materials::bake::direct_base(&ed.scene.materials.mats[m])).map(|h| (h.tex.width, h.tex.height));
    match info {
        Some((w, h)) => ui.label(RichText::new(format!("Canvas: {w}x{h} Base Color bitmap")).color(crate::ui::theme::AMBER)),
        None => ui.label(dim("No canvas on the selected object yet.")),
    };
    ui.horizontal(|ui| {
        ui.label("Canvas Size");
        egui::ComboBox::from_id_salt("canvas_size").width(110.0).selected_text(format!("{0}x{0}", ed.paint.canvas_size)).show_ui(
            ui,
            |ui| {
                for s in SIZES {
                    ui.selectable_value(&mut ed.paint.canvas_size, s, format!("{s}x{s}"));
                }
            },
        );
    });
    if action(ui, "Auto-Setup Paint Canvas", ed.current().is_some(), 280.0) {
        ed.auto_setup_canvas();
    }
    ui.label(dim("Runs Smart UV Project if the UVs overlap, then attaches a blank canvas."));
    if info.is_some() {
        if ed.paint.save_path.is_empty() {
            let name = ed.current_obj().map(|o| o.name.replace(|c: char| !c.is_ascii_alphanumeric(), "_")).unwrap_or_default();
            ed.paint.save_path = format!("{name}_canvas.png");
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut ed.paint.save_path).desired_width(170.0));
            if ui.button("💾 Save PNG").clicked() {
                let p = ed.paint.save_path.clone();
                ed.save_canvas(&p);
            }
        });
    }
}

fn brush(ui: &mut egui::Ui, ed: &mut Editor) {
    let texture = ed.paint.mode == PaintMode::Texture;
    if texture && ed.paint.brush.tool == BrushTool::Erase {
        ed.paint.brush.tool = BrushTool::Paint;
    }
    ui.horizontal(|ui| {
        for tool in [BrushTool::Paint, BrushTool::Blur, BrushTool::Erase] {
            let ok = !(texture && tool == BrushTool::Erase);
            let r = ui.add_enabled_ui(ok, |ui| toggle_button(ui, ed.paint.brush.tool == tool, tool.label())).inner;
            if r.clicked() {
                ed.paint.brush.tool = tool;
            }
        }
    });
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
    ui.checkbox(&mut ed.paint.selected_only, "Selected Faces Only");
    if action(ui, "Fill Object", ed.current().is_some(), 280.0) {
        ed.paint_fill();
    }
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.paint.active {
        return;
    }
    let mut open = true;
    egui::Window::new(RichText::new("PaintBox").strong())
        .id(egui::Id::new("paint_box"))
        .open(&mut open)
        .default_pos([260.0, 90.0])
        .default_width(300.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (m, label) in [(PaintMode::Vertex, "Vertex Paint"), (PaintMode::Texture, "Texture Paint")] {
                    if toggle_button(ui, ed.paint.mode == m, label).clicked() {
                        ed.paint.mode = m;
                    }
                }
            });
            ui.separator();
            ui.label(RichText::new("Brush").strong());
            brush(ui, ed);
            ui.separator();
            match ed.paint.mode {
                PaintMode::Vertex => {
                    ui.label(RichText::new("VertexPaint Layer").strong());
                    layer_controls(ui, ed);
                }
                PaintMode::Texture => {
                    ui.label(RichText::new("Paint Canvas").strong());
                    canvas_controls(ui, ed);
                }
            }
            ui.separator();
            let target = ed.current_obj().map_or("nothing selected".to_string(), |o| o.name.clone());
            ui.label(dim(&format!("Painting: {target} · right-click a viewport to exit")));
        });
    if !open {
        ed.toggle_paint();
    }
}

/// Modify panel rollout when a VertexPaint modifier is highlighted.
pub fn modifier_rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "VertexPaint", true, |ui| {
        let label = if ed.paint.active { "Close PaintBox" } else { "🎨 Open PaintBox" };
        if action(ui, label, true, CONTENT_W) {
            ed.paint.mode = PaintMode::Vertex;
            ed.toggle_paint();
        }
        layer_controls(ui, ed);
        ui.label(dim("Per-vertex RGB layer: no UVs required."));
    });
}
