//! Keyboard chords and the default 3ds Max keymap. Hotkeys are only
//! shortcuts: every action they trigger also has a button or menu entry.

use super::Action;
use crate::editor::NavMode;
use crate::mesh::SubObject;
use bevy_egui::egui::{Key, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub key: Key,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Chord {
    pub const fn plain(key: Key) -> Self {
        Self { key, ctrl: false, shift: false, alt: false }
    }
    pub const fn ctrl(key: Key) -> Self {
        Self { key, ctrl: true, shift: false, alt: false }
    }
    pub const fn shift(key: Key) -> Self {
        Self { key, ctrl: false, shift: true, alt: false }
    }
    pub const fn alt(key: Key) -> Self {
        Self { key, ctrl: false, shift: false, alt: true }
    }
    pub const fn ctrl_shift(key: Key) -> Self {
        Self { key, ctrl: true, shift: true, alt: false }
    }

    pub fn from_event(key: Key, m: Modifiers) -> Self {
        Self { key, ctrl: m.ctrl || m.command, shift: m.shift, alt: m.alt }
    }

    /// "Ctrl+Shift+S", "Alt+W", "F9", "Space".
    pub fn text(self) -> String {
        let mut s = String::new();
        for (on, name) in [(self.ctrl, "Ctrl+"), (self.alt, "Alt+"), (self.shift, "Shift+")] {
            if on {
                s.push_str(name);
            }
        }
        s.push_str(self.key.symbol_or_name());
        s
    }
}

/// The MiniMax default keymap (3ds Max conventions, never Blender's).
pub fn default_keymap() -> Vec<(Chord, Action)> {
    use Action::*;
    use Key::*;
    let mut v = vec![
        (Chord::ctrl(N), NewScene),
        (Chord::ctrl(O), OpenScene),
        (Chord::ctrl(S), SaveScene),
        (Chord::ctrl_shift(S), SaveSceneAs),
        (Chord::ctrl(Z), Undo),
        (Chord::ctrl(Y), Redo),
        (Chord::plain(Key::Delete), Action::Delete),
        (Chord::ctrl(A), SelectAll),
        (Chord::ctrl(D), SelectNone),
        (Chord::ctrl(I), SelectInvert),
        (Chord::plain(H), SceneExplorer),
        (Chord::plain(Space), LockSelection),
        (Chord::plain(Escape), Cancel),
        (Chord::plain(Q), SelectObject),
        (Chord::plain(W), Move),
        (Chord::plain(E), Rotate),
        (Chord::plain(R), Scale),
        (Chord::plain(S), Snap),
        (Chord::plain(A), AngleSnap),
        (Chord::ctrl_shift(Key::P), PercentSnap),
        (Chord::alt(R), SelectRing),
        (Chord::alt(L), SelectLoop),
        (Chord::plain(Backspace), RemoveSubObjects),
        (Chord::alt(A), Align),
        (Chord::alt(Q), Isolate),
        (Chord::plain(J), JointTool),
        (Chord::alt(X), SeeThrough),
        (Chord::plain(G), ToggleGrid),
        (Chord::plain(F3), Wireframe),
        (Chord::plain(F4), EdgedFaces),
        (Chord::plain(Z), ZoomExtents),
        (Chord::shift(Z), ZoomExtentsAll),
        (Chord::alt(W), Maximize),
        (Chord::plain(C), CameraView),
        (Chord::plain(Key::F9), Render),
        (Chord::plain(F10), RenderSetup),
        (Chord::plain(M), MaterialEditor),
        (Chord::plain(Num8), Environment),
        (Chord::ctrl(L), DefaultLights),
        (Chord::ctrl(Key::P), Nav(NavMode::Pan)),
        (Chord::ctrl(Key::R), Nav(NavMode::Orbit)),
    ];
    let subs = [SubObject::Vertex, SubObject::Edge, SubObject::Border, SubObject::Polygon, SubObject::Element, SubObject::Object];
    for (k, s) in [Num1, Num2, Num3, Num4, Num5, Num6].into_iter().zip(subs) {
        v.push((Chord::plain(k), Sub(s)));
    }
    v
}

/// The first chord bound to `action`.
pub fn chord_of(map: &[(Chord, Action)], action: Action) -> Option<Chord> {
    map.iter().find(|(_, a)| *a == action).map(|(c, _)| *c)
}

/// The action bound to `chord`.
pub fn action_of(map: &[(Chord, Action)], chord: Chord) -> Option<Action> {
    map.iter().find(|(c, _)| *c == chord).map(|(_, a)| *a)
}
