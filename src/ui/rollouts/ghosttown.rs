//! Phase 8C rollouts: GhostTown settlement parameters (base object and
//! modifier), the Medieval Architect, and Battlefield / Ruin (standalone,
//! and nested under each `[✓] Damaged / Battlefield Mode` toggle).

use super::modifiers::sub_buttons;
use super::spline::{grid, mod_params};
use super::worlds::{row, seed_row};
use crate::editor::Editor;
use crate::generators::{Era, SettlementParams};
use crate::materials::MultiPreset;
use crate::modifiers::battlefield::{BattleParams, DestructionMode};
use crate::modifiers::medieval::{ArchPreset, ArchitectMod, PlanShape};
use crate::modifiers::{ModSub, ModifierKind};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, action, dim, rollout};
use bevy_egui::egui::{self, RichText};

/// `[ Assign GhostTown Material Pack ]` + the ID legend.
pub fn pack_button(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.horizontal(|ui| {
        if action(ui, "Assign GhostTown Pack", true, CONTENT_W * 0.5 - 4.0) {
            ed.assign_pack(MultiPreset::GhostTown);
        }
        if action(ui, "Assign Photo Pack", true, CONTENT_W * 0.5 - 4.0) {
            ed.assign_pack(MultiPreset::GhostTownPhoto);
        }
    });
    ui.collapsing("Material ID legend (29 IDs)", |ui| {
        ui.label(dim("1 Road · 2 Lot/Dirt · 3 Facade/Stone · 4 Timber/Trim · 5 Roof · 6 Glass · 7 Lit Windows · 8 Foliage · 9 Charred · 10 Iron · 11 Plaster · 12 Rebar"));
        ui.label(dim("13 Asphalt · 14 Lane Lines · 15 Sidewalk Slabs · 16 Curbs · 17 Cobbles · 18 River Water · 19 Brick · 20 Stucco · 21 Slate/Metal Roof · 22 Rusted Metal · 23 Broken Glass · 24 Neon/Lantern Glow"));
        ui.label(dim("25 Glowing Embers · 26 Bullet Scars / Soot · 27 Rubble Dust / Ash · 28 Blood Smears · 29 Battlefield Smoke"));
    });
}

/// Applies a Destruction rollout's ember-glow request to the current object.
fn glow(ed: &mut Editor, out: super::ruin::Out) {
    if let (Some(g), Some(id)) = (out.glow, ed.current()) {
        ed.set_ember_glow(id, g);
    }
}

/// Settlement Parameters (`sized` shows Area Width / Length; the modifier
/// takes them from its input).
pub fn settlement_params(ui: &mut egui::Ui, t: &mut Track, p: &mut SettlementParams, sized: bool) -> super::ruin::Out {
    ui.label(RichText::new("Settlement Preset").strong());
    egui::ComboBox::from_id_salt("ghosttown_era").width(CONTENT_W - 12.0).selected_text(p.era.label()).show_ui(ui, |ui| {
        for e in Era::ALL {
            if ui.selectable_label(p.era == e, e.label()).clicked() && p.era != e {
                // A new era starts from its defaults, keeping area and seed.
                *p = SettlementParams { width: p.width, length: p.length, seed: p.seed, ..SettlementParams::of(e) };
                (t.started, t.changed) = (true, true);
            }
        }
    });
    seed_row(ui, t, &mut p.seed);
    grid(ui, "ghosttown_params", |ui| {
        if sized {
            row(ui, t, "Area Width:", &mut p.width, 10.0..=2_000.0, 160.0, 0.5);
            row(ui, t, "Area Length:", &mut p.length, 10.0..=2_000.0, 160.0, 0.5);
        }
        row(ui, t, "Block Size:", &mut p.block_size, 4.0..=200.0, 40.0, 0.2);
        row(ui, t, "Street Width:", &mut p.street_width, 1.0..=40.0, 10.0, 0.05);
        row(ui, t, "Sidewalk Width:", &mut p.sidewalk_width, 0.0..=10.0, 2.5, 0.02);
        row(ui, t, "Building Density:", &mut p.density, 0.0..=1.0, 0.85, 0.005);
        row(ui, t, "Min Floors:", &mut p.min_floors, 1..=80, 4, 0.05);
        row(ui, t, "Max Floors:", &mut p.max_floors, 1..=120, 30, 0.05);
        row(ui, t, "Roof Pitch:", &mut p.roof_pitch, 0.0..=70.0, 35.0, 0.2);
        row(ui, t, "Tree / Prop Density:", &mut p.prop_density, 0.0..=1.0, 0.5, 0.005);
    });
    p.max_floors = p.max_floors.max(p.min_floors);
    super::town_toggles::toggles(ui, t, p);
    ui.separator();
    let out = super::ruin::destruction(ui, t, &mut p.ruin, "ghosttown", false);
    ui.label(dim("Each building is wrecked one level either side of this, seeded per building."));
    ui.separator();
    damaged(ui, t, &mut p.damaged, &mut p.damage, "ghosttown");
    out
}

