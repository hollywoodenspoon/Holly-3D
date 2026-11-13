//! Material Library browser: category list, search, favourites and preview
//! swatches of every ready-made material. Clicking a swatch loads it into the
//! active Material Editor slot (undoable).

use crate::editor::Editor;
use crate::materials::library::{self, Category, Item, Kind};
use crate::materials::preview;
use crate::ui::theme;
use crate::ui::widgets::{dim, toggle_button};
use bevy_egui::egui::{self, Color32, RichText};
use std::collections::HashMap;

const SWATCH: f32 = 72.0;
const PX: u32 = 64;
/// Previews rendered per frame, so opening the browser never stalls.
const BUDGET: usize = 4;

/// Which items the browser lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shelf {
    #[default]
    All,
    Favourites,
    Cat(Category),
}

/// UI-only browser state: filters and cached swatches.
#[derive(Default)]
pub struct LibraryUi {
    pub shelf: Shelf,
    pub search: String,
    /// `[ Procedural | Photo | Both ]`.
    pub kind: Kind,
    swatches: HashMap<&'static str, egui::TextureHandle>,
}

/// The items a shelf and search text list, in browser order.
pub fn listed(shelf: Shelf, search: &str, favourites: &[String], kind: Kind) -> Vec<(Category, Item)> {
    let q = search.trim().to_lowercase();
    library::all()
        .into_iter()
        .filter(|(_, i)| kind.shows(*i))
        .filter(|(c, i)| match shelf {
            Shelf::All => true,
            Shelf::Favourites => favourites.iter().any(|f| f == i.name()),
            Shelf::Cat(k) => *c == k,
        })
        .filter(|(c, i)| q.is_empty() || i.name().to_lowercase().contains(&q) || c.label().to_lowercase().contains(&q))
        .collect()
}

impl LibraryUi {
    pub fn clear_textures(&mut self) {
        self.swatches.clear();
    }

    /// The swatch of `item`, rendering it now if the frame budget allows.
    fn swatch(&mut self, ctx: &egui::Context, item: Item, budget: &mut usize) -> Option<egui::TextureId> {
        if let Some(t) = self.swatches.get(item.name()) {
            return Some(t.id());
        }
        if *budget == 0 {
            ctx.request_repaint();
            return None;
        }
        *budget -= 1;
        let px = preview::render_preview(&item.material(), PX);
        let bytes: Vec<u8> = px.iter().flatten().copied().collect();
        let img = egui::ColorImage::from_rgba_unmultiplied([PX as usize; 2], &bytes);
        let tex = ctx.load_texture(format!("matlib_{}", item.name()), img, egui::TextureOptions::LINEAR);
        let id = tex.id();
        self.swatches.insert(item.name(), tex);
        Some(id)
    }
}

fn shelves(ui: &mut egui::Ui, st: &mut LibraryUi, favourites: usize) {
    egui::ScrollArea::vertical().id_salt("matlib_shelves").auto_shrink([false, false]).show(ui, |ui| {
        let mut row = |ui: &mut egui::Ui, s: Shelf, text: String| {
            if ui.selectable_label(st.shelf == s, text).clicked() {
                st.shelf = s;
            }
        };
        row(ui, Shelf::All, format!("All ({})", library::all().len()));
        row(ui, Shelf::Favourites, format!("★ Favourites ({favourites})"));
        ui.separator();
        for c in Category::ALL {
            row(ui, Shelf::Cat(c), format!("{} ({})", c.label(), c.items().len()));
        }
    });
}

/// One swatch cell; returns (apply, toggle favourite).
fn cell(ui: &mut egui::Ui, st: &mut LibraryUi, item: Item, fav: bool, budget: &mut usize) -> (bool, bool) {
    let mut out = (false, false);
    ui.vertical(|ui| {
        ui.set_width(SWATCH);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(SWATCH, SWATCH), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            match st.swatch(ui.ctx(), item, budget) {
                Some(tex) => _ = p.image(tex, rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE),
                None => _ = p.rect_filled(rect, 10.0, theme::border()),
            }
            let stroke = if resp.hovered() { egui::Stroke::new(2.0, theme::amber()) } else { egui::Stroke::new(1.0, theme::border()) };
            p.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
        }
        out.0 = resp.on_hover_text(format!("{}\nClick: load into the active slot", item.name())).clicked();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let star = if fav { RichText::new("★").color(theme::amber()) } else { RichText::new("☆") };
            out.1 = ui.add(egui::Button::new(star).small().frame(false)).on_hover_text("Favourite").clicked();
            ui.add(egui::Label::new(RichText::new(item.name()).small()).truncate());
        });
    });
    out
}

