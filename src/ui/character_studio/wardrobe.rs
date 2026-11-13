//! MakeHuman asset pickers: hair (with dye colours), brows and lashes,
//! eyes, teeth and tongue, clothes, skin and body topology. Each section
//! is drawn bare in a MakeHuman window tab, or inside the "Character —
//! MakeHuman Assets" rollout on the Modify panel.

use crate::character::makehuman::library::{all_items, eye_colors, pretty, skins, Category};
use crate::character::params::{CharacterParams, Eyes};
use crate::character::wardrobe::{MhAssets, MAX_CLOTHES};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim, rollout};
use bevy_egui::egui;

/// Named hair dyes (sRGB).
pub const DYES: [(&str, [f32; 3]); 12] = [
    ("Black", [0.09, 0.075, 0.07]),
    ("Dark Brown", [0.20, 0.13, 0.09]),
    ("Brown", [0.36, 0.24, 0.15]),
    ("Chestnut", [0.45, 0.25, 0.13]),
    ("Auburn", [0.55, 0.22, 0.12]),
    ("Ginger", [0.72, 0.36, 0.16]),
    ("Blonde", [0.85, 0.70, 0.45]),
    ("Platinum", [0.86, 0.82, 0.74]),
    ("Grey", [0.62, 0.62, 0.62]),
    ("Blue", [0.20, 0.35, 0.80]),
    ("Pink", [0.90, 0.45, 0.65]),
    ("Green", [0.25, 0.60, 0.35]),
];

/// A "None" + items combo box for one optional slot.
fn pick(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut Option<String>, all: &[String]) {
    ui.horizontal(|ui| {
        ui.add_sized([90.0, 18.0], egui::Label::new(label));
        let shown = v.as_deref().map_or("None".to_string(), pretty);
        egui::ComboBox::from_id_salt(("mh_pick", label)).width(content_w() - 110.0).selected_text(shown).show_ui(ui, |ui| {
            t.r(ui.selectable_value(v, None, "None"));
            for x in all {
                t.r(ui.selectable_value(v, Some(x.clone()), pretty(x)));
            }
        });
    });
}

/// A grid of selectable name tiles ("None" first).
fn tiles(ui: &mut egui::Ui, t: &mut Track, salt: &str, v: &mut Option<String>, all: &[String]) {
    let cols = ((content_w() / 120.0) as usize).max(2);
    let w = content_w() / cols as f32 - 6.0;
    egui::Grid::new(("mh_tiles", salt)).num_columns(cols).show(ui, |ui| {
        let none = std::iter::once(None).chain(all.iter().map(|s| Some(s.clone())));
        for (i, x) in none.enumerate() {
            let name = x.as_deref().map_or("None".to_string(), pretty);
            if t.r(ui.add_sized([w, 26.0], egui::Button::new(name).selected(*v == x))).clicked() && *v != x {
                *v = x;
                t.changed = true;
            }
            if i % cols == cols - 1 {
                ui.end_row();
            }
        }
    });
}

fn swatch(ui: &mut egui::Ui, c: [f32; 3], selected: bool, tip: &str) -> egui::Response {
    let (r, p) = ui.allocate_painter(egui::vec2(22.0, 22.0), egui::Sense::click());
    let col = egui::Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8);
    p.rect_filled(r.rect.shrink(2.0), 6.0, col);
    if selected || r.hovered() {
        let stroke = if selected { crate::ui::theme::accent() } else { crate::ui::theme::hover() };
        p.rect_stroke(r.rect.shrink(1.0), 7.0, egui::Stroke::new(2.0, stroke), egui::StrokeKind::Inside);
    }
    r.on_hover_text(tip)
}

