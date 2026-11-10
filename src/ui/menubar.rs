//! The 3ds Max menu bar: File · Edit · Tools · Views · Create · Modifiers ·
//! Rendering · Help. The action menus come from `actions::layout::MENUS`;
//! Create and Modifiers are generated from the Create tab and the catalog.

use super::action_ui::menu_entries;
use super::actions::layout::MENUS;
use super::theme;
use crate::editor::{CommandTab, Editor, Tool};
use crate::generators::WorldKind;
use crate::mech::MechKind;
use crate::modifiers::GeoMode;
use crate::modifiers::catalog::{self, Fit};
use crate::primitives::PrimitiveKind;
use crate::render::LightKind;
use crate::scene::camera::CameraKind;
use crate::splines::ShapeKind;
use bevy_egui::egui::{self, RichText};

pub const HEIGHT: f32 = 24.0;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) -> egui::Rect {
    egui::Panel::top("menu_bar")
        .resizable(false)
        .show_separator_line(false)
        .exact_size(HEIGHT)
        .frame(egui::Frame::new().fill(theme::INPUT).inner_margin(egui::Margin::symmetric(8, 2)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("MiniMax").color(theme::AMBER).strong());
                for (title, entries) in MENUS {
                    if *title == "Rendering" {
                        ui.menu_button("Create", |ui| create_menu(ui, ed));
                        ui.menu_button("Modifiers", |ui| modifiers_menu(ui, ed));
                    }
                    ui.menu_button(*title, |ui| menu_entries(ui, ed, entries));
                }
            });
        })
        .response
        .rect
}

/// Picks a Create tool (and shows the Create tab, like 3ds Max).
fn pick(ui: &mut egui::Ui, ed: &mut Editor, label: &str, tip: &str, tool: Tool) {
    let r = ui.button(label);
    let r = if tip.is_empty() { r } else { r.on_hover_text(tip) };
    if r.clicked() {
        ed.tool = tool;
        ed.iface.nav_mode = None;
        ed.tab = CommandTab::Create;
        ed.status = format!("{label}: click in a viewport to place it");
        ui.close();
    }
}

/// Every Create-tab object type, grouped as on the Create tab.
pub fn create_menu(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.menu_button("Standard Primitives", |ui| {
        for k in PrimitiveKind::ALL {
            pick(ui, ed, k.label(), "", Tool::Create(k));
        }
    });
    ui.menu_button("GeoGen", |ui| {
        for m in GeoMode::ALL {
            pick(ui, ed, m.label(), "", Tool::CreateGeoGen(m));
        }
    });
    ui.menu_button("Worlds", |ui| {
        for k in WorldKind::ALL {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Settlements (GhostTown)", |ui| {
        for k in WorldKind::SETTLEMENTS {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Mech", |ui| {
        for k in MechKind::ALL {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateMech(k));
        }
    });
    ui.separator();
    ui.menu_button("Shapes / Splines", |ui| {
        for k in ShapeKind::ALL {
            let tool = if k == ShapeKind::Line { Tool::Line { view: 0, knots: vec![], dragging: false } } else { Tool::CreateShape(k) };
            pick(ui, ed, k.label(), "", tool);
        }
    });
    ui.menu_button("Lights", |ui| {
        for k in LightKind::ALL {
            pick(ui, ed, k.label(), "", Tool::CreateLight { kind: k, first: None });
        }
    });
    ui.menu_button("Cameras", |ui| {
        for k in CameraKind::ALL {
            pick(ui, ed, k.label(), "", Tool::CreateCamera { kind: k, first: None });
        }
    });
}

/// Every modifier, grouped; ones that don't fit the selection are dimmed.
pub fn modifiers_menu(ui: &mut egui::Ui, ed: &mut Editor) {
    for (group, list) in catalog::GROUPS {
        ui.menu_button(*group, |ui| {
            for info in *list {
                if modifier_row(ui, ed, info.name, info.tip) {
                    ui.close();
                }
            }
        });
    }
}

/// One clickable modifier row; returns true when it added the modifier.
pub fn modifier_row(ui: &mut egui::Ui, ed: &mut Editor, name: &str, tip: &str) -> bool {
    let fit = catalog::fit(name, ed.current().map(|id| &ed.scene.objects[id]));
    let (text, enabled, hover) = match fit {
        Fit::Yes => (RichText::new(name), true, tip.to_string()),
        Fit::Dim(why) => (RichText::new(name).color(theme::TEXT_DIM), true, format!("{tip}\n{why}")),
        Fit::No(why) => (RichText::new(name).color(theme::TEXT_DIM), false, format!("{tip}\n{why}")),
    };
    let r = ui.add_enabled(enabled, egui::Button::new(text).frame(false)).on_hover_text(&hover).on_disabled_hover_text(&hover);
    if r.clicked() {
        ed.add_modifier(name);
        ed.tab = CommandTab::Modify;
        return true;
    }
    false
}
