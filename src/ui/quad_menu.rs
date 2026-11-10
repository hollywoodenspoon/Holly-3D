//! Right-click Quad Menu (4 quadrants around the cursor).

use super::theme;
use crate::editor::{Editor, Tool, TransformMode};
use crate::mesh::SubObject;
use crate::mesh::ops::{ExtrudeMode, PolyOp};
use crate::scene::CloneKind;
use bevy_egui::egui::{self, RichText};

#[derive(Clone, Copy)]
enum Item {
    Op(&'static str),
    Move,
    Rotate,
    Scale,
    Clone,
    Align,
    Hide,
    UnhideAll,
    Freeze,
    Isolate,
    ToPoly,
    ToSpline,
}

const TOOLS: [&str; 8] = ["Extrude", "Chamfer", "Bevel", "Inset", "Connect", "Target Weld", "Cut", "QuickSlice"];

fn quadrant(ui: &mut egui::Ui, title: &str, items: &[(&str, Item)], picked: &mut Option<Item>) {
    egui::Frame::new().fill(theme::CARD).stroke(egui::Stroke::new(1.0, theme::BORDER)).corner_radius(4).inner_margin(6).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.set_width(170.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(title).color(theme::AMBER).small().strong());
            for (label, item) in items {
                let w = ui.available_width();
                if ui.add_sized([w, 20.0], egui::Button::new(*label).frame(false)).clicked() {
                    *picked = Some(*item);
                }
            }
        });
    });
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    let Some(pos) = ed.quad_menu else { return };
    let mut picked = None;
    let tools: Vec<(&str, Item)> = TOOLS.iter().map(|t| (*t, Item::Op(t))).collect();
    let area = egui::Area::new(egui::Id::new("quad_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(pos.x - 190.0, pos.y - 110.0))
        .show(ctx, |ui| {
            egui::Grid::new("quad").spacing([8.0, 8.0]).show(ui, |ui| {
                quadrant(ui, "tools 1", &tools, &mut picked);
                quadrant(
                    ui,
                    "display",
                    &[
                        ("Hide Selected", Item::Hide),
                        ("Unhide All", Item::UnhideAll),
                        ("Freeze Selected", Item::Freeze),
                        ("Isolate Selection", Item::Isolate),
                    ],
                    &mut picked,
                );
                ui.end_row();
                quadrant(
                    ui,
                    "convert to",
                    &[("Convert to Editable Poly", Item::ToPoly), ("Convert to Editable Spline", Item::ToSpline)],
                    &mut picked,
                );
                quadrant(
                    ui,
                    "transform",
                    &[
                        ("Move", Item::Move),
                        ("Rotate", Item::Rotate),
                        ("Scale", Item::Scale),
                        ("Clone", Item::Clone),
                        ("Align", Item::Align),
                    ],
                    &mut picked,
                );
                ui.end_row();
            });
        });
    let clicked_outside = ctx.input(|i| i.pointer.primary_pressed()) && !area.response.contains_pointer();
    if clicked_outside {
        ed.quad_menu = None;
    }
    let Some(item) = picked else { return };
    ed.quad_menu = None;
    match item {
        Item::Move => ed.mode = TransformMode::Move,
        Item::Rotate => ed.mode = TransformMode::Rotate,
        Item::Scale => ed.mode = TransformMode::Scale,
        Item::Clone => ed.clone_selected(CloneKind::Copy),
        Item::Align => ed.start_align(),
        Item::ToSpline => ed.convert_to_editable_spline(),
        Item::Hide => ed.hide_selected(),
        Item::UnhideAll => ed.unhide_all(),
        Item::Freeze => ed.freeze_selected(),
        Item::Isolate => ed.toggle_isolate(),
        Item::ToPoly => ed.convert_to_editable_poly(),
        Item::Op(name) => tool_op(ed, name),
    }
}

fn tool_op(ed: &mut Editor, name: &str) {
    let sub = ed.sub;
    let op = match (name, sub) {
        ("Extrude", SubObject::Vertex) => PolyOp::ExtrudeVertices { height: 0.2, width: 0.1 },
        ("Extrude", s) if s.is_edge_like() => PolyOp::ExtrudeEdges { height: 0.2, width: 0.1 },
        ("Extrude", _) => PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 0.25 },
        ("Chamfer", SubObject::Vertex) => PolyOp::ChamferVertices { amount: 0.1 },
        ("Chamfer", _) => PolyOp::ChamferEdges { amount: 0.1, segments: 1, tension: 0.5, open: false },
        ("Bevel", _) => PolyOp::Bevel { mode: ExtrudeMode::Group, height: 0.2, outline: -0.05 },
        ("Inset", _) => PolyOp::Inset { by_polygon: false, amount: 0.05 },
        ("Connect", SubObject::Vertex) => PolyOp::ConnectVertices,
        ("Connect", _) => PolyOp::ConnectEdges { segments: 1, pinch: 0.0, slide: 0.0 },
        ("Target Weld", _) => return ed.tool = Tool::TargetWeld { first: None },
        ("Cut", _) => return ed.tool = Tool::Cut { last: None },
        _ => return ed.tool = Tool::QuickSlice { start: None },
    };
    if !ed.in_sub_object() {
        ed.status = "Enter a sub-object mode (1–5) first".into();
        return;
    }
    let op = ed.remembered(op);
    ed.open_caddy(op);
}
