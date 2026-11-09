//! Create tab: Geometry / Shapes / Lights & Cameras.

use super::CONTENT_W;
use crate::editor::{Editor, Tool};
use crate::primitives::PrimitiveKind;
use crate::render::LightKind;
use crate::ui::widgets::{dim, rollout};
use bevy::math::Vec3;
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
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
            ui.add_enabled_ui(false, |ui| {
                ui.add_sized([CONTENT_W / 2.0 - 8.0, 28.0], egui::Button::new("GeoGen Rock/Crystal"))
                    .on_disabled_hover_text("Arrives in Phase 5");
            });
            ui.end_row();
        });
        ui.label(dim("Click a type, then click in a viewport to place it. Right-click a type to create at the origin."));
    });
    rollout(ui, "Shapes / Splines", false, |ui| {
        ui.add_enabled_ui(false, |ui| {
            ui.horizontal_wrapped(|ui| {
                for s in ["Line", "Circle", "Rectangle", "Star", "NGon"] {
                    ui.button(s).on_disabled_hover_text("Arrives in Phase 5");
                }
            });
        });
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
    rollout(ui, "Cameras", false, |ui| {
        ui.add_enabled_ui(false, |ui| {
            ui.button("Target Camera").on_disabled_hover_text("Arrives with Render Setup (F10)");
        });
    });
}
