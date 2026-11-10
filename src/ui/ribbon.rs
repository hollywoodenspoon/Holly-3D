//! Modeling Ribbon (toggle with the ▭ Ribbon button): big buttons for every
//! Edit Poly tool, grouped like 3ds Max's ribbon. The groups follow the
//! sub-object level (1–5); right-click a caddy tool to open its settings.

use super::action_ui::sized_button;
use super::actions::Action;
use super::theme;
use crate::editor::{Editor, Tool};
use crate::mesh::SubObject;
use crate::mesh::ops::{ExtrudeMode, PolyOp};
use bevy_egui::egui::{self, RichText};

const H: f32 = 30.0;

/// What a ribbon button does.
enum Cmd {
    Act(Action),
    /// Applies at once.
    Op(PolyOp),
    /// Applies the remembered settings; right-click opens the caddy.
    Caddy(PolyOp),
    /// Starts a click tool (pressing again leaves it).
    Pick(Tool),
    Grow,
    Shrink,
    SlicePlane,
}

type Group = (&'static str, Vec<(&'static str, Cmd)>);

fn groups(sub: SubObject) -> Vec<Group> {
    use Cmd::*;
    let mut g: Vec<Group> = vec![(
        "Polygon Modeling",
        vec![
            ("1 Vertex", Act(Action::Sub(SubObject::Vertex))),
            ("2 Edge", Act(Action::Sub(SubObject::Edge))),
            ("3 Border", Act(Action::Sub(SubObject::Border))),
            ("4 Polygon", Act(Action::Sub(SubObject::Polygon))),
            ("5 Element", Act(Action::Sub(SubObject::Element))),
            ("To Poly", Act(Action::ConvertToPoly)),
        ],
    )];
    g.push(("Selection", vec![("Grow", Grow), ("Shrink", Shrink), ("Ring", Act(Action::SelectRing)), ("Loop", Act(Action::SelectLoop))]));
    let level: Group = match sub {
        SubObject::Vertex => (
            "Vertices",
            vec![
                ("Remove", Op(PolyOp::RemoveVertices)),
                ("Break", Op(PolyOp::BreakVertices)),
                ("Extrude", Caddy(PolyOp::ExtrudeVertices { height: 0.2, width: 0.1 })),
                ("Weld", Caddy(PolyOp::Weld { level: SubObject::Vertex, threshold: 0.01 })),
                ("Chamfer", Caddy(PolyOp::ChamferVertices { amount: 0.1 })),
                ("Target Weld", Pick(Tool::TargetWeld { first: None })),
                ("Connect", Op(PolyOp::ConnectVertices)),
            ],
        ),
        SubObject::Edge | SubObject::Border => (
            if sub == SubObject::Border { "Borders" } else { "Edges" },
            vec![
                ("Insert Vertex", Pick(Tool::InsertVertex)),
                ("Remove", Act(Action::RemoveSubObjects)),
                ("Split", Op(PolyOp::SplitEdges)),
                ("Extrude", Caddy(PolyOp::ExtrudeEdges { height: 0.2, width: 0.1 })),
                ("Chamfer", Caddy(PolyOp::ChamferEdges { amount: 0.1, segments: 1, tension: 0.5, open: false })),
                ("Connect", Caddy(PolyOp::ConnectEdges { segments: 1, pinch: 0.0, slide: 0.0 })),
                ("Bridge", Caddy(PolyOp::BridgeBorders { segments: 1, taper: 0.0, twist: 0 })),
                ("Cap", Op(PolyOp::Cap)),
                ("Target Weld", Pick(Tool::TargetWeld { first: None })),
            ],
        ),
        SubObject::Polygon | SubObject::Element => (
            if sub == SubObject::Element { "Elements" } else { "Polygons" },
            vec![
                ("Insert Vertex", Pick(Tool::InsertVertex)),
                ("Extrude", Caddy(PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 0.25 })),
                ("Outline", Caddy(PolyOp::Outline { amount: -0.05 })),
                ("Bevel", Caddy(PolyOp::Bevel { mode: ExtrudeMode::Group, height: 0.2, outline: -0.05 })),
                ("Inset", Caddy(PolyOp::Inset { by_polygon: false, amount: 0.05 })),
                ("Bridge", Caddy(PolyOp::BridgePolygons { segments: 1, taper: 0.0, twist: 0 })),
                ("Flip", Op(PolyOp::Flip)),
                ("Hinge", Pick(Tool::PickHinge)),
                ("Along Spline", Pick(Tool::PickSpline)),
            ],
        ),
        SubObject::Object => ("Object", vec![("Isolate", Act(Action::Isolate)), ("Clone", Act(Action::Clone)), ("Align", Act(Action::Align))]),
    };
    g.push(level);
    if sub != SubObject::Object {
        g.push((
            "Geometry (All)",
            vec![
                ("Repeat Last", Act(Action::RepeatLast)),
                ("Create", Pick(Tool::CreatePoly { points: vec![] })),
                ("Collapse", Op(PolyOp::Collapse { level: sub })),
                ("Slice Plane", SlicePlane),
                ("QuickSlice", Pick(Tool::QuickSlice { start: None })),
                ("Cut", Pick(Tool::Cut { last: None })),
                ("MSmooth", Op(PolyOp::MSmooth { strength: 0.5 })),
                ("Tessellate", Caddy(PolyOp::Tessellate { face_center: false })),
                ("Make Planar", Op(PolyOp::MakePlanar { level: sub })),
            ],
        ));
    }
    g
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) -> egui::Rect {
    egui::Panel::top("modeling_ribbon")
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new().fill(theme::CARD).inner_margin(egui::Margin::symmetric(8, 4)))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let in_sub = ed.in_sub_object();
                for (title, cmds) in groups(ed.sub) {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(title).small().color(theme::AMBER));
                        ui.horizontal(|ui| {
                            for (label, cmd) in cmds {
                                cell(ui, ed, label, cmd, in_sub);
                            }
                        });
                    });
                    ui.separator();
                }
            });
        })
        .response
        .rect
}

