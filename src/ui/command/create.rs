//! Create tab: Geometry / Shapes / Lights & Cameras.

use super::CONTENT_W;
use crate::editor::{Editor, Tool};
use crate::generators::WorldKind;
use crate::mech::MechKind;
use crate::modifiers::GeoMode;
use crate::primitives::PrimitiveKind;
use crate::render::LightKind;
use crate::scene::camera::CameraKind;
use crate::splines::ShapeKind;
use crate::ui::widgets::{dim, rollout};
use bevy::math::Vec3;
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    creation_method(ui, ed);
    rollout(ui, "Geometry — Standard Primitives", true, |ui| {
        egui::Grid::new("prims").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, kind) in PrimitiveKind::ALL.iter().enumerate() {
                let active = ed.tool == Tool::Create(*kind);
                let r = ui.add_sized([CONTENT_W / 2.0 - 8.0, 28.0], egui::Button::new(kind.label()).selected(active));
                if r.clicked() {
                    ed.tool = if active { Tool::None } else { Tool::Create(*kind) };
                }
                if r.secondary_clicked() {
                    ed.create_primitive(*kind, Vec3::ZERO);
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        ui.label(dim("Pick a type, then drag in a viewport: the base first, then move and click for the height. A plain click places a default size; right-click cancels. Right-click a type to create at the origin."));
    });
    rollout(ui, "Geometry — GeoGen", true, |ui| {
        egui::Grid::new("geogen").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, mode) in GeoMode::ALL.iter().enumerate() {
                let active = ed.tool == Tool::CreateGeoGen(*mode);
                let r = ui
                    .add_sized([CONTENT_W / 2.0 - 8.0, 28.0], egui::Button::new(mode.short()).selected(active))
                    .on_hover_text(mode.label());
                if r.clicked() {
                    ed.tool = if active { Tool::None } else { Tool::CreateGeoGen(*mode) };
                }
                if r.secondary_clicked() {
                    ed.create_geogen(*mode, Vec3::ZERO);
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    });
    rollout(ui, "Geometry — Worlds", true, |ui| {
        ui.horizontal(|ui| {
            for kind in WorldKind::ALL {
                let active = ed.tool == Tool::CreateWorld(kind);
                let r =
                    ui.add_sized([CONTENT_W / 3.0 - 6.0, 28.0], egui::Button::new(kind.label()).selected(active)).on_hover_text(kind.tip());
                if r.clicked() {
                    ed.tool = if active { Tool::None } else { Tool::CreateWorld(kind) };
                }
                if r.secondary_clicked() {
                    ed.create_world(kind, Vec3::ZERO);
                }
            }
        });
        ui.label(dim("Each world comes with its Multi/Sub-Object pack, so the generated Material IDs render right away."));
    });
    rollout(ui, "Geometry — Settlement", true, |ui| {
        egui::Grid::new("create_settlement").num_columns(2).show(ui, |ui| {
            for (i, kind) in WorldKind::SETTLEMENTS.into_iter().enumerate() {
                let active = ed.tool == Tool::CreateWorld(kind);
                let r = ui.add_sized([CONTENT_W / 2.0 - 8.0, 28.0], egui::Button::new(kind.label()).selected(active)).on_hover_text(kind.tip());
                if r.clicked() {
                    ed.tool = if active { Tool::None } else { Tool::CreateWorld(kind) };
                }
                if r.secondary_clicked() {
                    ed.create_world(kind, Vec3::ZERO);
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        ui.label(dim("GhostTown: click to place a whole settlement with the GhostTown Material Pack."));
    });
    crate::ui::buildings_browser::create_rollout(ui, ed);
    crate::ui::character_studio::create_rollout(ui, ed);
    rollout(ui, "Geometry — Mech", true, |ui| {
        ui.horizontal(|ui| {
            for kind in MechKind::ALL {
                let active = ed.tool == Tool::CreateMech(kind);
                let r =
                    ui.add_sized([CONTENT_W / 3.0 - 6.0, 28.0], egui::Button::new(kind.label()).selected(active)).on_hover_text(kind.tip());
                if r.clicked() {
                    ed.tool = if active { Tool::None } else { Tool::CreateMech(kind) };
                }
                if r.secondary_clicked() {
                    ed.create_mech(kind, Vec3::ZERO);
                }
            }
        });
        ui.label(dim("Link gears with Motion Links (Assembly rollout); bind a hose's ends to two parts in its Modify panel."));
    });
    rollout(ui, "Shapes / Splines", true, |ui| {
        egui::Grid::new("shapes").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, kind) in ShapeKind::ALL.iter().enumerate() {
                let active = match &ed.tool {
                    Tool::CreateShape(k) => k == kind,
                    Tool::Line { .. } => *kind == ShapeKind::Line,
                    _ => false,
                };
                let r = ui.add_sized([CONTENT_W / 2.0 - 8.0, 28.0], egui::Button::new(kind.label()).selected(active));
                if r.clicked() {
                    ed.tool = match (active, kind) {
                        (true, _) => Tool::None,
                        (false, ShapeKind::Line) => Tool::Line { view: 0, knots: vec![], dragging: false },
                        (false, k) => Tool::CreateShape(*k),
                    };
                }
                if r.secondary_clicked() && *kind != ShapeKind::Line {
                    let view = ed.viewports.active;
                    ed.create_shape(*kind, view, Vec3::ZERO);
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        ui.label(dim(
            "Line: click for corners, drag for Bezier curves, right-click to finish. Add Lathe or Extrude from the Modifier List.",
        ));
    });
    rollout(ui, "💡 Lights", true, |ui| {
        egui::Grid::new("lights").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, kind) in LightKind::ALL.iter().enumerate() {
                let active = matches!(ed.tool, Tool::CreateLight { kind: k, .. } if k == *kind);
                let r = ui.add_sized([CONTENT_W / 2.0 - 8.0, 28.0], egui::Button::new(kind.label()).selected(active));
                if r.clicked() {
                    ed.tool = if active { Tool::None } else { Tool::CreateLight { kind: *kind, first: None } };
                }
                if r.secondary_clicked() {
                    ed.create_light_default(*kind);
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        ui.label(dim("Target lights: click the light, then click its target. Ctrl+L toggles Default Lights; 8 opens Environment."));
    });
    rollout(ui, "🎥 Cameras", true, |ui| {
        ui.horizontal(|ui| {
            for kind in CameraKind::ALL {
                let active = matches!(ed.tool, Tool::CreateCamera { kind: k, .. } if k == kind);
                let r = ui.add_sized([CONTENT_W / 2.0 - 8.0, 28.0], egui::Button::new(kind.label()).selected(active));
                if r.clicked() {
                    ed.tool = if active { Tool::None } else { Tool::CreateCamera { kind, first: None } };
                }
                if r.secondary_clicked() {
                    ed.create_camera(kind, Vec3::new(6.0, -8.0, 4.0), Vec3::new(0.0, 0.0, 0.5));
                }
            }
        });
        ui.label(dim("Target Camera: click the camera, then its target. Press C to look through the selected camera."));
    });
}

/// Creation Method (3ds Max): Box / Cube for the Box, Edge / Center for round objects.
fn creation_method(ui: &mut egui::Ui, ed: &mut Editor) {
    use crate::editor::create_drag::CreateKind;
    let Some(kind) = CreateKind::of(&ed.tool) else { return };
    let m = &mut ed.iface.create_method;
    rollout(ui, "Creation Method", true, |ui| {
        ui.horizontal(|ui| {
            if kind == CreateKind::Prim(PrimitiveKind::Box) {
                ui.radio_value(&mut m.cube, false, "Box").on_hover_text("Drag the base corner to corner, then move for the height");
                ui.radio_value(&mut m.cube, true, "Cube").on_hover_text("Drag out from the centre; all sides equal");
            } else if !kind.is_rect() {
                ui.radio_value(&mut m.edge, false, "Center").on_hover_text("Press at the centre, drag out the radius");
                ui.radio_value(&mut m.edge, true, "Edge").on_hover_text("Drag across the diameter, edge to edge");
            } else {
                ui.label(dim("Drag the area corner to corner."));
            }
        });
        ui.label(dim(kind.prompt(0)));
    });
}