/// Hair style tiles, dye colour, eyebrows and eyelashes.
pub fn hair(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    let mh = &mut p.mh;
    ui.label(egui::RichText::new("Hair").strong());
    tiles(ui, t, "hair", &mut mh.hair, &all_items(Category::Hair));
    ui.label(egui::RichText::new("Hair Colour").strong());
    ui.horizontal_wrapped(|ui| {
        if ui.add(egui::Button::new("Natural").selected(mh.hair_color.is_none())).on_hover_text("The texture's own colour").clicked() && mh.hair_color.is_some() {
            mh.hair_color = None;
            (t.started, t.changed) = (true, true);
        }
        for (name, c) in DYES {
            if swatch(ui, c, mh.hair_color == Some(c), name).clicked() {
                mh.hair_color = Some(c);
                (t.started, t.changed) = (true, true);
            }
        }
        let mut c = mh.hair_color.unwrap_or([0.36, 0.24, 0.15]);
        let r = ui.color_edit_button_rgb(&mut c).on_hover_text("Any colour");
        if r.changed() {
            mh.hair_color = Some(c);
            t.changed = true;
        }
        t.started |= r.clicked();
    });
    ui.label(dim("The dye keeps the strand detail (dark roots, light tips) and also colours the eyebrows."));
    pick(ui, t, "Eyebrows:", &mut mh.eyebrows, &all_items(Category::Eyebrows));
    pick(ui, t, "Eyelashes:", &mut mh.eyelashes, &all_items(Category::Eyelashes));
}

/// Eye mesh detail, eye colour, teeth and tongue.
pub fn eyes(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    ui.horizontal(|ui| {
        ui.add_sized([90.0, 18.0], egui::Label::new("Eye Mesh:"));
        for e in Eyes::ALL {
            t.r(ui.selectable_value(&mut p.eyes, *e, e.label()));
        }
    });
    ui.label(egui::RichText::new("Eye Colour").strong());
    let colors: Vec<String> = eye_colors().into_iter().map(str::to_string).collect();
    tiles(ui, t, "eyes", &mut p.mh.eye_color, &colors);
    pick(ui, t, "Teeth:", &mut p.mh.teeth, &all_items(Category::Teeth));
    pick(ui, t, "Tongue:", &mut p.mh.tongue, &all_items(Category::Tongue));
}

/// The clothes checklist.
pub fn clothes(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams, max_h: f32) {
    let mh = &mut p.mh;
    ui.label(egui::RichText::new(format!("Clothes ({} of up to {MAX_CLOTHES})", mh.clothes.len())).strong());
    egui::ScrollArea::vertical().id_salt("mh_clothes").max_height(max_h).show(ui, |ui| {
        for name in all_items(Category::Clothes) {
            let mut on = mh.clothes.contains(&name);
            if t.r(ui.checkbox(&mut on, pretty(&name))).changed() {
                mh.toggle_clothes(&name);
            }
        }
    });
    if ui.button("Take Off All Clothes").clicked() {
        mh.clothes.clear();
        (t.started, t.changed) = (true, true);
    }
    ui.label(dim("Clothes hide the skin they cover. IDs 12+ in the Character Pack."));
}

/// Skin texture and body topology.
pub fn skin(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    let mh = &mut p.mh;
    let all: Vec<String> = skins().into_iter().map(str::to_string).collect();
    ui.label(egui::RichText::new("Skin").strong());
    tiles(ui, t, "skin", &mut mh.skin, &all);
    pick(ui, t, "Topology:", &mut mh.topology, &all_items(Category::Proxy));
    ui.label(dim("Topology swaps the full body for a lighter proxy mesh that follows the same sliders."));
}

/// Face Kit / Remove All buttons.
pub fn kit_buttons(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    let mh = &mut p.mh;
    ui.horizontal_wrapped(|ui| {
        if ui.button("Face Kit").on_hover_text("Eyebrows, eyelashes, teeth and tongue").clicked() {
            let kit = MhAssets::face_kit();
            (mh.eyebrows, mh.eyelashes, mh.teeth, mh.tongue) = (kit.eyebrows, kit.eyelashes, kit.teeth, kit.tongue);
            (t.started, t.changed) = (true, true);
        }
        if ui.button("Remove All Assets").clicked() {
            *mh = MhAssets::default();
            (t.started, t.changed) = (true, true);
        }
    });
}

/// Modify-panel rollouts.
pub fn rollout_ui(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    rollout(ui, "Character — Hair", true, |ui| hair(ui, t, p));
    rollout(ui, "Character — Eyes & Mouth", false, |ui| eyes(ui, t, p));
    rollout(ui, "Character — Clothes", false, |ui| clothes(ui, t, p, 160.0));
    rollout(ui, "Character — Skin & Topology", false, |ui| {
        skin(ui, t, p);
        kit_buttons(ui, t, p);
        ui.label(dim("Assets are fitted with each one's .mhclo binding, so they follow every slider and pose. Import your own in the MakeHuman window's Library tab."));
    });
}
