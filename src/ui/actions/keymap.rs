//! The live, editable keymap (Preferences -> Hotkeys): rebinding with
//! conflict handling, key sets, and the text form saved in
//! `preferences.ron` and keymap exports.

use super::Action;
use super::keys::{Chord, chord_of, default_keymap};
use crate::prefs::{KeyPrefs, KeySet};
use bevy_egui::egui::Key;
use std::sync::{RwLock, RwLockReadGuard};

pub type Keymap = Vec<(Chord, Action)>;

static LIVE: RwLock<Keymap> = RwLock::new(Vec::new());

/// The keymap in use (the defaults until Preferences load).
pub fn current() -> RwLockReadGuard<'static, Keymap> {
    {
        let r = LIVE.read().unwrap_or_else(|e| e.into_inner());
        if !r.is_empty() {
            return r;
        }
    }
    set_current(default_keymap());
    LIVE.read().unwrap_or_else(|e| e.into_inner())
}

pub fn set_current(map: Keymap) {
    *LIVE.write().unwrap_or_else(|e| e.into_inner()) = map;
}

/// A key set's keymap: Ilex's, or the Classic key set.
pub fn for_set(set: KeySet) -> Keymap {
    let mut map = default_keymap();
    if set == KeySet::Max {
        use Action::*;
        map.retain(|(c, a)| {
            !matches!(a, SceneExplorer | ZoomExtentsAll | Sub(crate::mesh::SubObject::Object)) && *c != Chord::plain(Key::H)
        });
        map.extend([
            (Chord::plain(Key::H), SelectByName),
            (Chord::shift(Key::Q), Render),
            (Chord::alt(Key::Z), Nav(crate::editor::NavMode::Zoom)),
            (Chord::ctrl_shift(Key::Z), ZoomExtentsAll),
        ]);
    }
    map
}

impl Chord {
    /// Stable text for saving: "Ctrl+Shift+S", "Alt+W", "F9", "Num8".
    pub fn id(self) -> String {
        let mut s = String::new();
        for (on, name) in [(self.ctrl, "Ctrl+"), (self.alt, "Alt+"), (self.shift, "Shift+")] {
            if on {
                s.push_str(name);
            }
        }
        s.push_str(self.key.name());
        s
    }

    pub fn parse(text: &str) -> Option<Chord> {
        let mut c = Chord::plain(Key::from_name(text.rsplit('+').next()?.trim())?);
        for part in text.split('+').rev().skip(1) {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" => c.ctrl = true,
                "alt" => c.alt = true,
                "shift" => c.shift = true,
                _ => return None,
            }
        }
        Some(c)
    }
}

impl Action {
    /// Stable text for saving, e.g. "Move", "Sub(Vertex)", "Sky(Night)".
    pub fn id(self) -> String {
        format!("{self:?}")
    }
    pub fn from_id(id: &str) -> Option<Action> {
        Action::all().into_iter().find(|a| a.id() == id)
    }
}

/// The keymap described by Preferences: the saved bindings (unknown entries
/// are skipped), plus defaults for newer actions whose key is still free.
pub fn from_prefs(p: &KeyPrefs) -> Keymap {
    let base = for_set(p.set);
    let Some(saved) = &p.bindings else { return base };
    let mut map: Keymap = vec![];
    for (a, c) in saved {
        if let (Some(a), Some(c)) = (Action::from_id(a), Chord::parse(c))
            && !map.iter().any(|(mc, _)| *mc == c)
        {
            map.push((c, a));
        }
    }
    let known: Vec<&str> = saved.iter().map(|(a, _)| a.as_str()).collect();
    for (c, a) in base {
        if !known.contains(&a.id().as_str()) && !map.iter().any(|(mc, _)| *mc == c) {
            map.push((c, a));
        }
    }
    map
}

/// The text form saved in `preferences.ron`. Actions left without a key are
/// saved as ("Action", "") so a cleared key stays cleared.
pub fn to_prefs(map: &Keymap) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = map.iter().map(|(c, a)| (a.id(), c.id())).collect();
    for a in Action::all() {
        if chord_of(map, a).is_none() {
            v.push((a.id(), String::new()));
        }
    }
    v
}

/// The other action already using `chord`, if any.
pub fn conflict(map: &Keymap, action: Action, chord: Chord) -> Option<Action> {
    map.iter().find(|(c, a)| *c == chord && *a != action).map(|(_, a)| *a)
}

/// Binds `chord` to `action` (replacing its old key). A conflicting action
/// either takes over `action`'s old key (`swap`) or loses its key.
pub fn assign(map: &mut Keymap, action: Action, chord: Chord, swap: bool) {
    let old = chord_of(map, action);
    let other = conflict(map, action, chord);
    map.retain(|(c, a)| *a != action && *c != chord);
    if let (true, Some(other), Some(old)) = (swap, other, old) {
        map.push((old, other));
    }
    map.push((chord, action));
}

