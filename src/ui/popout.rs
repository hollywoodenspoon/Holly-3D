//! Pop-out windows: any big dialog can leave the main window and live in
//! its own OS window (so a tiling window manager can place it). A dialog
//! draws through [`window`]; in the main window it is an `egui::Window`
//! with a ↗ button in its title bar, popped out it fills its own window.
//! Closing that OS window docks the dialog back. The popped set is saved
//! in `prefs.layout.popouts`.

use bevy_egui::egui::{self, Ui, UiBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// A dialog that can pop out into its own OS window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dock {
    MaterialEditor,
    MaterialLibrary,
    FrameWindow,
    UvEditor,
    CharacterStudio,
    FaceCloner,
    BuildingsBrowser,
    PaintBox,
    Help,
    Preferences,
}

impl Dock {
    pub const ALL: [Dock; 10] = [
        Dock::MaterialEditor,
        Dock::MaterialLibrary,
        Dock::FrameWindow,
        Dock::UvEditor,
        Dock::CharacterStudio,
        Dock::FaceCloner,
        Dock::BuildingsBrowser,
        Dock::PaintBox,
        Dock::Help,
        Dock::Preferences,
    ];

    /// Saved name in preferences, also the end of the window's app id.
    pub fn key(self) -> &'static str {
        match self {
            Dock::MaterialEditor => "material-editor",
            Dock::MaterialLibrary => "material-library",
            Dock::FrameWindow => "frame-window",
            Dock::UvEditor => "uv-editor",
            Dock::CharacterStudio => "character-studio",
            Dock::FaceCloner => "face-cloner",
            Dock::BuildingsBrowser => "buildings-browser",
            Dock::PaintBox => "paintbox",
            Dock::Help => "help",
            Dock::Preferences => "preferences",
        }
    }

    pub fn from_key(k: &str) -> Option<Dock> {
        Dock::ALL.into_iter().find(|d| d.key() == k)
    }

    pub fn title(self) -> &'static str {
        match self {
            Dock::MaterialEditor => "Material Editor",
            Dock::MaterialLibrary => "Material Library",
            Dock::FrameWindow => "Rendered Frame Window",
            Dock::UvEditor => "Edit UVWs",
            Dock::CharacterStudio => "MakeHuman",
            Dock::FaceCloner => "Face Cloner",
            Dock::BuildingsBrowser => "Buildings & Props Browser",
            Dock::PaintBox => "PaintBox",
            Dock::Help => "Help",
            Dock::Preferences => "Preferences",
        }
    }

    /// Wayland app id / X11 class (for window-manager rules).
    pub fn app_id(self) -> String {
        format!("ilex-{}", self.key())
    }

    /// First size of its OS window, in logical pixels.
    pub fn size(self) -> (u32, u32) {
        match self {
            Dock::MaterialEditor => (560, 860),
            Dock::MaterialLibrary => (760, 600),
            Dock::FrameWindow => (900, 740),
            Dock::UvEditor => (700, 660),
            Dock::CharacterStudio => (420, 720),
            Dock::FaceCloner => (460, 720),
            Dock::BuildingsBrowser => (680, 560),
            Dock::PaintBox => (300, 720),
            Dock::Help => (1000, 720),
            Dock::Preferences => (600, 600),
        }
    }
}

/// Which window is being drawn right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Main,
    Pop(Dock),
}

#[derive(Default)]
struct State {
    host: Option<Host>,
    popped: HashSet<Dock>,
    /// Open and popped this frame: these need an OS window.
    want: HashSet<Dock>,
    /// Where each dialog was drawn last, to notice moves.
    drawn_in: HashMap<Dock, Host>,
    changed: bool,
    loaded: bool,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(State::default))
}

/// Popped docks from their saved names (unknown names are dropped).
pub fn parse(keys: &[String]) -> HashSet<Dock> {
    keys.iter().filter_map(|k| Dock::from_key(k)).collect()
}

/// Saved names, in a stable order.
pub fn keys(set: &HashSet<Dock>) -> Vec<String> {
    Dock::ALL.into_iter().filter(|d| set.contains(d)).map(|d| d.key().to_string()).collect()
}

/// Start of the main window's pass.
pub fn begin_main(saved: &[String]) {
    with(|s| {
        s.host = Some(Host::Main);
        if !s.loaded {
            s.popped = parse(saved);
            s.loaded = true;
        }
        s.want.clear();
    });
}

