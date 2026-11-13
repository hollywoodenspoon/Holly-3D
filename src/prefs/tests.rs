//! Preferences round trips, defaults and helpers.

use super::*;

#[test]
fn preferences_round_trip_through_ron() {
    let mut p = Preferences::default();
    p.theme.preset = ThemePreset::MidnightBlue;
    p.theme.ui_scale = 1.25;
    p.keys.bindings = Some(vec![("Move".into(), "W".into())]);
    p.viewport.invert_zoom = true;
    p.general.undo_steps = 200;
    p.general.units = Units::Feet;
    p.render.width = 1920;
    assert_eq!(Preferences::from_ron(&p.to_ron()).unwrap(), p);
}

#[test]
fn save_and_load_from_disk() {
    let dir = std::env::temp_dir().join(format!("ilex_prefs_{}", std::process::id()));
    let path = dir.join("preferences.ron");
    let mut p = Preferences::default();
    p.general.autosave_minutes = 12;
    p.save_to(&path).unwrap();
    assert_eq!(Preferences::load_from(&path).unwrap(), p);
    assert_eq!(Preferences::load_from(&dir.join("missing.ron")).unwrap(), Preferences::default());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn legacy_minimax_dirs_are_kept_until_ilex_exists() {
    let base = std::env::temp_dir().join(format!("ilex_dirs_{}", std::process::id()));
    std::fs::create_dir_all(base.join("minimax")).unwrap();
    assert_eq!(app_dir(&base), base.join("minimax"));
    std::fs::create_dir_all(base.join("ilex")).unwrap();
    assert_eq!(app_dir(&base), base.join("ilex"));
    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn partial_files_fill_in_defaults() {
    let p = Preferences::from_ron("(general: (undo_steps: 10))").unwrap();
    assert_eq!(p.general.undo_steps, 10);
    assert_eq!(p.general.autosave_minutes, GeneralPrefs::default().autosave_minutes);
    assert_eq!(p.theme, ThemePrefs::default());
    assert!(Preferences::from_ron("(general: oops)").is_err());
}

#[test]
fn units_and_zoom_helpers() {
    assert_eq!(Units::Centimeters.format(2.5), "250.0cm");
    assert_eq!(Units::Feet.format(0.3048), "1.000ft");
    let mut v = ViewportPrefs { zoom_speed: 2.0, ..Default::default() };
    assert_eq!(v.zoom_steps(1.0), 2.0);
    v.invert_zoom = true;
    assert_eq!(v.zoom_steps(1.0), -2.0);
}