fn cell(ui: &mut egui::Ui, ed: &mut Editor, label: &str, cmd: Cmd, in_sub: bool) {
    let w = (label.len() as f32 * 7.0 + 16.0).max(52.0);
    let size = egui::vec2(w, H);
    if let Cmd::Act(a) = cmd {
        sized_button(ui, ed, a, label, size);
        return;
    }
    let on = match &cmd {
        Cmd::Pick(t) => std::mem::discriminant(&ed.tool) == std::mem::discriminant(t),
        Cmd::SlicePlane => ed.slice_plane.is_some(),
        _ => false,
    };
    let enabled = in_sub || matches!(cmd, Cmd::Grow | Cmd::Shrink) && ed.current().is_some();
    let r = ui.add_enabled_ui(enabled, |ui| ui.add_sized(size, egui::Button::new(label).selected(on))).inner;
    let tip = if matches!(cmd, Cmd::Caddy(_)) { "Click applies the last settings; right-click opens the settings caddy" } else { "" };
    let r = if tip.is_empty() { r } else { r.on_hover_text(tip) };
    match cmd {
        Cmd::Op(op) if r.clicked() => {
            ed.apply_op(op);
        }
        Cmd::Caddy(op) => {
            if r.clicked() {
                let op = ed.remembered(op);
                ed.apply_op(op);
            } else if r.secondary_clicked() {
                let op = ed.remembered(op);
                ed.open_caddy(op);
            }
        }
        Cmd::Pick(tool) if r.clicked() => ed.tool = if on { Tool::None } else { tool },
        Cmd::Grow if r.clicked() => ed.edit_selection(|m, s| m.grow_selection(s)),
        Cmd::Shrink if r.clicked() => ed.edit_selection(|m, s| m.shrink_selection(s)),
        Cmd::SlicePlane if r.clicked() => ed.toggle_slice_plane(),
        _ => {}
    }
}
