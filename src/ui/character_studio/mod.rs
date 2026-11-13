//! The MakeHuman window (header `MakeHuman`, Tools menu, Create and
//! Characters tabs): tabs for the archetype and creature tiles with
//! Randomise, body macros, detail sliders, hair, eyes, clothes, skin,
//! pose, Face Cloner, stylised low-poly parts, and the asset Library.

pub mod library_ui;
pub mod params;
pub mod pose;
pub mod wardrobe;

use crate::character::params::{Archetype, CharacterParams, Creature};
use crate::character::randomise::{randomise, Scope};
use crate::editor::characters::CharacterPick;
use crate::editor::{Editor, Tool};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim, rollout};
use bevy::math::Vec3;
use bevy_egui::egui;

/// The MakeHuman window's tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Main,
    Body,
    Detail,
    Hair,
    Eyes,
    Clothes,
    Skin,
    Pose,
    FaceCloner,
    Stylised,
    Library,
}

impl Tab {
    pub const ALL: [Tab; 11] = [Self::Main, Self::Body, Self::Detail, Self::Hair, Self::Eyes, Self::Clothes, Self::Skin, Self::Pose, Self::FaceCloner, Self::Stylised, Self::Library];
    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Body => "Body",
            Self::Detail => "Face & Body Detail",
            Self::Hair => "Hair",
            Self::Eyes => "Eyes",
            Self::Clothes => "Clothes",
            Self::Skin => "Skin",
            Self::Pose => "Pose",
            Self::FaceCloner => "Face Cloner",
            Self::Stylised => "Stylised",
            Self::Library => "Library",
        }
    }
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.iface.character_studio_open {
        return;
    }
    let mut open = true;
    let win = egui::Window::new("MakeHuman").default_size([420.0, 640.0]);
    crate::ui::popout::window(ctx, crate::ui::popout::Dock::CharacterStudio, win, &mut open, |ui| contents(ui, ed));
    ed.iface.character_studio_open &= open;
}

fn contents(ui: &mut egui::Ui, ed: &mut Editor) {
    let id = egui::Id::new("mh_window_tab");
    let mut tab: Tab = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.horizontal_wrapped(|ui| {
        for x in Tab::ALL {
            ui.selectable_value(&mut tab, x, x.label());
        }
    });
    ui.data_mut(|d| d.insert_temp(id, tab));
    ui.separator();
    egui::ScrollArea::vertical().id_salt("mh_window_body").show(ui, |ui| {
        if tab == Tab::Library {
            return library_ui::show(ui);
        }
        if tab == Tab::Main {
            gallery(ui, ed, 3);
            ui.separator();
        }
        let Some((oid, mut p)) = ed.character() else {
            ui.label(dim("Select a character (or place one from Main) to edit it here."));
            return;
        };
        let mut t = Track::default();
        page(ui, ed, tab, &mut t, &mut p);
        if t.started {
            ed.checkpoint("Character Parameters");
        }
        if t.changed {
            ed.set_character(oid, p);
        }
    });
}

fn page(ui: &mut egui::Ui, ed: &mut Editor, tab: Tab, t: &mut Track, p: &mut CharacterParams) {
    let human = p.creature == Creature::Humanoid;
    match tab {
        Tab::Main => {
            params::archetype(ui, t, p);
            if human {
                ui.separator();
                randomise_ui(ui, t, p);
            }
        }
        _ if !human => {
            ui.label(dim("This page is for humanoids; creatures are set on the Main page."));
        }
        Tab::Body => params::body(ui, t, p),
        Tab::Detail => params::micro(ui, t, p),
        Tab::Hair => wardrobe::hair(ui, t, p),
        Tab::Eyes => wardrobe::eyes(ui, t, p),
        Tab::Clothes => wardrobe::clothes(ui, t, p, 420.0),
        Tab::Skin => {
            wardrobe::skin(ui, t, p);
            wardrobe::kit_buttons(ui, t, p);
        }
        Tab::Pose => pose::rollout_ui(ui, t, p),
        Tab::FaceCloner => crate::ui::face_cloner::rollout::show(ui, &mut ed.iface, t, p),
        Tab::Stylised => {
            params::stylised_toggle(ui);
            if params::stylised_shown(ui, p) {
                params::stylised(ui, t, p);
            } else {
                ui.label(dim("Low-poly procedural hair, outfits and weapons for stylised characters. Turn them on above."));
            }
        }
        Tab::Library => {}
    }
}

#[derive(Clone, Copy)]
struct RandUi {
    seed: u64,
    scope: Scope,
    keep_gender: bool,
    strength: f32,
}

impl Default for RandUi {
    fn default() -> Self {
        Self { seed: 1, scope: Scope::Everything, keep_gender: true, strength: 0.6 }
    }
}

fn randomise_ui(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    let id = egui::Id::new("mh_randomise");
    let mut r: RandUi = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.label(egui::RichText::new("Randomise").strong());
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("mh_rand_scope").selected_text(r.scope.label()).show_ui(ui, |ui| {
            for s in Scope::ALL {
                ui.selectable_value(&mut r.scope, s, s.label());
            }
        });
        ui.checkbox(&mut r.keep_gender, "Keep gender");
    });
    ui.horizontal(|ui| {
        ui.label("Face variation:");
        ui.add(egui::Slider::new(&mut r.strength, 0.0..=1.0).max_decimals(2));
    });
    ui.horizontal(|ui| {
        ui.label("Seed:");
        ui.add(egui::DragValue::new(&mut r.seed));
        let mut go = ui.button("Apply Seed").on_hover_text("The same seed always gives the same person").clicked();
        if ui.button("🎲 Randomise").on_hover_text("A new person from a new seed").clicked() {
            r.seed = r.seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407) >> 40;
            go = true;
        }
        if go {
            randomise(p, r.seed, r.scope, r.keep_gender, r.strength);
            (t.started, t.changed) = (true, true);
        }
    });
    ui.data_mut(|d| d.insert_temp(id, r));
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
        if ui.add_sized([content_w(), 26.0], egui::Button::new("Open MakeHuman Window...").selected(ed.iface.character_studio_open)).clicked() {
            ed.iface.character_studio_open ^= true;
        }
    });
}
