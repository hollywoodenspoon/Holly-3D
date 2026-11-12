//! Phase 10A Character & Creature Studio window (Tools menu, toolbar
//! `♀♂ Characters`, Create tab): one-click archetype and creature tiles,
//! then the parameters of the selected character.

pub mod params;

use crate::character::params::{Archetype, Creature};
use crate::editor::Editor;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{dim, rollout};
use bevy::math::Vec3;
use bevy_egui::egui;

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.iface.character_studio_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Character & Creature Studio").open(&mut open).default_size([380.0, 620.0]).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            gallery(ui, ed, 2);
            ui.separator();
            if ed.character().is_some() {
                params::base(ui, ed);
            } else {
                ui.label(dim("Select a character to edit its body, hair, outfit and weapon here."));
            }
        });
    });
    ed.iface.character_studio_open &= open;
}

/// Archetype + creature tile grids (`cols` per row).
pub fn gallery(ui: &mut egui::Ui, ed: &mut Editor, cols: usize) {
    let w = CONTENT_W / cols as f32 - 6.0;
    ui.label(egui::RichText::new("Humanoids (MakeHuman hm08, CC0)").strong());
    egui::Grid::new("char_archetypes").num_columns(cols).show(ui, |ui| {
        for (i, &a) in Archetype::ALL.iter().enumerate() {
            if ui.add_sized([w, 26.0], egui::Button::new(a.label())).on_hover_text("Place at the origin").clicked() {
                ed.create_archetype(a, Vec3::ZERO);
            }
            if i % cols == cols - 1 {
                ui.end_row();
            }
        }
    });
    ui.label(egui::RichText::new("Creatures").strong());
    egui::Grid::new("char_creatures").num_columns(cols).show(ui, |ui| {
        for (i, &c) in Creature::ALL.iter().skip(1).enumerate() {
            if ui.add_sized([w, 26.0], egui::Button::new(c.label())).clicked() {
                ed.create_creature(c, Vec3::ZERO);
            }
            if i % cols == cols - 1 {
                ui.end_row();
            }
        }
    });
}

/// Create tab rollout.
pub fn create_rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Geometry — Characters & Creatures", true, |ui| {
        gallery(ui, ed, 2);
        if ui.add_sized([CONTENT_W, 26.0], egui::Button::new("Open Character & Creature Studio...").selected(ed.iface.character_studio_open)).clicked() {
            ed.iface.character_studio_open ^= true;
        }
    });
}
