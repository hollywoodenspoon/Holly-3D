//! Character Parameters rollouts (Modify panel and the Studio window):
//! archetype, MakeHuman macros, Anime Ratio, eyes / hair / outfit /
//! weapon, creature, and the 240 micro sliders grouped like MakeHuman.

use crate::character::makehuman::modifiers::sliders;
use crate::character::params::*;
use crate::editor::Editor;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, rollout};
use bevy_egui::egui;

fn combo<T: Copy + PartialEq>(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut T, all: &[T], name: fn(T) -> &'static str) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(("char", label)).width(CONTENT_W - 110.0).selected_text(name(*v)).show_ui(ui, |ui| {
            for &x in all {
                t.r(ui.selectable_value(v, x, name(x)));
            }
        });
    });
}

fn slider(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, default: f32) {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 18.0], egui::Label::new(label).truncate());
        let r = t.r(ui.add(egui::Slider::new(v, range).max_decimals(2)));
        if r.secondary_clicked() {
            *v = default;
            t.changed = true;
        }
    });
}

/// Draws every rollout for `p`; returns true when it changed. Undo is
/// recorded by the caller on `Track::started`.
pub fn edit(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    rollout(ui, "Character — Archetype", true, |ui| {
        let mut a = p.archetype;
        combo(ui, t, "Archetype:", &mut a, Archetype::ALL, Archetype::label);
        if a != p.archetype {
            a.apply(p);
        }
        ui.label(dim("Picking an archetype resets the sliders to its recipe (right-click a slider to reset it)."));
        combo(ui, t, "Creature:", &mut p.creature, Creature::ALL, Creature::label);
        if p.creature != Creature::Humanoid {
            slider(ui, t, "Creature Size", &mut p.creature_size, 0.2..=6.0, 1.0);
            ui.horizontal(|ui| {
                ui.label("Seed:");
                t.r(ui.add(egui::DragValue::new(&mut p.seed)));
            });
        }
    });
    if p.creature != Creature::Humanoid {
        return;
    }
    rollout(ui, "Character — Body (MakeHuman Macros)", true, |ui| {
        let m = &mut p.macros;
        slider(ui, t, "Gender ♀ … ♂", &mut m.gender, 0.0..=1.0, 0.5);
        slider(ui, t, "Age", &mut m.age, 0.0..=1.0, 0.5);
        slider(ui, t, "Muscle", &mut m.muscle, 0.0..=1.0, 0.5);
        slider(ui, t, "Weight", &mut m.weight, 0.0..=1.0, 0.5);
        slider(ui, t, "Height", &mut m.height, 0.0..=1.0, 0.5);
        slider(ui, t, "Proportions", &mut m.proportions, 0.0..=1.0, 0.5);
        slider(ui, t, "Breast Size", &mut m.breast_size, 0.0..=1.0, 0.5);
        slider(ui, t, "Breast Firmness", &mut m.breast_firmness, 0.0..=1.0, 0.5);
        ui.label(dim("Ethnic mix (normalised):"));
        slider(ui, t, "African", &mut m.african, 0.0..=1.0, 1.0 / 3.0);
        slider(ui, t, "Asian", &mut m.asian, 0.0..=1.0, 1.0 / 3.0);
        slider(ui, t, "Caucasian", &mut m.caucasian, 0.0..=1.0, 1.0 / 3.0);
        slider(ui, t, "Anime Ratio", &mut p.anime, 0.0..=1.0, 0.0);
    });
    rollout(ui, "Character — Eyes, Hair, Outfit & Weapon", true, |ui| {
        combo(ui, t, "Eyes:", &mut p.eyes, Eyes::ALL, Eyes::label);
        combo(ui, t, "Hair:", &mut p.hair, Hair::ALL, Hair::label);
        slider(ui, t, "Hair Length", &mut p.hair_length, 0.0..=1.0, 0.5);
        combo(ui, t, "Outfit:", &mut p.outfit, Outfit::ALL, Outfit::label);
        combo(ui, t, "Weapon:", &mut p.weapon, Weapon::ALL, Weapon::label);
        ui.label(dim("Material IDs: 1 Skin · 2 Eyes · 3 Hair · 4 Cloth · 5 Metal · 6 Leather · 7–12+ MakeHuman assets (Character Pack)."));
    });
    super::wardrobe::rollout_ui(ui, t, p);
    super::pose::rollout_ui(ui, t, p);
    micro(ui, t, p);
}

fn micro(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    rollout(ui, "Character — Detail Sliders (MakeHuman)", false, |ui| {
        let id = egui::Id::new("char_micro_filter");
        let mut filter: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
        let mut mirror: bool = ui.data_mut(|d| d.get_temp(id.with("mirror")).unwrap_or(true));
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add(egui::TextEdit::singleline(&mut filter).desired_width(120.0).hint_text("nose, ear..."));
            ui.checkbox(&mut mirror, "Mirror L/R");
        });
        let needle = filter.to_lowercase();
        let mut group = "";
        for s in sliders() {
            if mirror && s.key.contains("/l-") {
                continue;
            }
            if !needle.is_empty() && !s.key.contains(&needle) {
                continue;
            }
            if s.group != group {
                group = &s.group;
                ui.label(egui::RichText::new(group.to_uppercase()).strong());
            }
            let mut v = p.micro.get(&s.key).copied().unwrap_or(0.0);
            let lo = if s.two_sided() { -1.0 } else { 0.0 };
            let before = v;
            slider(ui, t, &s.label, &mut v, lo..=1.0, 0.0);
            if v != before {
                if mirror {
                    p.set_micro(&s.key, v);
                } else {
                    p.micro.insert(s.key.clone(), v);
                }
            }
        }
        if ui.button("Reset All Detail Sliders").clicked() {
            p.micro.clear();
            t.started = true;
            t.changed = true;
        }
        ui.data_mut(|d| {
            d.insert_temp(id, filter);
            d.insert_temp(id.with("mirror"), mirror);
        });
    });
}

/// Modify-panel entry: the Character Parameters of the current object.
pub fn base(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some((id, mut p)) = ed.character() else { return };
    let mut t = Track::default();
    edit(ui, &mut t, &mut p);
    if p.creature == Creature::Humanoid {
        crate::ui::face_cloner::rollout::show(ui, &mut ed.iface, &mut t, &mut p);
    }
    if t.started {
        ed.checkpoint("Character Parameters");
    }
    if t.changed {
        ed.set_character(id, p);
    }
}
