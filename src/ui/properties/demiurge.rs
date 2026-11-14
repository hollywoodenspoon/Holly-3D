//! Demiurge tab: worlds (terrain, ocean, planet, smoke plume), whole
//! settlements, single buildings and props, and GeoGen rocks and crystals.

use super::content_w;
use crate::editor::{Editor, Tool};
use crate::generators::WorldKind;
use crate::modifiers::GeoMode;
use crate::ui::widgets::{dim, rollout};
use bevy::math::Vec3;
use bevy_egui::egui;

/// One grid of world kinds: click arms the tool, right-click places at the origin.
fn world_grid(ui: &mut egui::Ui, ed: &mut Editor, id: &str, kinds: &[WorldKind], cols: usize) {
    let w = content_w() / cols as f32 - 8.0;
    egui::Grid::new(id).num_columns(cols).spacing([6.0, 6.0]).show(ui, |ui| {
        for (i, &kind) in kinds.iter().enumerate() {
            let active = ed.tool == Tool::CreateWorld(kind);
            let r = ui.add_sized([w, 28.0], egui::Button::new(kind.label()).selected(active)).on_hover_text(kind.tip());
            if r.clicked() {
                ed.tool = if active { Tool::None } else { Tool::CreateWorld(kind) };
            }
            if r.secondary_clicked() {
                ed.create_world(kind, Vec3::ZERO);
            }
            if i % cols == cols - 1 {
                ui.end_row();
            }
        }
    });
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Worlds: Terrain, Ocean, Planet, Smoke Plume", true, |ui| {
        world_grid(ui, ed, "dm_worlds", &WorldKind::ALL, 2);
        ui.label(dim("Each world comes with its Multi/Sub-Object pack, so the generated Material IDs render right away."));
    });
    rollout(ui, "Clouds", true, |ui| {
        world_grid(ui, ed, "dm_clouds", &WorldKind::CLOUDS, 2);
        ui.label(dim("Drag to size a cloud, then drag up for its height. A Cloud Field fills a whole sky layer; Break Into Clouds splits it."));
    });
    rollout(ui, "Trails", true, |ui| {
        world_grid(ui, ed, "dm_trails", &WorldKind::TRAILS, 2);
        ui.label(dim("Drag to set a trail's length. The source sits at the pivot; Pick Path Spline makes it follow a spline."));
    });
    rollout(ui, "Settlements", true, |ui| {
        world_grid(ui, ed, "dm_settlements", &WorldKind::SETTLEMENTS, 2);
        ui.label(dim("Click, then click in a viewport to place a whole settlement with its Material Pack."));
    });
    crate::ui::buildings_browser::create_rollout(ui, ed);
    rollout(ui, "GeoGen: Rocks, Crystals, Stalagmites", true, |ui| {
        let w = content_w() / 2.0 - 8.0;
        egui::Grid::new("dm_geogen").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, mode) in GeoMode::ALL.iter().enumerate() {
                let active = ed.tool == Tool::CreateGeoGen(*mode);
                let r = ui.add_sized([w, 28.0], egui::Button::new(mode.short()).selected(active)).on_hover_text(mode.label());
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
}
