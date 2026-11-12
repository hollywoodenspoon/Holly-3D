//! Help window: topics, step-by-step pages with screenshots, the
//! Getting Started tour and the hotkey cheat sheet.

pub mod pages;
pub mod pages2;
pub mod pages3;
pub mod shots;
mod tour;
pub mod view;

use crate::editor::{CommandTab, Editor};
use crate::mesh::SubObject;
use crate::scene::BaseObject;
use crate::ui::command::PANEL_W;
use bevy_egui::egui;
use pages2::{ALL, index_of, text_of};

/// UI-only Help state.
#[derive(Default)]
pub struct HelpUi {
    pub search: String,
    pub textures: view::Textures,
    /// The startup tour was considered this session.
    pub tour_checked: bool,
}

/// Which window (by egui id) leads to which Help page.
const WINDOW_PAGES: [(&str, &str); 14] = [
    ("material_editor", "materials"),
    ("material_library", "materials"),
    ("render_setup", "render"),
    ("rfw", "render"),
    ("paint_box", "paint"),
    ("env_dialog", "sky"),
    ("uv_editor", "modifiers"),
    ("caddy", "editpoly"),
    ("Preferences", "prefs"),
    ("Hotkey List", "hotkeys"),
    ("Array", "transform"),
    ("Mirror: World Coordinates", "transform"),
    ("align_dialog", "transform"),
    ("Grid and Snap Settings", "transform"),
];

/// The Help page for what is under the mouse (F1).
fn context_page(ctx: &egui::Context, ed: &Editor) -> usize {
    let Some(pos) = ctx.pointer_latest_pos() else { return 0 };
    if let Some(layer) = ctx.layer_id_at(pos) {
        for (title, page) in WINDOW_PAGES {
            if layer.id == egui::Id::new(title) {
                return index_of(page);
            }
        }
    }
    if pos.x < ctx.viewport_rect().max.x - PANEL_W {
        return index_of(if ed.paint.active { "paint" } else { "basics" });
    }
    let poly = ed.current().is_some_and(|id| matches!(ed.scene.objects[id].base, BaseObject::EditablePoly(_)));
    index_of(match ed.tab {
        CommandTab::Create => "create",
        CommandTab::Modify if poly || ed.sub != SubObject::Object => "editpoly",
        CommandTab::Modify => "modifiers",
        _ => "basics",
    })
}

/// Pages matching the search words (all pages when it is empty).
pub fn matching(search: &str) -> Vec<usize> {
    let words: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
    (0..ALL.len()).filter(|&i| words.is_empty() || words.iter().all(|w| text_of(&ALL[i]).contains(w.as_str()))).collect()
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, st: &mut HelpUi) {
    if !st.tour_checked {
        st.tour_checked = true;
        if ed.prefs.general.show_tour && ed.iface.tour.is_none() {
            ed.iface.tour = Some(0);
        }
    }
    tour::show(ctx, ed, &mut st.textures);
    if ed.iface.help_context {
        ed.iface.help_context = false;
        if ed.iface.help_open {
            ed.iface.help_page = context_page(ctx, ed);
        }
    }
    if !ed.iface.help_open {
        return;
    }
    let mut open = true;
    let win = egui::Window::new("Help").open(&mut open).default_size([980.0, 680.0]).collapsible(false);
    win.pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.content_rect().center()).show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(200.0);
                ui.add(egui::TextEdit::singleline(&mut st.search).hint_text("🔍 Search help").desired_width(196.0));
                let hits = matching(&st.search);
                if hits.is_empty() {
                    ui.label(crate::ui::widgets::dim("Nothing found"));
                }
                egui::ScrollArea::vertical().id_salt("help_topics").max_height(560.0).show(ui, |ui| {
                    for i in hits {
                        if ui.selectable_label(ed.iface.help_page == i, ALL[i].title).clicked() {
                            ed.iface.help_page = i;
                        }
                    }
                });
                ui.separator();
                if ui.button("▶ Getting Started Tour").clicked() {
                    ed.iface.tour = Some(0);
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let i = ed.iface.help_page.min(ALL.len() - 1);
                    if ui.add_enabled(i > 0, egui::Button::new("◀ Previous")).clicked() {
                        ed.iface.help_page = i - 1;
                    }
                    if ui.add_enabled(i + 1 < ALL.len(), egui::Button::new("Next ▶")).clicked() {
                        ed.iface.help_page = i + 1;
                    }
                });
                egui::ScrollArea::vertical().id_salt(("help_page", ed.iface.help_page)).auto_shrink(false).show(ui, |ui| {
                    let p = ALL[ed.iface.help_page.min(ALL.len() - 1)];
                    view::page(ui, ed, &mut st.textures, &p);
                });
            });
        });
    });
    ed.iface.help_open &= open;
}

#[cfg(test)]
mod tests {
    use super::pages::Block;
    use super::*;

    #[test]
    fn every_page_shot_is_built_in_and_marks_are_on_it() {
        for p in ALL {
            for b in p.blocks {
                if let Block::Shot(id, marks) = b {
                    assert!(shots::SHOTS.iter().any(|s| s.0 == *id), "{}: {id}", p.id);
                    assert!(!marks.is_empty());
                    for mk in *marks {
                        assert!((0.0..=1.0).contains(&mk.at[0]) && (0.0..=1.0).contains(&mk.at[1]), "{}: {}", p.id, mk.label);
                    }
                }
            }
        }
        for id in shots::IDS {
            assert!(ALL.iter().any(|p| p.blocks.iter().any(|b| matches!(b, Block::Shot(s, _) if *s == id))), "{id} is unused");
        }
    }

    #[test]
    fn search_and_ids() {
        assert_eq!(matching("").len(), ALL.len());
        assert!(matching("deep space").contains(&index_of("sky")));
        assert!(matching("BEZIER").contains(&index_of("splines")));
        assert!(matching("zzzqqq").is_empty());
        assert_eq!(ALL[index_of("render")].id, "render");
        assert_eq!(index_of("nope"), 0);
        let mut ids: Vec<_> = ALL.iter().map(|p| p.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), ALL.len());
    }

    #[test]
    fn help_action_and_context() {
        let mut ed = Editor::default();
        crate::ui::actions::Action::Help.run(&mut ed);
        assert!(ed.iface.help_open && ed.iface.help_context);
        let ctx = egui::Context::default();
        assert_eq!(context_page(&ctx, &ed), 0, "no pointer: Getting Started");
    }
}
