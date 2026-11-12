//! PR 6 GhostTown toggles: the preset's Buildings & Props checkboxes,
//! building parts, town features, Emissive Lights and Prop Style.

use crate::generators::buildings::BuildingKind;
use crate::generators::{PropStyle, SettlementParams};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui::{self, RichText};

/// Two-column checkbox grid.
fn checks<'a>(ui: &mut egui::Ui, t: &mut Track, id: &str, flags: impl IntoIterator<Item = (&'static str, &'a mut bool)>) {
    egui::Grid::new(id).num_columns(2).show(ui, |ui| {
        for (i, (label, on)) in flags.into_iter().enumerate() {
            t.r(ui.checkbox(on, label));
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}

pub fn toggles(ui: &mut egui::Ui, t: &mut Track, p: &mut SettlementParams) {
    let palette = p.era.palette();
    if !palette.is_empty() {
        ui.collapsing(RichText::new("Buildings").strong(), |ui| {
            ui.label(dim("Unchecked types are left out of this preset."));
            let mut on: Vec<(&'static str, bool, BuildingKind)> = palette.iter().map(|k| (k.label(), !p.skip.contains(k), *k)).collect();
            let before: Vec<bool> = on.iter().map(|o| o.1).collect();
            checks(ui, t, "town_buildings", on.iter_mut().map(|(l, b, _)| (*l, b)));
            if on.iter().map(|o| o.1).ne(before) {
                p.skip = on.iter().filter(|o| !o.1).map(|o| o.2).collect();
            }
        });
        ui.collapsing(RichText::new("Building Parts").strong(), |ui| checks(ui, t, "town_parts", p.parts.flags()));
    }
    ui.collapsing(RichText::new("Town Features").strong(), |ui| {
        ui.label(dim("Each preset uses the features it has."));
        checks(ui, t, "town_features", p.features.flags());
    });
    t.r(ui.checkbox(&mut p.emissive, "Emissive Lights (lit windows, lamps, neon)"));
    ui.horizontal(|ui| {
        ui.label("Prop Style:");
        egui::ComboBox::from_id_salt("town_props").width(CONTENT_W * 0.55).selected_text(p.prop_style.label()).show_ui(ui, |ui| {
            for s in PropStyle::ALL {
                t.r(ui.selectable_value(&mut p.prop_style, s, s.label()));
            }
        });
    });
}
