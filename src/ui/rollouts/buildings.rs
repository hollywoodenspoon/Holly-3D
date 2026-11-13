//! PR 6 rollouts: Buildings & Props parameters (Modify tab of a building /
//! prop base object).

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::buildings::{BuildingKind, BuildingParams, Category, Detail, MatStyle, StairKind};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui::{self, RichText};

/// Building Parameters. Returns the new material style when the user
/// switched it (the caller reassigns the matching pack) and the Destruction
/// rollout's ember-glow request.
pub fn params(ui: &mut egui::Ui, t: &mut Track, p: &mut BuildingParams) -> (Option<MatStyle>, super::ruin::Out) {
    ui.label(RichText::new("Building / Prop Type").strong());
    kind_combo(ui, t, p);
    ui.label(dim(p.kind.category().label()));
    seed_row(ui, t, &mut p.seed);
    grid(ui, "building_params", |ui| {
        row(ui, t, "Width:", &mut p.width, 0.2..=400.0, p.kind.defaults().0, 0.05);
        row(ui, t, "Length:", &mut p.length, 0.2..=400.0, p.kind.defaults().1, 0.05);
        if p.kind.has_floors() {
            let label = match p.kind {
                BuildingKind::ArchBridge => "Arches:",
                BuildingKind::Crane => "Boom Extra:",
                BuildingKind::SandbagWall => "Courses:",
                _ => "Floors:",
            };
            row(ui, t, label, &mut p.floors, 1..=200, p.kind.defaults().2, 0.05);
        }
    });
    t.r(ui.checkbox(&mut p.emissive, "Emissive Lights"))
        .on_hover_text("Lit windows, lamps, lanterns, neon signs and forge fires glow; off = dark glass");
    ui.horizontal(|ui| {
        ui.label("Detail:");
        for d in Detail::ALL {
            let tip = match d {
                Detail::Standard => "Light settlement look: flat window panes, few parts",
                Detail::Mid => "Window frames, sills and mullions, trims and extra small parts",
            };
            if ui.selectable_label(p.detail == d, d.label()).on_hover_text(tip).clicked() && p.detail != d {
                p.detail = d;
                (t.started, t.changed) = (true, true);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Stairs:");
        egui::ComboBox::from_id_salt("building_stairs").selected_text(p.stairs.label()).show_ui(ui, |ui| {
            for s in StairKind::ALL {
                if ui.selectable_label(p.stairs == s, s.label()).clicked() && p.stairs != s {
                    p.stairs = s;
                    (t.started, t.changed) = (true, true);
                }
            }
        });
    });
    ui.label(RichText::new("Building Parts").strong());
    egui::Grid::new("building_parts").num_columns(2).show(ui, |ui| {
        for (i, (label, on)) in p.parts.flags().into_iter().enumerate() {
            t.r(ui.checkbox(on, label));
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ui.label(dim("Parts only appear on the types that have them (a kiosk has no fire escape)."));
    if p.kind.category() == Category::Residential {
        ui.label(RichText::new("Lot").strong());
        egui::Grid::new("building_lot").num_columns(2).show(ui, |ui| {
            for (i, (label, on)) in p.lot.flags().into_iter().enumerate() {
                t.r(ui.checkbox(on, label));
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        ui.label(dim("Garden, driveway, garage and fence around detached houses."));
    }
    ui.separator();
    let pile = p.kind == BuildingKind::RubblePile;
    let out = super::ruin::destruction(ui, t, &mut p.ruin, "building", pile);
    ui.separator();
    let mut style = None;
    ui.horizontal(|ui| {
        ui.label("Material Style:");
        for s in MatStyle::ALL {
            if ui.selectable_label(p.style == s, s.label()).on_hover_text(style_tip(s)).clicked() && p.style != s {
                p.style = s;
                style = Some(s);
                (t.started, t.changed) = (true, true);
            }
        }
    });
    (style, out)
}

fn style_tip(s: MatStyle) -> &'static str {
    match s {
        MatStyle::Procedural => "Settlement pack built from MaterialCraft procedural textures (no downloads)",
        MatStyle::Photo => "Settlement pack using installed Poly Haven photo textures where available",
    }
}

/// Type combo grouped by category; switching type resets size to its
/// defaults and keeps seed, parts, lights and style.
fn kind_combo(ui: &mut egui::Ui, t: &mut Track, p: &mut BuildingParams) {
    egui::ComboBox::from_id_salt("building_kind").width(content_w() - 12.0).height(420.0).selected_text(p.kind.label()).show_ui(ui, |ui| {
        for c in Category::ALL {
            ui.label(RichText::new(c.label()).strong());
            for k in c.kinds() {
                if ui.selectable_label(p.kind == k, format!("   {}", k.label())).clicked() && p.kind != k {
                    *p = BuildingParams {
                        seed: p.seed,
                        parts: p.parts,
                        emissive: p.emissive,
                        style: p.style,
                        stairs: p.stairs,
                        ruin: p.ruin.clone(),
                        detail: p.detail,
                        lot: p.lot,
                        ..BuildingParams::of(k)
                    };
                    (t.started, t.changed) = (true, true);
                }
            }
        }
    });
}
