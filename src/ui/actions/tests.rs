use super::keys::{Chord, action_of, chord_of, default_keymap};
use super::layout::{TOOLBAR_LEFT, TOOLBAR_RIGHT, placed_actions};
use super::*;
use crate::editor::Editor;
use crate::primitives::PrimitiveKind;
use bevy::math::Vec3;
use bevy_egui::egui::{self, Key};

/// "A button for everything": every action has a menu entry or a button.
#[test]
fn every_action_has_a_button() {
    let placed = placed_actions();
    let missing: Vec<_> = Action::all().into_iter().filter(|a| !placed.contains(a)).collect();
    assert!(missing.is_empty(), "actions with no button or menu entry: {missing:?}");
}

#[test]
fn every_hotkey_action_is_listed_and_chords_are_unique() {
    let map = default_keymap();
    let all = Action::all();
    for (i, (c, a)) in map.iter().enumerate() {
        assert!(all.contains(a), "{a:?} is bound but missing from Action::all()");
        assert!(map[i + 1..].iter().all(|(c2, _)| c2 != c), "{} is bound twice", c.text());
    }
    assert_eq!(action_of(&map, Chord::plain(Key::W)), Some(Action::Move));
    assert_eq!(action_of(&map, Chord::shift(Key::Z)), Some(Action::ZoomExtentsAll));
    assert_eq!(chord_of(&map, Action::SaveSceneAs).map(Chord::text).as_deref(), Some("Ctrl+Shift+S"));
    assert_eq!(action_of(&map, Chord::alt(Key::G)), None);
}

#[test]
fn all_actions_are_unique_and_labelled() {
    let all = Action::all();
    for (i, a) in all.iter().enumerate() {
        assert!(!all[i + 1..].contains(a), "{a:?} listed twice");
        assert!(!a.label().is_empty() && !a.short().is_empty(), "{a:?} has no label");
    }
}

/// Whether the bundled fonts draw `c` (egui's `has_glyph` wrongly says no for
/// every glyph in the emoji font, so compare against the replacement glyph).
fn drawable(ctx: &egui::Context, c: char) -> bool {
    let uv = |ch: char| {
        ctx.fonts_mut(|f| {
            let g = f.layout_no_wrap(ch.to_string(), egui::FontId::proportional(14.0), egui::Color32::WHITE);
            g.rows.first().and_then(|r| r.row.glyphs.first()).map(|g| g.uv_rect)
        })
    };
    c.is_ascii() || uv(c) != uv('\u{E000}')
}

fn fonts() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
    ctx
}

/// Toolbar captions must not render as empty boxes.
#[test]
fn toolbar_glyphs_exist_in_the_ui_font() {
    let ctx = fonts();
    let ok = |s: &str| s.chars().all(|c| drawable(&ctx, c));
    let mut bad = Vec::new();
    for s in TOOLBAR_LEFT.iter().chain(TOOLBAR_RIGHT) {
        if let layout::Slot::A(a) = s {
            let text = a.short();
            if !ok(&text) {
                bad.push((*a, text));
            }
        }
    }
    for row in layout::NAV_BUTTONS {
        for a in row {
            if !ok(&a.short()) {
                bad.push((a, a.short()));
            }
        }
    }
    for p in SkyPreset::ALL {
        if !ok(p.icon()) {
            bad.push((Action::Sky(p), p.icon().into()));
        }
    }
    assert!(bad.is_empty(), "missing glyphs: {bad:?}");
}

/// Every action can run on an empty and on a populated scene without panicking.
#[test]
fn every_action_runs() {
    let skip = |a: Action| {
        // File dialogs block on a native window.
        matches!(a, Action::OpenScene | Action::SaveScene | Action::SaveSceneAs | Action::Import | Action::ExportAll | Action::ExportSelected)
    };
    for populated in [false, true] {
        let mut ed = Editor::default();
        if populated {
            ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        }
        for a in Action::all().into_iter().filter(|a| !skip(*a)) {
            let _ = a.enabled(&ed);
            let _ = a.checked(&ed);
            a.run(&mut ed);
        }
    }
}

#[test]
fn toggles_report_their_state() {
    let mut ed = Editor::default();
    assert_eq!(Action::PercentSnap.checked(&ed), Some(false));
    Action::PercentSnap.run(&mut ed);
    assert_eq!(Action::PercentSnap.checked(&ed), Some(true));
    Action::Rotate.run(&mut ed);
    assert_eq!((Action::Rotate.checked(&ed), Action::Move.checked(&ed)), (Some(true), Some(false)));
    assert_eq!(Action::Undo.checked(&ed), None);
    assert!(!Action::Clone.enabled(&ed), "nothing selected");
    Action::Sky(SkyPreset::Sunset).run(&mut ed);
    assert_eq!(Action::Sky(SkyPreset::Sunset).checked(&ed), Some(true));
    assert!(ed.env_open, "a sky preset opens the Environment window");
}


/// No string anywhere in the UI uses a symbol the bundled fonts cannot draw.
#[test]
fn ui_source_strings_only_use_drawable_glyphs() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let ctx = fonts();
    let mut files = Vec::new();
    walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"), &mut files);
    let mut bad = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // Only look inside string literals.
            for (i, part) in code.split('"').enumerate() {
                if i % 2 == 1 && part.chars().any(|c| !drawable(&ctx, c)) {
                    bad.push(format!("{}:{}: {part}", f.file_name().unwrap().to_string_lossy(), n + 1));
                }
            }
        }
    }
    assert!(bad.is_empty(), "strings with undrawable glyphs:\n{}", bad.join("\n"));
}

#[test]
fn glyph_check_tells_drawn_from_missing() {
    let ctx = fonts();
    assert!(drawable(&ctx, '🔍') && drawable(&ctx, '⟲') && drawable(&ctx, '✔'));
    assert!(!drawable(&ctx, '\u{E001}') && !drawable(&ctx, '\u{2BFF}'));
}
