//! User Preferences (toolbar ⚙ or Edit -> Preferences): theme, hotkeys,
//! viewports, general, rendering and asset settings. Saved as RON to
//! `~/.config/minimax/preferences.ron` and kept between sessions. Every
//! field has a default, so older or hand-edited files still load.

pub mod palette;

use crate::render::{Engine, RenderSettings};
pub use palette::{Palette, Rgb, ThemePreset};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: ThemePrefs,
    pub keys: KeyPrefs,
    pub viewport: ViewportPrefs,
    pub general: GeneralPrefs,
    pub render: RenderPrefs,
    pub assets: AssetPrefs,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemePrefs {
    pub preset: ThemePreset,
    /// The colours used when `preset` is Custom.
    pub custom: Palette,
    /// Interface size, 0.9–1.5 (1.0 = 100 %).
    pub ui_scale: f32,
    /// Body text size in points.
    pub font_size: f32,
}

impl Default for ThemePrefs {
    fn default() -> Self {
        Self { preset: ThemePreset::SlateAmber, custom: Palette::default(), ui_scale: 1.0, font_size: 12.5 }
    }
}

impl ThemePrefs {
    pub fn palette(&self) -> Palette {
        if self.preset == ThemePreset::Custom { self.custom } else { self.preset.palette() }
    }
}

/// Built-in key sets for Preferences -> Hotkeys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum KeySet {
    #[default]
    MiniMax,
    /// Closer to 3ds Max's own Default UI keyboard shortcuts.
    Max,
}

impl KeySet {
    pub const ALL: [KeySet; 2] = [KeySet::MiniMax, KeySet::Max];
    pub fn label(self) -> &'static str {
        match self {
            KeySet::MiniMax => "MiniMax default",
            KeySet::Max => "3ds Max standard",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyPrefs {
    pub set: KeySet,
    /// The whole keymap as (action, chord) text pairs, e.g. ("Move", "W").
    /// `None` uses the key set unchanged.
    pub bindings: Option<Vec<(String, String)>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewportPrefs {
    /// Viewports show the Environment (8) background colour.
    pub use_env_background: bool,
    /// Viewport background when `use_env_background` is off (sRGB 0–1).
    pub background: [f32; 3],
    pub grid: [f32; 3],
    pub grid_axis: [f32; 3],
    pub vertex: Rgb,
    pub vertex_selected: Rgb,
    pub invert_zoom: bool,
    pub zoom_speed: f32,
    pub orbit_speed: f32,
    pub pan_speed: f32,
}

impl Default for ViewportPrefs {
    fn default() -> Self {
        Self {
            use_env_background: true,
            background: [0.235, 0.24, 0.255],
            grid: [0.32, 0.33, 0.35],
            grid_axis: [0.12, 0.12, 0.13],
            vertex: [0x3D, 0x8B, 0xFF],
            vertex_selected: [0xFF, 0x2E, 0x1F],
            invert_zoom: false,
            zoom_speed: 1.0,
            orbit_speed: 1.0,
            pan_speed: 1.0,
        }
    }
}

impl ViewportPrefs {
    /// Wheel / drag zoom steps after speed and Invert Zoom.
    pub fn zoom_steps(&self, raw: f32) -> f32 {
        raw * self.zoom_speed * if self.invert_zoom { -1.0 } else { 1.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Units {
    #[default]
    Meters,
    Centimeters,
    Feet,
}

impl Units {
    pub const ALL: [Units; 3] = [Units::Meters, Units::Centimeters, Units::Feet];
    pub fn label(self) -> &'static str {
        match self {
            Units::Meters => "Meters (m)",
            Units::Centimeters => "Centimeters (cm)",
            Units::Feet => "Feet (ft)",
        }
    }
    /// Shows a scene length (1 unit = 1 m) in these units: "2.50m", "250.0cm", "8.20ft".
    pub fn format(self, metres: f32) -> String {
        match self {
            Units::Meters => format!("{metres:.3}m"),
            Units::Centimeters => format!("{:.1}cm", metres * 100.0),
            Units::Feet => format!("{:.3}ft", metres / 0.3048),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralPrefs {
    /// Autosave every N minutes (0 = off).
    pub autosave_minutes: u32,
    /// Undo steps kept (Ctrl+Z).
    pub undo_steps: u32,
    pub units: Units,
    /// Show the Getting Started tour on startup (Help).
    pub show_tour: bool,
}

impl Default for GeneralPrefs {
    fn default() -> Self {
        Self { autosave_minutes: 5, undo_steps: 64, units: Units::Meters, show_tour: true }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderPrefs {
    pub engine: Engine,
    pub blender_exe: String,
    pub width: u32,
    pub height: u32,
}

impl Default for RenderPrefs {
    fn default() -> Self {
        let r = RenderSettings::default();
        Self { engine: r.engine, blender_exe: r.blender_exe, width: r.width, height: r.height }
    }
}

impl RenderPrefs {
    /// Copies the defaults into a scene's Render Setup.
    pub fn apply(&self, r: &mut RenderSettings) {
        r.engine = self.engine;
        r.blender_exe = self.blender_exe.clone();
        r.width = self.width.max(1);
        r.height = self.height.max(1);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AssetPrefs {
    /// Where downloaded material and model packs go.
    pub folder: String,
    pub photos: bool,
    pub lowpoly: bool,
    pub realistic: bool,
}

impl Default for AssetPrefs {
    fn default() -> Self {
        let folder = data_dir().join("assets").display().to_string();
        Self { folder, photos: true, lowpoly: true, realistic: false }
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|| ".".into())
}

/// `$XDG_CONFIG_HOME/minimax` (usually `~/.config/minimax`).
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute());
    base.unwrap_or_else(|| home().join(".config")).join("minimax")
}

/// `$XDG_DATA_HOME/minimax` (usually `~/.local/share/minimax`).
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute());
    base.unwrap_or_else(|| home().join(".local/share")).join("minimax")
}

impl Preferences {
    pub fn default_path() -> PathBuf {
        config_dir().join("preferences.ron")
    }

    pub fn from_ron(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("Preferences: {e}"))
    }

    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap_or_default()
    }

    /// Reads `path`; a missing file gives the defaults.
    pub fn load_from(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_ron(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Preferences: {e}")),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Preferences: {e}"))?;
        }
        std::fs::write(path, self.to_ron()).map_err(|e| format!("Preferences: {e}"))
    }
}

#[cfg(test)]
mod tests {
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
        let dir = std::env::temp_dir().join(format!("minimax_prefs_{}", std::process::id()));
        let path = dir.join("preferences.ron");
        let mut p = Preferences::default();
        p.general.autosave_minutes = 12;
        p.save_to(&path).unwrap();
        assert_eq!(Preferences::load_from(&path).unwrap(), p);
        assert_eq!(Preferences::load_from(&dir.join("missing.ron")).unwrap(), Preferences::default());
        std::fs::remove_dir_all(&dir).ok();
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
}
