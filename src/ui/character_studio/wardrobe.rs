//! "Character — MakeHuman Assets" rollout: eyebrows, eyelashes, teeth,
//! tongue, hair and body topology pickers, a clothes checklist, and the
//! skin and eye colour textures from the bundled MakeHuman system pack.

use crate::character::makehuman::library::{eye_colors, items, pretty, skins, Category};
use crate::character::params::CharacterParams;
use crate::character::wardrobe::{MhAssets, MAX_CLOTHES};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, rollout};
use bevy_egui::egui;

/// A "None" + items combo box for one optional slot.
fn pick(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut Option<String>, all: &[&str]) {
    ui.horizontal(|ui| {
        ui.add_sized([90.0, 18.0], egui::Label::new(label));
        let shown = v.as_deref().map_or("None".to_string(), pretty);
        egui::ComboBox::from_id_salt(("mh_pick", label)).width(CONTENT_W - 110.0).selected_text(shown).show_ui(ui, |ui| {
            t.r(ui.selectable_value(v, None, "None"));
            for &x in all {
                t.r(ui.selectable_value(v, Some(x.to_string()), pretty(x)));
            }
        });
    });
}

pub fn rollout_ui(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    rollout(ui, "Character — MakeHuman Assets (CC0)", true, |ui| {
        let mh = &mut p.mh;
        for cat in [Category::Eyebrows, Category::Eyelashes, Category::Teeth, Category::Tongue, Category::Hair, Category::Proxy] {
            if let Some(slot) = mh.slot_mut(cat) {
                pick(ui, t, &format!("{}:", cat.label()), slot, &items(cat));
            }
        }
        if mh.hair.is_some() {
            ui.label(dim("MakeHuman hair replaces the procedural hair style above."));
        }
        let mut skin = mh.skin.clone();
        pick(ui, t, "Skin:", &mut skin, &skins());
        mh.skin = skin;
        let mut eye = mh.eye_color.clone();
        pick(ui, t, "Eye Colour:", &mut eye, &eye_colors());
        mh.eye_color = eye;
        ui.label(egui::RichText::new(format!("Clothes ({} of up to {MAX_CLOTHES})", mh.clothes.len())).strong());
        egui::ScrollArea::vertical().id_salt("mh_clothes").max_height(160.0).show(ui, |ui| {
            for name in items(Category::Clothes) {
                let mut on = mh.clothes.iter().any(|c| c == name);
                if t.r(ui.checkbox(&mut on, pretty(name))).changed() {
                    mh.toggle_clothes(name);
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Face Kit").on_hover_text("Eyebrows, eyelashes, teeth and tongue").clicked() {
                let kit = MhAssets::face_kit();
                (mh.eyebrows, mh.eyelashes, mh.teeth, mh.tongue) = (kit.eyebrows, kit.eyelashes, kit.teeth, kit.tongue);
                (t.started, t.changed) = (true, true);
            }
            if ui.button("Take Off All Clothes").clicked() {
                mh.clothes.clear();
                (t.started, t.changed) = (true, true);
            }
            if ui.button("Remove All Assets").clicked() {
                *mh = MhAssets::default();
                (t.started, t.changed) = (true, true);
            }
        });
        ui.label(dim(
            "Fitted with each asset's .mhclo binding, so they follow every slider and pose. Clothes hide the skin they cover. IDs: 7 Brows · 8 Lashes · 9 Teeth · 10 Tongue · 11 Hair · 12+ Clothes; the textured materials are added to the Character Pack.",
        ));
    });
}