/// `[✓] Damaged / Battlefield Mode` with its parameters.
fn damaged(ui: &mut egui::Ui, t: &mut Track, on: &mut bool, d: &mut BattleParams, salt: &str) {
    t.r(ui.checkbox(on, "Damaged / Battlefield Mode"));
    if *on {
        ui.indent(salt, |ui| battle_params(ui, t, d, salt));
    }
}

/// Destruction mode and spinners (shared by the modifier and the toggles).
pub fn battle_params(ui: &mut egui::Ui, t: &mut Track, p: &mut BattleParams, salt: &str) {
    egui::ComboBox::from_id_salt(("battle_mode", salt)).width(CONTENT_W - 30.0).selected_text(p.mode.label()).show_ui(ui, |ui| {
        for m in DestructionMode::ALL {
            if ui.selectable_label(p.mode == m, m.label()).clicked() && p.mode != m {
                *p = BattleParams { seed: p.seed, ..BattleParams::of(m) };
                (t.started, t.changed) = (true, true);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Intensity:");
        t.r(ui.add(egui::Slider::new(&mut p.intensity, 0.0..=100.0).suffix(" %")));
    });
    seed_row(ui, t, &mut p.seed);
    grid(ui, &format!("battle_params_{salt}"), |ui| {
        row(ui, t, "Crater Count:", &mut p.crater_count, 0..=48, 4, 0.05);
        row(ui, t, "Crater Radius:", &mut p.crater_radius, 0.3..=200.0, 6.0, 0.05);
        row(ui, t, "Crater Depth:", &mut p.crater_depth, 0.05..=100.0, 2.5, 0.02);
        row(ui, t, "Rubble Density:", &mut p.rubble, 0.0..=1.0, 0.6, 0.005);
        row(ui, t, "Soot Darkening:", &mut p.soot, 0.0..=1.0, 0.6, 0.005);
    });
    ui.label(dim(match p.mode {
        DestructionMode::Modern => "Sheared floors, bent rebar, shattered glass, bomb and mech-footprint craters.",
        DestructionMode::MedievalSiege => "V-breaches, fallen merlons, charred rafters (ID 9), impact pits and ruts. No rebar.",
        DestructionMode::AncientRuins => "Eroded walls, caved roofs, snapped pillars, sinkholes, moss (ID 8).",
    }));
    t.r(ui.checkbox(&mut p.plumes, "Spawn Rising Smoke Plumes")).on_hover_text("Columns of smoke (ID 29) rise from the tallest wrecks, with fire (ID 25) at the base");
    if p.plumes {
        grid(ui, &format!("battle_plumes_{salt}"), |ui| row(ui, t, "Smoke Plume Count:", &mut p.plume_count, 0..=40, 3, 0.05));
        ui.collapsing("Plume Look", |ui| super::smoke::params(ui, t, &mut p.plume, &format!("battle_{salt}")));
    }
}

/// GhostTown stack modifier.
pub fn settlement_modifier(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "GhostTown Settlement", true, |ui| {
        ui.label(dim("Fills a flat Plane / Editable Poly, or a closed spline's outline."));
        let mut out = super::ruin::Out::default();
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Settlement(m) = k else { return };
            out = settlement_params(ui, t, &mut m.params, false);
        });
        glow(ed, out);
        pack_button(ui, ed);
    });
}

/// Medieval Architect stack modifier.
pub fn architect_modifier(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Medieval Architect", true, |ui| {
        ui.label(dim("Bricks, merlons, floors and windows are recounted from the base size: resize it, details never stretch."));
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Architect(a) = k else { return };
            architect_params(ui, t, a);
        });
        pack_button(ui, ed);
    });
}

