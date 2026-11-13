//! Keyboard shortcuts (no modal G/R/S keys). Every hotkey is looked up in the action keymap, so each one also has a
//! button or menu entry.

use super::actions::keymap;
use super::actions::keys::{Chord, action_of};
use crate::editor::Editor;
use crate::mesh::ops::PolyOp;
use bevy_egui::egui::{self, Key};

pub fn handle(ctx: &egui::Context, ed: &mut Editor) {
    if ctx.egui_wants_keyboard_input() || ed.iface.capturing_key {
        return;
    }
    // Array / Mirror / Align dialogs own the keyboard: Esc cancels, Enter confirms.
    if ed.array_dialog.is_some() || ed.mirror_dialog.is_some() || ed.align_dialog.is_some() {
        let (esc, enter) = ctx.input(|i| (i.key_pressed(Key::Escape), i.key_pressed(Key::Enter)));
        if ed.array_dialog.is_some() {
            if esc {
                ed.cancel_array();
            } else if enter {
                ed.confirm_array();
            }
        } else if ed.mirror_dialog.is_some() {
            if esc {
                ed.cancel_mirror();
            } else if enter {
                ed.confirm_mirror();
            }
        } else if esc {
            ed.cancel_align();
        } else if enter {
            ed.confirm_align();
        }
        return;
    }
    let (held, keys): (egui::Modifiers, Vec<(Key, egui::Modifiers)>) = ctx.input(|i| {
        let keys = i
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect();
        (i.modifiers, keys)
    });
    let map = keymap().clone();
    for (key, ev_mods) in keys {
        // Combine the event's modifiers with the held state (robust to event ordering).
        let m = egui::Modifiers {
            alt: ev_mods.alt || held.alt,
            ctrl: ev_mods.ctrl || held.ctrl,
            shift: ev_mods.shift || held.shift,
            ..ev_mods
        };
        let chord = Chord::from_event(key, m);
        // Ctrl+Backspace: Remove edges together with their orphaned vertices.
        if chord == Chord::ctrl(Key::Backspace) && ed.in_sub_object() && ed.sub.is_edge_like() {
            ed.apply_op(PolyOp::RemoveEdges { remove_verts: true });
            continue;
        }
        if let Some(action) = action_of(&map, chord).filter(|a| a.enabled(ed)) {
            crate::diagnostics::breadcrumbs::record("Hotkey", format!("{} -> {}", chord.text(), action.label()));
            action.run(ed);
        }
    }
}

/// The active keymap (Preferences -> Hotkeys edits it).
pub fn keymap() -> std::sync::RwLockReadGuard<'static, keymap::Keymap> {
    keymap::current()
}
