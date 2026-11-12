//! Draws an `Action` as a toolbar button or a menu entry: toggles light up
//! in the accent colour, unavailable actions grey out and every tooltip names the hotkey.

use super::actions::Action;
use super::actions::keys::chord_of;
use super::actions::layout::Entry;
use super::hotkeys::keymap;
use super::theme;
use crate::editor::Editor;
use bevy_egui::egui::{self, Response, RichText};

/// "Ctrl+Z", or empty when the action has no hotkey.
pub fn hotkey_text(a: Action) -> String {
    chord_of(&keymap(), a).map(|c| c.text()).unwrap_or_default()
}

/// "Select and Move (W) — tip".
pub fn tooltip(a: Action) -> String {
    let key = hotkey_text(a);
    let mut s = a.label();
    if !key.is_empty() {
        s = format!("{s}  ({key})");
    }
    if !a.tip().is_empty() {
        s = format!("{s}\n{}", a.tip());
    }
    s
}

/// A toolbar button with `caption`; runs the action when clicked.
pub fn button_with(ui: &mut egui::Ui, ed: &mut Editor, a: Action, caption: &str) -> Response {
    let on = a.checked(ed).unwrap_or(false);
    let text = if on { RichText::new(caption).color(theme::input()).strong() } else { RichText::new(caption) };
    let r = ui.add_enabled(a.enabled(ed), egui::Button::new(text).selected(on)).on_hover_text(tooltip(a));
    if r.clicked() {
        a.run(ed);
    }
    r
}

/// A toolbar button with the action's short caption.
pub fn button(ui: &mut egui::Ui, ed: &mut Editor, a: Action) -> Response {
    button_with(ui, ed, a, &a.short())
}

/// A big fixed-size button (ribbon, nav buttons, panels).
pub fn sized_button(ui: &mut egui::Ui, ed: &mut Editor, a: Action, caption: &str, size: egui::Vec2) -> Response {
    let on = a.checked(ed).unwrap_or(false);
    let text = if on { RichText::new(caption).color(theme::input()).strong() } else { RichText::new(caption) };
    let r = ui.add_enabled_ui(a.enabled(ed), |ui| ui.add_sized(size, egui::Button::new(text).selected(on))).inner.on_hover_text(tooltip(a));
    if r.clicked() {
        a.run(ed);
    }
    r
}

/// A menu row: label, hotkey on the right, a tick for toggles that are on.
pub fn menu_item(ui: &mut egui::Ui, ed: &mut Editor, a: Action) {
    let label = match a.checked(ed) {
        Some(true) => format!("■ {}", a.label()),
        Some(false) => format!("☐ {}", a.label()),
        None => a.label(),
    };
    let btn = egui::Button::new(label).shortcut_text(hotkey_text(a));
    let r = ui.add_enabled(a.enabled(ed), btn);
    let r = if a.tip().is_empty() { r } else { r.on_hover_text(a.tip()) };
    if r.clicked() {
        a.run(ed);
        ui.close();
    }
}

/// Draws a table of menu entries (with cascading sub-menus).
pub fn menu_entries(ui: &mut egui::Ui, ed: &mut Editor, entries: &[Entry]) {
    for e in entries {
        match e {
            Entry::A(a) => menu_item(ui, ed, *a),
            Entry::Sep => {
                ui.separator();
            }
            Entry::Sub(title, sub) => {
                ui.menu_button(*title, |ui| menu_entries(ui, ed, sub));
            }
        }
    }
}
