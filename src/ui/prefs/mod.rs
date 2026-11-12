//! Preferences window (toolbar ⚙ or Edit -> Preferences): Theme, Hotkeys,
//! Viewports, General, Rendering and Assets. Changes apply at once and are
//! saved to `~/.config/ilex/preferences.ron` when the mouse is released.

mod assets_tab;
mod hotkeys_tab;
mod settings_tabs;
mod theme_tab;

use super::theme;
use crate::editor::Editor;
use crate::prefs::Preferences;
use bevy_egui::egui::{self, RichText};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PrefsTab {
    #[default]
    Theme,
    Hotkeys,
    Viewports,
    General,
    Rendering,
    Assets,
}

impl PrefsTab {
    const ALL: [PrefsTab; 6] = [Self::Theme, Self::Hotkeys, Self::Viewports, Self::General, Self::Rendering, Self::Assets];
    fn label(self) -> &'static str {
        match self {
            Self::Theme => "🎨 Theme",
            Self::Hotkeys => "⌨ Hotkeys",
            Self::Viewports => "⊞ Viewports",
            Self::General => "⚙ General",
            Self::Rendering => "▶ Rendering",
            Self::Assets => "📦 Assets",
        }
    }
}

/// Window-only state (not saved, not undoable).
#[derive(Default)]
pub struct PrefsUi {
    pub tab: PrefsTab,
    pub hotkeys: hotkeys_tab::HotkeysUi,
    /// What is on disk, so changes are saved once.
    saved: Option<Preferences>,
    /// Last save error, shown in the window.
    error: Option<String>,
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, st: &mut PrefsUi) {
    persist(ctx, ed, st);
    if !ed.iface.prefs_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Preferences").open(&mut open).default_size([560.0, 520.0]).collapsible(false).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            for t in PrefsTab::ALL {
                if ui.selectable_label(st.tab == t, t.label()).clicked() && st.tab != t {
                    st.tab = t;
                    st.hotkeys.cancel();
                    ed.iface.capturing_key = false;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, true]).max_height(440.0).show(ui, |ui| match st.tab {
            PrefsTab::Theme => theme_tab::show(ui, ed),
            PrefsTab::Hotkeys => hotkeys_tab::show(ui, ed, &mut st.hotkeys),
            PrefsTab::Viewports => settings_tabs::viewports(ui, ed),
            PrefsTab::General => settings_tabs::general(ui, ed),
            PrefsTab::Rendering => settings_tabs::rendering(ui, ed),
            PrefsTab::Assets => assets_tab::assets(ui, ed),
        });
        ui.separator();
        ui.horizontal(|ui| {
            match &st.error {
                Some(e) => ui.label(RichText::new(e).color(theme::vertex_sel())),
                None => ui.label(super::widgets::dim(&format!("Saved to {}", Preferences::default_path().display()))),
            };
        });
    });
    if !open {
        ed.iface.prefs_open = false;
        ed.iface.capturing_key = false;
        st.hotkeys.cancel();
    }
}

/// Applies changed settings and writes them to disk once the mouse is up.
fn persist(ctx: &egui::Context, ed: &mut Editor, st: &mut PrefsUi) {
    let saved = st.saved.get_or_insert_with(|| ed.prefs.clone());
    if *saved == ed.prefs || ctx.input(|i| i.pointer.any_down()) {
        return;
    }
    ed.apply_prefs();
    st.error = ed.prefs.save_to(&Preferences::default_path()).err();
    st.saved = Some(ed.prefs.clone());
}

/// A "Reset" button; true when clicked.
fn reset_button(ui: &mut egui::Ui, what: &str) -> bool {
    ui.button("↺ Reset").on_hover_text(format!("Puts the {what} back to their defaults")).clicked()
}