fn grid(ui: &mut egui::Ui, ed: &mut Editor, st: &mut LibraryUi) {
    let items = listed(st.shelf, &st.search, &ed.prefs.general.favourite_materials, st.kind);
    if items.is_empty() {
        let none_installed = st.kind == Kind::Photo && crate::assets::photo::all().is_empty();
        ui.label(dim(if none_installed {
            "No photo materials yet. Download the Photo Textures pack in Preferences > Assets."
        } else {
            "Nothing matches. Clear the search or pick another category."
        }));
        return;
    }
    let (mut apply, mut star) = (None, None);
    let mut budget = BUDGET;
    let gap = 8.0;
    let cols = ((ui.available_width() - 16.0 + gap) / (SWATCH + gap)).floor().max(1.0) as usize;
    egui::ScrollArea::vertical().id_salt("matlib_grid").auto_shrink([false, false]).show(ui, |ui| {
        for row in items.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &(_, item) in row {
                    let fav = ed.prefs.general.favourite_materials.iter().any(|f| f == item.name());
                    let (a, s) = cell(ui, st, item, fav, &mut budget);
                    if a {
                        apply = Some(item);
                    }
                    if s {
                        star = Some(item);
                    }
                }
            });
            ui.add_space(6.0);
        }
    });
    if let Some(i) = apply {
        ed.apply_library(i);
    }
    if let Some(i) = star {
        ed.toggle_favourite_material(i.name());
    }
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, st: &mut LibraryUi) {
    if !ed.mat_editor.library {
        return;
    }
    let mut open = true;
    let win = egui::Window::new(RichText::new("Material Library").strong())
        .id(egui::Id::new("material_library"))
        .default_pos([660.0, 90.0])
        .default_size([720.0, 560.0])
        .resizable(true);
    crate::ui::popout::window(ctx, crate::ui::popout::Dock::MaterialLibrary, win, &mut open, |ui| {
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.add(egui::TextEdit::singleline(&mut st.search).hint_text("Search materials...").desired_width(220.0));
                if ui.button("✖").on_hover_text("Clear the search").clicked() {
                    st.search.clear();
                }
                ui.separator();
                for k in Kind::ALL {
                    if toggle_button(ui, st.kind == k, k.label()).on_hover_text("Show procedural materials, photo materials, or both").clicked() {
                        st.kind = k;
                    }
                }
                ui.separator();
                let slot = ed.mat_editor.slot + 1;
                ui.label(dim(&format!("Loads into slot {slot}")));
                if ui.button("⬇ Assign to Selection").on_hover_text("Assign the active slot's material to the selected objects").clicked() {
                    ed.assign_material_to_selection();
                }
                if toggle_button(ui, ed.mat_editor.open, "🎨 Editor").on_hover_text("Material Editor (M)").clicked() {
                    ed.mat_editor.open = !ed.mat_editor.open;
                }
            });
            ui.separator();
            let favourites = ed.prefs.general.favourite_materials.len();
            ui.horizontal_top(|ui| {
                let h = ui.available_height();
                let top_down = egui::Layout::top_down_justified(egui::Align::Min);
                ui.allocate_ui_with_layout(egui::vec2(190.0, h), top_down, |ui| {
                    ui.set_width(190.0);
                    shelves(ui, st, favourites);
                });
                ui.separator();
                ui.vertical(|ui| grid(ui, ed, st));
            });
        });
    if !open {
        ed.mat_editor.library = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelves_and_search_filter_the_list() {
        let all = listed(Shelf::All, "", &[], Kind::Both);
        assert_eq!(all.len(), library::all().len());
        let roads = listed(Shelf::Cat(Category::ALL[2]), "", &[], Kind::Both);
        assert!(!roads.is_empty() && roads.len() < all.len());
        assert!(listed(Shelf::All, "ASPHALT", &[], Kind::Both).iter().all(|(_, i)| i.name().to_lowercase().contains("asphalt")));
        assert!(listed(Shelf::Favourites, "", &[], Kind::Both).is_empty());
        let favs = listed(Shelf::Favourites, "", &["Red Brick".into(), "Gold".into()], Kind::Both);
        assert_eq!(favs.len(), 2);
    }

    #[test]
    fn library_items_load_with_undo_and_favourites_toggle() {
        let mut ed = Editor::default();
        let before = ed.editing_material().clone();
        ed.apply_library(library::find("Red Brick").unwrap());
        assert_eq!(ed.editing_material().name, "Red Brick");
        assert_ne!(*ed.editing_material(), before);
        ed.undo();
        assert_eq!(*ed.editing_material(), before);
        ed.toggle_favourite_material("Gold");
        assert_eq!(ed.prefs.general.favourite_materials, ["Gold"]);
        ed.toggle_favourite_material("Gold");
        assert!(ed.prefs.general.favourite_materials.is_empty());
    }
}
