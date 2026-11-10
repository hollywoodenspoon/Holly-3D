//! Preferences -> Hotkeys: every action in one searchable list. Click a key,
//! press a new one; conflicts ask whether to swap or replace.

use super::reset_button;
use crate::editor::Editor;
use crate::prefs::KeySet;
use crate::ui::actions::Action;
use crate::ui::actions::keymap::{self, Keymap};
use crate::ui::actions::keys::{Chord, chord_of};
use crate::ui::theme;
use crate::ui::widgets::dim;
use bevy_egui::egui::{self, Key, RichText};

#[derive(Default)]
pub struct HotkeysUi {
    filter: String,
    /// Waiting for a key for this action.
    capture: Option<Action>,
    /// A key that is already used: (action, new key, current owner).
    pending: Option<(Action, Chord, Action)>,
    note: String,
}

impl HotkeysUi {
    pub fn cancel(&mut self) {
        self.capture = None;
        self.pending = None;
    }
}

/// Saves `map` as the live keymap and into Preferences.
fn commit(ed: &mut Editor, map: Keymap) {
    ed.prefs.keys.bindings = Some(keymap::to_prefs(&map));
    keymap::set_current(map);
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut HotkeysUi) {
    let mut map = keymap::current().clone();
    let set = ed.prefs.keys.set;
    ui.horizontal(|ui| {
        ui.label("Key set");
        egui::ComboBox::from_id_salt("key_set").selected_text(set.label()).show_ui(ui, |ui| {
            for s in KeySet::ALL {
                if ui.selectable_label(set == s, s.label()).clicked() && s != set {
                    ed.prefs.keys.set = s;
                    ed.prefs.keys.bindings = None;
                    keymap::set_current(keymap::for_set(s));
                    st.note = format!("Switched to the {} keys", s.label());
                }
            }
        });
        if reset_button(ui, "hotkeys of this key set") {
            ed.prefs.keys.bindings = None;
            keymap::set_current(keymap::for_set(set));
            st.note = "Every hotkey is back to its default".into();
        }
        if ui.button("Export...").on_hover_text("Save your keymap to a file").clicked() {
            export(ed, &map, st);
        }
        if ui.button("Import...").on_hover_text("Load a keymap file").clicked() {
            import(ed, st);
        }
    });
    capture(ui, ed, st, &mut map);
    if let Some((a, c, other)) = st.pending {
        egui::Frame::new().fill(theme::card()).corner_radius(4).inner_margin(6).show(ui, |ui| {
            ui.label(RichText::new(format!("{} is used by {}.", c.text(), other.label())).color(theme::amber()));
            ui.horizontal(|ui| {
                let old = chord_of(&map, a).map_or("no key".to_string(), |k| k.text());
                if ui.button("Swap").on_hover_text(format!("{} gets {old}", other.label())).clicked() {
                    keymap::assign(&mut map, a, c, true);
                    commit(ed, map.clone());
                    st.pending = None;
                }
                if ui.button("Replace").on_hover_text(format!("{} is left without a key", other.label())).clicked() {
                    keymap::assign(&mut map, a, c, false);
                    commit(ed, map.clone());
                    st.pending = None;
                }
                if ui.button("Cancel").clicked() {
                    st.pending = None;
                }
            });
        });
    }
    if !st.note.is_empty() {
        ui.label(dim(&st.note));
    }
    ui.add(egui::TextEdit::singleline(&mut st.filter).hint_text("Find an action or key…").desired_width(f32::INFINITY));
    let filter = st.filter.to_lowercase();
    egui::Grid::new("hotkey_editor").num_columns(3).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
        for a in Action::all() {
            let key = chord_of(&map, a);
            let key_text = key.map(|c| c.text()).unwrap_or_default();
            if !filter.is_empty() && !a.label().to_lowercase().contains(&filter) && !key_text.to_lowercase().contains(&filter) {
                continue;
            }
            ui.label(a.label()).on_hover_text(a.tip());
            let waiting = st.capture == Some(a);
            let caption = if waiting {
                RichText::new("Press a key… (Esc cancels)").strong()
            } else if key_text.is_empty() {
                dim("—  click to set")
            } else {
                RichText::new(&key_text).color(theme::amber())
            };
            if ui.add_sized([180.0, 20.0], egui::Button::selectable(waiting, caption)).on_hover_text("Click, then press the new key").clicked() {
                st.capture = if waiting { None } else { Some(a) };
                st.pending = None;
            }
            ui.horizontal(|ui| {
                if ui.small_button("↺").on_hover_text("Default key").clicked() {
                    keymap::reset_one(&mut map, set, a);
                    commit(ed, map.clone());
                }
                if ui.add_enabled(key.is_some(), egui::Button::new("✖").small()).on_hover_text("No key").clicked() {
                    keymap::clear(&mut map, a);
                    commit(ed, map.clone());
                }
            });
            ui.end_row();
        }
    });
    ed.iface.capturing_key = st.capture.is_some();
}

/// Reads the next key press for the action waiting in `st.capture`.
fn capture(ui: &mut egui::Ui, ed: &mut Editor, st: &mut HotkeysUi, map: &mut Keymap) {
    let Some(a) = st.capture else { return };
    let pressed = ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } => Some(Chord::from_event(*key, *modifiers)),
            _ => None,
        })
    });
    let Some(c) = pressed else { return };
    st.capture = None;
    if c == Chord::plain(Key::Escape) && a != Action::Cancel {
        st.note = "Cancelled".into();
        return;
    }
    match keymap::conflict(map, a, c) {
        Some(other) => st.pending = Some((a, c, other)),
        None => {
            keymap::assign(map, a, c, false);
            commit(ed, map.clone());
            st.note = format!("{} is now {}", a.label(), c.text());
        }
    }
}

fn export(ed: &mut Editor, map: &Keymap, st: &mut HotkeysUi) {
    let d = rfd::FileDialog::new().set_file_name("minimax-keys.ron").add_filter("Keymap (*.ron)", &["ron"]);
    if let Some(p) = d.save_file() {
        st.note = match std::fs::write(&p, keymap::export_text(map)) {
            Ok(()) => format!("Saved {}", p.display()),
            Err(e) => format!("Could not save: {e}"),
        };
        ed.status = st.note.clone();
    }
}

fn import(ed: &mut Editor, st: &mut HotkeysUi) {
    let Some(p) = rfd::FileDialog::new().add_filter("Keymap (*.ron)", &["ron"]).pick_file() else { return };
    let r = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| keymap::import_text(&t, ed.prefs.keys.set));
    st.note = match r {
        Ok(map) => {
            commit(ed, map);
            format!("Loaded {}", p.display())
        }
        Err(e) => format!("Could not load: {e}"),
    };
}