fn architect_params(ui: &mut egui::Ui, t: &mut Track, a: &mut ArchitectMod) {
    egui::ComboBox::from_id_salt("arch_preset").width(CONTENT_W - 12.0).selected_text(a.preset.label()).show_ui(ui, |ui| {
        for p in ArchPreset::ALL {
            t.r(ui.selectable_value(&mut a.preset, p, p.label()));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Plan:");
        for (s, l) in [(PlanShape::Auto, "Auto"), (PlanShape::Round, "Round"), (PlanShape::Square, "Square")] {
            t.r(ui.radio_value(&mut a.shape, s, l));
        }
    });
    ui.label(RichText::new("Details").strong());
    egui::Grid::new("arch_toggles").num_columns(2).show(ui, |ui| {
        t.r(ui.checkbox(&mut a.bricks, "3D Stone Bricks"));
        t.r(ui.checkbox(&mut a.crenellations, "Crenellations"));
        ui.end_row();
        t.r(ui.checkbox(&mut a.machicolations, "Machicolations"));
        t.r(ui.checkbox(&mut a.windows, "Arrow Slits / Windows"));
        ui.end_row();
        t.r(ui.checkbox(&mut a.door, "Door / Portcullis"));
        t.r(ui.checkbox(&mut a.buttresses, "Buttresses"));
        ui.end_row();
        t.r(ui.checkbox(&mut a.roof, "Roof / Spire"));
        ui.end_row();
    });
    grid(ui, "arch_params", |ui| {
        row(ui, t, "Master Brick Scale:", &mut a.brick_scale, 0.1..=20.0, 1.0, 0.01);
        row(ui, t, "Floor Height:", &mut a.floor_height, 1.0..=20.0, 3.5, 0.02);
        row(ui, t, "Wall Thickness:", &mut a.wall_thickness, 0.3..=20.0, 2.0, 0.02);
        row(ui, t, "Merlon/Crenel Ratio:", &mut a.merlon_ratio, 0.25..=6.0, 1.5, 0.01);
        row(ui, t, "Window Spacing:", &mut a.window_spacing, 0.5..=50.0, 3.0, 0.02);
        row(ui, t, "Spline Wall Height:", &mut a.spline_height, 1.0..=200.0, 9.0, 0.05);
    });
    seed_row(ui, t, &mut a.seed);
    damaged(ui, t, &mut a.damaged, &mut a.damage, "architect");
}

/// Battlefield / Ruin stack modifier with its Blast Zones sub-object.
pub fn battlefield_modifier(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Battlefield / Ruin", true, |ui| {
        sub_buttons(ui, ed, &[ModSub::BlastZones]);
        ui.label(dim("No zones: seeded blasts hit the structures. Select zones, then move with W and scale with R."));
        let bounds = ed.current().and_then(|id| ed.scene.objects[id].level_mesh.bounds());
        let mut out = super::ruin::Out::default();
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Battlefield(b) = k else { return };
            ui.horizontal(|ui| {
                if ui.add_sized([CONTENT_W / 2.0 - 4.0, 24.0], egui::Button::new("➕ Add Zone")).clicked() {
                    b.add_zone(bounds);
                    (t.started, t.changed) = (true, true);
                }
                let any = b.zones.iter().any(|z| z.selected);
                if ui.add_enabled(any, egui::Button::new("🗑 Remove Zone").min_size(egui::vec2(CONTENT_W / 2.0 - 4.0, 24.0))).clicked() {
                    b.remove_selected();
                    (t.started, t.changed) = (true, true);
                }
            });
            grid(ui, "blast_zones", |ui| {
                for (i, z) in b.zones.iter_mut().enumerate() {
                    if ui.selectable_label(z.selected, format!("Zone {}", i + 1)).clicked() {
                        z.selected = !z.selected;
                        t.changed = true;
                    }
                    t.r(crate::ui::widgets::max_spinner_range(ui, &mut z.radius, 0.05, 15.0, 0.1..=10_000.0));
                    ui.end_row();
                }
            });
            battle_params(ui, t, &mut b.params, "modifier");
            ui.separator();
            ui.label(RichText::new("Building Destruction").strong());
            ui.label(dim("Groups the input into buildings (overlapping footprints) and wrecks each one."));
            out = super::ruin::destruction(ui, t, &mut b.params.buildings, "battlefield", false);
        });
        glow(ed, out);
        pack_button(ui, ed);
    });
}