/// Leaves `action` without a hotkey.
pub fn clear(map: &mut Keymap, action: Action) {
    map.retain(|(_, a)| *a != action);
}

/// Puts `action` back on its key-set default (taking it from whoever has it).
pub fn reset_one(map: &mut Keymap, set: KeySet, action: Action) {
    match chord_of(&for_set(set), action) {
        Some(c) => assign(map, action, c, false),
        None => clear(map, action),
    }
}

/// Keymap export file contents (RON list of pairs).
pub fn export_text(map: &Keymap) -> String {
    ron::ser::to_string_pretty(&to_prefs(map), ron::ser::PrettyConfig::default()).unwrap_or_default()
}

pub fn import_text(text: &str, set: KeySet) -> Result<Keymap, String> {
    let pairs: Vec<(String, String)> = ron::from_str(text).map_err(|e| format!("Keymap: {e}"))?;
    Ok(from_prefs(&KeyPrefs { set, bindings: Some(pairs) }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::NavMode;

    #[test]
    fn chords_and_actions_round_trip_as_text() {
        for c in [Chord::ctrl_shift(Key::S), Chord::alt(Key::W), Chord::plain(Key::F9), Chord::plain(Key::Num8)] {
            assert_eq!(Chord::parse(&c.id()), Some(c), "{}", c.id());
        }
        assert_eq!(Chord::parse("Ctrl+Bogus"), None);
        assert_eq!(Chord::parse("Hyper+W"), None);
        for a in Action::all() {
            assert_eq!(Action::from_id(&a.id()), Some(a));
        }
    }

    #[test]
    fn rebinding_moves_a_key_and_catches_conflicts() {
        let mut map = default_keymap();
        // F is free: Move goes to F, W is now free.
        assign(&mut map, Action::Move, Chord::plain(Key::F), false);
        assert_eq!(chord_of(&map, Action::Move), Some(Chord::plain(Key::F)));
        assert_eq!(conflict(&map, Action::Rotate, Chord::plain(Key::W)), None);
        // E is Rotate's: a conflict. Swapping hands Rotate Move's old key F.
        assert_eq!(conflict(&map, Action::Move, Chord::plain(Key::E)), Some(Action::Rotate));
        assign(&mut map, Action::Move, Chord::plain(Key::E), true);
        assert_eq!(chord_of(&map, Action::Rotate), Some(Chord::plain(Key::F)));
        // Replacing instead leaves Rotate without a key.
        assign(&mut map, Action::Scale, Chord::plain(Key::F), false);
        assert_eq!(chord_of(&map, Action::Rotate), None);
        reset_one(&mut map, KeySet::MiniMax, Action::Rotate);
        assert_eq!(chord_of(&map, Action::Rotate), Some(Chord::plain(Key::E)));
        assert_eq!(chord_of(&map, Action::Move), None, "E was taken back from Move");
        let mut ids: Vec<_> = map.iter().map(|(c, _)| c.id()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "chords stay unique");
    }

    #[test]
    fn saved_keymaps_reload_and_keep_cleared_keys() {
        let mut map = default_keymap();
        assign(&mut map, Action::Move, Chord::plain(Key::F), false);
        clear(&mut map, Action::Snap);
        let prefs = KeyPrefs { set: KeySet::MiniMax, bindings: Some(to_prefs(&map)) };
        let back = from_prefs(&prefs);
        assert_eq!(chord_of(&back, Action::Move), Some(Chord::plain(Key::F)));
        assert_eq!(chord_of(&back, Action::Snap), None);
        assert_eq!(back.len(), map.len());
        assert_eq!(chord_of(&import_text(&export_text(&map), KeySet::MiniMax).unwrap(), Action::Move), Some(Chord::plain(Key::F)));
        assert!(import_text("nonsense", KeySet::MiniMax).is_err());
    }

    #[test]
    fn new_actions_get_their_default_keys_in_old_saves() {
        let prefs = KeyPrefs { set: KeySet::MiniMax, bindings: Some(vec![("Move".into(), "W".into())]) };
        let map = from_prefs(&prefs);
        assert_eq!(chord_of(&map, Action::Rotate), Some(Chord::plain(Key::E)));
    }

    #[test]
    fn max_key_set_differs_and_stays_unique() {
        let map = for_set(KeySet::Max);
        assert_eq!(chord_of(&map, Action::SelectByName), Some(Chord::plain(Key::H)));
        assert_eq!(chord_of(&map, Action::Nav(NavMode::Zoom)), Some(Chord::alt(Key::Z)));
        assert_eq!(chord_of(&map, Action::SceneExplorer), None);
        let mut ids: Vec<_> = map.iter().map(|(c, _)| c.id()).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