/// End of the main pass: the new saved list when it changed, and the docks
/// that need an OS window.
pub fn end_main() -> (Option<Vec<String>>, HashSet<Dock>) {
    with(|s| {
        s.host = None;
        let saved = std::mem::take(&mut s.changed).then(|| keys(&s.popped));
        (saved, s.want.clone())
    })
}

pub fn set_host(h: Option<Host>) {
    with(|s| s.host = h);
}

/// Docks a dialog back (its OS window was closed or "Dock" was clicked).
pub fn dock_back(d: Dock) {
    with(|s| {
        s.changed |= s.popped.remove(&d);
    });
}

/// True when dialog `d` is about to be drawn in `h` after being drawn in
/// another window: its cached textures belong to that window and must go.
pub fn arriving(d: Dock, h: Host) -> bool {
    with(|s| {
        let goes_here = match h {
            Host::Main => !s.popped.contains(&d),
            Host::Pop(p) => p == d,
        };
        goes_here && s.drawn_in.get(&d).is_some_and(|old| *old != h)
    })
}

/// Is `d` drawn in the window being drawn now? (For a dialog's side windows.)
pub fn here(d: Dock) -> bool {
    with(|s| match s.host {
        Some(Host::Pop(p)) => p == d,
        _ => !s.popped.contains(&d),
    })
}

/// Is the main window being drawn?
pub fn is_main() -> bool {
    with(|s| !matches!(s.host, Some(Host::Pop(_))))
}

fn note_drawn(s: &mut State, d: Dock, h: Host) {
    s.drawn_in.insert(d, h);
}

/// Draws dialog `dock`: as `win` in the main window, or filling its own
/// OS window when popped out. Call it only while the dialog is open.
pub fn window<'a, R>(ctx: &egui::Context, dock: Dock, win: egui::Window<'a>, open: &'a mut bool, body: impl FnOnce(&mut Ui) -> R) {
    let host = with(|s| s.host.unwrap_or(Host::Main));
    match host {
        Host::Main => {
            if with(|s| s.popped.contains(&dock)) {
                with(|s| s.want.insert(dock));
                return;
            }
            with(|s| note_drawn(s, dock, Host::Main));
            let Some(resp) = win.open(open).show(ctx, body) else { return };
            let r = resp.response.rect;
            let btn = egui::Rect::from_min_size(egui::pos2(r.max.x - 52.0, r.min.y + 4.0), egui::vec2(22.0, 20.0));
            let mut ui = Ui::new(ctx.clone(), egui::Id::new(("pop_btn", dock.key())), UiBuilder::new().layer_id(resp.response.layer_id).max_rect(btn));
            let b = ui.add_sized(btn.size(), egui::Button::new("↗").frame(false));
            if b.on_hover_text("Pop out into its own window").clicked() {
                with(|s| {
                    s.popped.insert(dock);
                    s.changed = true;
                });
            }
        }
        Host::Pop(d) if d == dock => {
            with(|s| note_drawn(s, dock, host));
            let builder = UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.viewport_rect());
            let mut root = Ui::new(ctx.clone(), egui::Id::new(("pop_root", dock.key())), builder);
            egui::CentralPanel::default().show(&mut root, |ui| {
                let top = ui.max_rect().right_top();
                body(ui);
                let btn = egui::Rect::from_min_size(top + egui::vec2(-26.0, -2.0), egui::vec2(24.0, 20.0));
                let back = ui.put(btn, egui::Button::new("↙").frame(false));
                if back.on_hover_text("Dock back into the main window").clicked() {
                    dock_back(dock);
                }
            });
        }
        Host::Pop(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_names_are_unique_and_round_trip() {
        let mut seen = HashSet::new();
        for d in Dock::ALL {
            assert!(seen.insert(d.key()), "{d:?}");
            assert_eq!(Dock::from_key(d.key()), Some(d));
            assert!(d.app_id().starts_with("ilex-"));
        }
        let set: HashSet<Dock> = [Dock::Help, Dock::MaterialEditor].into();
        let saved = keys(&set);
        assert_eq!(saved, vec!["material-editor".to_string(), "help".to_string()]);
        let mut back = saved.clone();
        back.push("no-such-window".into());
        assert_eq!(parse(&back), set, "unknown names are ignored");
    }
}
