//! PR 7 Destruction rollout, shared by Buildings & Props, the GhostTown
//! settlement and the Battlefield / Ruin modifier: destruction level,
//! era, rubble density, `[✓] Glowing Embers` + Ember Glow and
//! `[✓] Gore / Blood Smears` + Amount.

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::ruin::{DestructionLevel, RuinEra, RuinParams};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, action, dim};
use bevy_egui::egui::{self, RichText};

/// What the caller should do after the rollout.
#[derive(Default)]
pub struct Out {
    /// Set the object's ID 25 emission to this (0 = embers off).
    pub glow: Option<f32>,
}

/// The rollout body. `pile` shows the Rubble Pile's own parameters
/// (Pile Height) instead of the destruction level.
pub fn destruction(ui: &mut egui::Ui, t: &mut Track, p: &mut RuinParams, salt: &str, pile: bool) -> Out {
    let before = (p.embers, p.ember_glow);
    let mut out = Out::default();
    if pile {
        ui.label(RichText::new("Rubble Pile").strong());
        ui.label(dim("Width / Length set the spread; the mound is layered dust, chunks and the era's debris."));
    } else {
        ui.label(RichText::new("Destruction Level").strong());
        egui::ComboBox::from_id_salt(("ruin_level", salt)).width(CONTENT_W - 30.0).selected_text(p.level.label()).show_ui(ui, |ui| {
            for l in DestructionLevel::ALL {
                t.r(ui.selectable_value(&mut p.level, l, l.label()));
            }
        });
        ui.label(dim(p.level.hint()));
        if p.level == DestructionLevel::Intact {
            return out;
        }
    }
    ui.horizontal(|ui| {
        ui.label("Era:");
        egui::ComboBox::from_id_salt(("ruin_era", salt)).width(CONTENT_W - 70.0).selected_text(p.era.label()).show_ui(ui, |ui| {
            for e in RuinEra::ALL {
                t.r(ui.selectable_value(&mut p.era, e, e.label()));
            }
        });
    });
    seed_row(ui, t, &mut p.seed);
    grid(ui, &format!("ruin_params_{salt}"), |ui| {
        row(ui, t, "Rubble Density:", &mut p.rubble, 0.0..=1.0, 0.6, 0.005);
        if pile {
            row(ui, t, "Pile Height:", &mut p.pile_height, 0.1..=50.0, 3.0, 0.02);
        }
    });
    t.r(ui.checkbox(&mut p.embers, "Glowing Embers")).on_hover_text("Embers (ID 25) smoulder in the rubble crevices");
    if p.embers {
        ui.horizontal(|ui| {
            ui.label("Ember Glow:");
            t.r(crate::ui::widgets::max_spinner_range(ui, &mut p.ember_glow, 0.05, crate::materials::ghost_pack::EMBER_GLOW, 0.0..=50.0));
            if action(ui, "Apply Glow", true, 90.0) {
                out.glow = Some(p.ember_glow);
            }
        });
    }
    t.r(ui.checkbox(&mut p.gore, "Gore / Blood Smears")).on_hover_text("Blood pools and drag smears (ID 28) on the rubble and the ground");
    if p.gore {
        ui.horizontal(|ui| {
            ui.label("Amount:");
            t.r(ui.add(egui::Slider::new(&mut p.gore_amount, 0.0..=100.0).suffix(" %")));
        });
    }
    if before != (p.embers, p.ember_glow) {
        out.glow = Some(if p.embers { p.ember_glow } else { 0.0 });
    }
    out
}
