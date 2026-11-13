//! User Preferences (toolbar ⚙ or Edit -> Preferences): theme, hotkeys,
//! viewports, general, rendering and asset settings. Saved as RON to
//! `~/.config/ilex/preferences.ron` (a pre-rename `~/.config/minimax` is still used) and kept between sessions. Every
//! field has a default, so older or hand-edited files still load.

pub mod palette;

use crate::render::{ComputeDevice, Integrator, RenderSettings};
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
    pub layout: LayoutPrefs,
}

/// Sizes of the resizable panels, remembered between sessions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutPrefs {
    /// Width of the right Properties column in points.
    pub properties_width: f32,
    /// Height of the Scene Explorer at the top of that column.
    pub explorer_height: f32,
}

impl LayoutPrefs {
    pub const MIN_W: f32 = 260.0;
    pub const MAX_W: f32 = 640.0;
    pub const MIN_EXPLORER: f32 = 80.0;
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self { properties_width: 340.0, explorer_height: 200.0 }
    }
}

/// The interface font (Preferences > Theme).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiFont {
    /// Fira Code, built into the program (the removed Inter option loads as this).
    #[default]
    #[serde(alias = "Inter")]
    FiraCode,
    /// egui's own proportional font.
    Classic,
}

impl UiFont {
    pub const ALL: [UiFont; 2] = [UiFont::FiraCode, UiFont::Classic];
    pub fn label(self) -> &'static str {
        match self {
            UiFont::FiraCode => "Fira Code (default)",
            UiFont::Classic => "Classic (proportional)",
        }
    }
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
    /// The interface font.
    pub font: UiFont,
    /// Bumped when the default look changes; 0 in files saved before.
    #[serde(default)]
    pub look_version: u32,
}

impl Default for ThemePrefs {
    fn default() -> Self {
        Self { preset: ThemePreset::GraphiteTeal, custom: Palette::default(), ui_scale: 1.0, font_size: 11.5, font: UiFont::FiraCode, look_version: palette::LOOK_VERSION }
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
            KeySet::MiniMax => "Ilex default",
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
    /// Viewports show the Environment (8) background colour instead of the
    /// neutral viewport grey (renders always use the Environment colour).
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
    /// Orientation Cube in the top-right corner of every view.
    pub show_cube: bool,
    /// A cube click also frames the selection (Zoom Extents).
    pub cube_fit_on_change: bool,
}

impl Default for ViewportPrefs {
    fn default() -> Self {
        Self {
            use_env_background: false,
            background: [0.24, 0.24, 0.24],
            grid: [0.33, 0.33, 0.33],
            grid_axis: [0.13, 0.13, 0.13],
            vertex: [0x3D, 0x8B, 0xFF],
            vertex_selected: [0xFF, 0x2E, 0x1F],
            invert_zoom: false,
            zoom_speed: 1.0,
            orbit_speed: 1.0,
            pan_speed: 1.0,
            show_cube: true,
            cube_fit_on_change: false,
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
    /// Material Library favourites (by name).
    pub favourite_materials: Vec<String>,
}

impl Default for GeneralPrefs {
    fn default() -> Self {
        Self { autosave_minutes: 5, undo_steps: 64, units: Units::Meters, show_tour: true, favourite_materials: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderPrefs {
    /// Renderer for new scenes (old `engine` / `blender_exe` keys from the
    /// removed Cycles bridge are ignored on load).
    pub integrator: Integrator,
    pub device: ComputeDevice,
    pub width: u32,
    pub height: u32,
}

impl Default for RenderPrefs {
    fn default() -> Self {
        let r = RenderSettings::default();
        Self { integrator: r.path.integrator, device: r.path.device, width: r.width, height: r.height }
    }
}

impl RenderPrefs {
    /// Copies the defaults into a scene's Render Setup.
    pub fn apply(&self, r: &mut RenderSettings) {
        r.path.integrator = self.integrator;
        r.path.device = self.device;
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
    /// Hosts and sizes of the packs (1K / 2K photo textures).
    pub sources: crate::assets::Sources,
}

impl Default for AssetPrefs {
    fn default() -> Self {
        let folder = data_dir().join("assets").display().to_string();
        Self { folder, photos: true, lowpoly: true, realistic: false, sources: Default::default() }
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|| ".".into())
}

/// `<base>/ilex`, or the pre-rename `<base>/minimax` while only that exists,
/// so preferences, assets and crash reports carry over untouched.
pub fn app_dir(base: &std::path::Path) -> PathBuf {
    let (new, old) = (base.join("ilex"), base.join("minimax"));
    if !new.exists() && old.is_dir() { old } else { new }
}

/// `$XDG_CONFIG_HOME/ilex` (usually `~/.config/ilex`).
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute());
    app_dir(&base.unwrap_or_else(|| home().join(".config")))
}

/// `$XDG_DATA_HOME/ilex` (usually `~/.local/share/ilex`).
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute());
    app_dir(&base.unwrap_or_else(|| home().join(".local/share")))
}

impl Preferences {
    pub fn default_path() -> PathBuf {
        config_dir().join("preferences.ron")
    }

    pub fn from_ron(text: &str) -> Result<Self, String> {
        let mut p: Self = ron::from_str(text).map_err(|e| format!("Preferences: {e}"))?;
        p.viewport.migrate(p.theme.look_version);
        p.theme.migrate();
        Ok(p)
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
mod tests;
