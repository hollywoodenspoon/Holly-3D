//! The generated header menus: Create (every object type, as on the Create,
//! Demiurge and Characters tabs) and Modifiers (the Modifier List catalog).
//! The other menus come from `actions::layout::MENUS` (see `header`).

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

/// Picks a Create tool (and shows the Create tab).
fn pick(ui: &mut egui::Ui, ed: &mut Editor, label: &str, tip: &str, tool: Tool) {
    let r = ui.button(label);
    let r = if tip.is_empty() { r } else { r.on_hover_text(tip) };
    if r.clicked() {
        ed.tool = tool;
        ed.iface.nav_mode = None;
        ed.tab = CommandTab::Create;
        ed.status = format!("{label}: drag in a viewport to size it, or click for a default one");
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
    ui.menu_button("Demiurge Worlds", |ui| {
        for k in WorldKind::ALL {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Demiurge Clouds", |ui| {
        for k in WorldKind::CLOUDS {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Demiurge Trails", |ui| {
        for k in WorldKind::TRAILS {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Demiurge Simulations", |ui| {
        for k in WorldKind::SIMS {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Demiurge Explosions", |ui| {
        for k in WorldKind::PYRO {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Demiurge Settlements", |ui| {
        for k in WorldKind::SETTLEMENTS {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
        }
    });
    ui.menu_button("Demiurge Buildings & Props", |ui| {
        if ui.button("Prefab Library...").clicked() {
            ed.buildings_browser_open = true;
            ui.close();
        }
        ui.separator();
        for c in crate::generators::buildings::Category::ALL {
            ui.menu_button(c.label(), |ui| {
                for k in c.kinds() {
                    let k = WorldKind::Building(k);
                    pick(ui, ed, k.label(), k.tip(), Tool::CreateWorld(k));
                }
            });
        }
    });
    ui.menu_button("Mech", |ui| {
        for k in MechKind::ALL {
            pick(ui, ed, k.label(), k.tip(), Tool::CreateMech(k));
        }
        if ui.button("Spider Platform: Railgun").clicked() {
            ed.create_spider_railgun(bevy::math::Vec3::ZERO);
            ui.close();
        }
        if ui.button("Rig Selection as Spider Platform").clicked() {
            ed.rig_selected_spider();
            ui.close();
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
        Fit::Dim(why) => (RichText::new(name).color(theme::text_dim()), true, format!("{tip}\n{why}")),
        Fit::No(why) => (RichText::new(name).color(theme::text_dim()), false, format!("{tip}\n{why}")),
    };
    let r = ui.add_enabled(enabled, egui::Button::new(text).frame(false)).on_hover_text(&hover).on_disabled_hover_text(&hover);
    if r.clicked() {
        ed.add_modifier(name);
        ed.tab = CommandTab::Modify;
        return true;
    }
    false
}
