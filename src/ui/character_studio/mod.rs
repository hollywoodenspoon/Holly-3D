//! Phase 10A Character & Creature Studio window (Tools menu, toolbar
//! `♀♂ Characters`, Create tab): one-click archetype and creature tiles,
//! then the parameters of the selected character.

pub mod params;
pub mod pose;
pub mod wardrobe;

use crate::character::params::{Archetype, Creature};
use crate::editor::characters::CharacterPick;
use crate::editor::{Editor, Tool};
use crate::ui::properties::content_w;
use crate::ui::widgets::{dim, rollout};
use bevy::math::Vec3;
use bevy_egui::egui;

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.iface.character_studio_open {
        return;
    }
    let mut open = true;
    let win = egui::Window::new("Character & Creature Studio").default_size([380.0, 620.0]);
    crate::ui::popout::window(ctx, crate::ui::popout::Dock::CharacterStudio, win, &mut open, |ui| {
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

/// One tile: arms the Create Character tool (click or drag in a viewport).
fn tile(ui: &mut egui::Ui, ed: &mut Editor, w: f32, pick: CharacterPick) {
    let active = ed.tool == Tool::CreateCharacter(pick);
    let r = ui.add_sized([w, 26.0], egui::Button::new(pick.label()).selected(active));
    if r.on_hover_text("Click, then click in a viewport to place it, or drag to set its height").clicked() {
        ed.tool = if active { Tool::None } else { Tool::CreateCharacter(pick) };
        if !active {
            ed.status = format!("{}: click to place, or drag to set the height", pick.label());
        }
    }
}

/// Archetype + creature tile grids (`cols` per row).
pub fn gallery(ui: &mut egui::Ui, ed: &mut Editor, cols: usize) {
    let w = content_w() / cols as f32 - 6.0;
    ui.label(egui::RichText::new("Humanoids (MakeHuman hm08, CC0)").strong());
    egui::Grid::new("char_archetypes").num_columns(cols).show(ui, |ui| {
        for (i, &a) in Archetype::ALL.iter().enumerate() {
            tile(ui, ed, w, CharacterPick::Archetype(a));
            if i % cols == cols - 1 {
                ui.end_row();
            }
        }
    });
    ui.label(egui::RichText::new("Creatures").strong());
    egui::Grid::new("char_creatures").num_columns(cols).show(ui, |ui| {
        for (i, &c) in Creature::ALL.iter().skip(1).enumerate() {
            tile(ui, ed, w, CharacterPick::Creature(c));
            if i % cols == cols - 1 {
                ui.end_row();
            }
        }
    });
    let armed = match ed.tool {
        Tool::CreateCharacter(p) => p,
        _ => CharacterPick::Archetype(Archetype::Human),
    };
    if ui.add_sized([content_w(), 24.0], egui::Button::new(format!("Place {} at Origin", armed.label()))).clicked() {
        ed.create_pick(armed, Vec3::ZERO);
    }
}

/// Create tab rollout.
pub fn create_rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Geometry — Characters & Creatures", true, |ui| {
        gallery(ui, ed, 2);
        if ui.add_sized([content_w(), 26.0], egui::Button::new("Open Character & Creature Studio...").selected(ed.iface.character_studio_open)).clicked() {
            ed.iface.character_studio_open ^= true;
        }
    });
}
