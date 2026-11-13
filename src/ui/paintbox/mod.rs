//! PaintBox floater (3ds Max VertexPaint palette) for Viewport Paint, plus
//! the VertexPaint modifier rollout in the Modify panel. Texture Paint
//! adds the layer list, damage stamps, mesh map baking and Smart Masks.

mod bake;
mod brush;
mod layers;

use crate::editor::Editor;
use crate::modifiers::ModifierKind;
use crate::paint::PaintMode;
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, action, dim, max_spinner_range, rollout, toggle_button};
use bevy_egui::egui::{self, RichText};

/// Width of full-width PaintBox buttons.
pub(super) const W: f32 = 300.0;

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

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.paint.active {
        return;
    }
    let mut open = true;
    let win = egui::Window::new(RichText::new("PaintBox").strong())
        .id(egui::Id::new("paint_box"))
        .default_pos([260.0, 90.0])
        .default_width(W + 16.0)
        .resizable(false);
    crate::ui::popout::window(ctx, crate::ui::popout::Dock::PaintBox, win, &mut open, |ui| {
            let h = (ctx.content_rect().height() - 160.0).max(200.0);
            egui::ScrollArea::vertical().max_height(h).min_scrolled_height(h).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (m, label) in [(PaintMode::Vertex, "Vertex Paint"), (PaintMode::Texture, "Texture Paint")] {
                    if toggle_button(ui, ed.paint.mode == m, label).clicked() {
                        ed.paint.mode = m;
                    }
                }
            });
            ui.separator();
            ui.label(RichText::new("Brush").strong());
            brush::show(ui, ed);
            ui.separator();
            match ed.paint.mode {
                PaintMode::Vertex => {
                    ui.label(RichText::new("VertexPaint Layer").strong());
                    layer_controls(ui, ed);
                }
                PaintMode::Texture => {
                    ui.label(RichText::new("Layers").strong());
                    layers::show(ui, ed);
                    ui.separator();
                    ui.label(RichText::new("Mesh Maps & Smart Masks").strong());
                    bake::show(ui, ed);
                }
            }
            ui.separator();
            let target = ed.current_obj().map_or("nothing selected".to_string(), |o| o.name.clone());
            ui.label(dim(&format!("Painting: {target} · right-click a viewport to exit")));
            });
        });
    if !open {
        ed.toggle_paint();
    }
}

/// Modify panel rollout when a VertexPaint modifier is highlighted.
pub fn modifier_rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "VertexPaint", true, |ui| {
        let label = if ed.paint.active { "Close PaintBox" } else { "🎨 Open PaintBox" };
        if action(ui, label, true, content_w()) {
            ed.paint.mode = PaintMode::Vertex;
            ed.toggle_paint();
        }
        layer_controls(ui, ed);
        ui.label(dim("Per-vertex RGB layer: no UVs required."));
    });
}
