//! Theme presets for Preferences -> Theme: the colours every panel uses.
//! Colours are sRGB bytes so `preferences.ron` stays readable.

use serde::{Deserialize, Serialize};

pub type Rgb = [u8; 3];

/// The interface colours (the `src/ui/theme.rs` accessors read these).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Palette {
    /// Light themes start from egui's light visuals.
    pub light: bool,
    pub panel: Rgb,
    pub card: Rgb,
    pub input: Rgb,
    /// Active Q/W/E/R, sub-object and snap buttons.
    pub accent: Rgb,
    /// Hover outline.
    pub hover: Rgb,
    pub text: Rgb,
    pub text_dim: Rgb,
    pub button: Rgb,
    pub button_hover: Rgb,
    pub border: Rgb,
}

impl Default for Palette {
    fn default() -> Self {
        ThemePreset::GraphiteTeal.palette()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemePreset {
    #[default]
    GraphiteTeal,
    SlateAmber,
    MaxClassic,
    MidnightBlue,
    Light,
    HighContrast,
    Custom,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 7] =
        [Self::GraphiteTeal, Self::SlateAmber, Self::MaxClassic, Self::MidnightBlue, Self::Light, Self::HighContrast, Self::Custom];

    pub fn label(self) -> &'static str {
        match self {
            Self::GraphiteTeal => "Graphite & Teal (default)",
            Self::SlateAmber => "Slate & Amber",
            Self::MaxClassic => "Classic Gray",
            Self::MidnightBlue => "Midnight Blue",
            Self::Light => "Light",
            Self::HighContrast => "High Contrast",
            Self::Custom => "Custom",
        }
    }

    /// The preset's colours (`Custom` starts from Graphite & Teal).
    pub fn palette(self) -> Palette {
        let p = |light, panel, card, input, accent, hover, text, text_dim, button, button_hover, border| Palette {
            light,
            panel,
            card,
            input,
            accent,
            hover,
            text,
            text_dim,
            button,
            button_hover,
            border,
        };
        match self {
            // Neutral greys with no blue tint and a blue-green accent.
            Self::GraphiteTeal | Self::Custom => p(
                false,
                [0x24, 0x24, 0x24],
                [0x2E, 0x2E, 0x2E],
                [0x1A, 0x1A, 0x1A],
                [0x26, 0xB8, 0xB0],
                [0x3F, 0xD0, 0xC7],
                [0xE8, 0xE8, 0xE8],
                [0xA6, 0xA6, 0xA6],
                [0x33, 0x33, 0x33],
                [0x3D, 0x3D, 0x3D],
                [0x3C, 0x3C, 0x3C],
            ),
            Self::SlateAmber => p(
                false,
                [0x22, 0x23, 0x27],
                [0x2B, 0x2D, 0x31],
                [0x18, 0x19, 0x1C],
                [0xE5, 0x9C, 0x24],
                [0x3A, 0x7A, 0xFE],
                [0xD6, 0xD8, 0xDD],
                [0x8C, 0x90, 0x99],
                [0x36, 0x38, 0x3E],
                [0x40, 0x43, 0x4A],
                [0x3A, 0x3C, 0x42],
            ),
            Self::MaxClassic => p(
                false,
                [0x44, 0x44, 0x44],
                [0x50, 0x50, 0x50],
                [0x2B, 0x2B, 0x2B],
                [0xF0, 0xB4, 0x3C],
                [0x5B, 0x9B, 0xD5],
                [0xEE, 0xEE, 0xEE],
                [0xB4, 0xB4, 0xB4],
                [0x30, 0x30, 0x30],
                [0x3A, 0x3A, 0x3A],
                [0x2A, 0x2A, 0x2A],
            ),
            Self::MidnightBlue => p(
                false,
                [0x1A, 0x20, 0x33],
                [0x22, 0x2A, 0x40],
                [0x12, 0x17, 0x26],
                [0x45, 0xB5, 0xE8],
                [0x8C, 0x9C, 0xFF],
                [0xDC, 0xE3, 0xF0],
                [0x8E, 0x9A, 0xB5],
                [0x2C, 0x35, 0x50],
                [0x36, 0x40, 0x6A],
                [0x31, 0x3A, 0x57],
            ),
            Self::Light => p(
                true,
                [0xE8, 0xE9, 0xEC],
                [0xF4, 0xF5, 0xF7],
                [0xFF, 0xFF, 0xFF],
                [0xB8, 0x6A, 0x00],
                [0x2F, 0x6F, 0xEB],
                [0x1E, 0x20, 0x24],
                [0x55, 0x5A, 0x64],
                [0xD5, 0xD7, 0xDC],
                [0xC8, 0xCB, 0xD1],
                [0xB8, 0xBC, 0xC4],
            ),
            Self::HighContrast => p(
                false,
                [0x00, 0x00, 0x00],
                [0x10, 0x10, 0x10],
                [0x00, 0x00, 0x00],
                [0xFF, 0xD4, 0x00],
                [0x00, 0xBF, 0xFF],
                [0xFF, 0xFF, 0xFF],
                [0xCC, 0xCC, 0xCC],
                [0x22, 0x22, 0x22],
                [0x38, 0x38, 0x38],
                [0xFF, 0xFF, 0xFF],
            ),
        }
    }
}

/// Raised whenever the default look changes, so saved prefs move over once.
pub const LOOK_VERSION: u32 = 1;

impl super::ThemePrefs {
    /// Version 1 made Graphite & Teal the default: prefs still on the old
    /// default (Slate & Amber) switch once; a later pick of Slate & Amber stays.
    pub fn migrate(&mut self) {
        if self.look_version < LOOK_VERSION {
            if self.preset == ThemePreset::SlateAmber {
                self.preset = ThemePreset::GraphiteTeal;
            }
            self.look_version = LOOK_VERSION;
        }
    }
}

/// WCAG relative luminance of an sRGB colour.
pub fn luminance(c: Rgb) -> f32 {
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

/// WCAG contrast ratio, 1.0 (none) to 21.0 (black on white).
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

impl Palette {
    /// Readability problems ("Text on buttons: 2.1:1, needs 4.5:1"), empty when fine.
    pub fn readability(&self) -> Vec<String> {
        let checks = [
            ("Text on panels", self.text, self.panel, 4.5),
            ("Text on cards", self.text, self.card, 4.5),
            ("Text on buttons", self.text, self.button, 4.5),
            ("Text in fields", self.text, self.input, 4.5),
            ("Dim text on panels", self.text_dim, self.panel, 3.0),
            ("Active buttons", self.accent, self.panel, 3.0),
        ];
        checks
            .iter()
            .filter_map(|&(what, a, b, need)| {
                let c = contrast(a, b);
                (c < need).then(|| format!("{what}: {c:.1}:1, needs {need}:1"))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_matches_wcag_extremes() {
        assert!((contrast([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.05);
        assert!((contrast([90, 90, 90], [90, 90, 90]) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn every_theme_keeps_text_readable() {
        for t in ThemePreset::ALL {
            let problems = t.palette().readability();
            assert!(problems.is_empty(), "{}: {problems:?}", t.label());
        }
    }

    #[test]
    fn graphite_teal_is_the_default() {
        assert_eq!(ThemePreset::default(), ThemePreset::GraphiteTeal);
        assert_eq!(super::super::ThemePrefs::default().preset, ThemePreset::GraphiteTeal);
        assert_eq!(Palette::default().accent, [0x26, 0xB8, 0xB0]);
    }

    #[test]
    fn old_amber_prefs_move_to_teal_once() {
        use super::super::Preferences;
        let old = Preferences::from_ron("(theme: (preset: SlateAmber))").unwrap();
        assert_eq!(old.theme.preset, ThemePreset::GraphiteTeal);
        assert_eq!(old.theme.look_version, LOOK_VERSION);
        let picked = Preferences::from_ron(&format!("(theme: (preset: SlateAmber, look_version: {LOOK_VERSION}))")).unwrap();
        assert_eq!(picked.theme.preset, ThemePreset::SlateAmber);
        let other = Preferences::from_ron("(theme: (preset: MidnightBlue))").unwrap();
        assert_eq!(other.theme.preset, ThemePreset::MidnightBlue);
    }

    #[test]
    fn readability_flags_grey_on_grey() {
        let p = Palette { text: [0x50, 0x50, 0x50], ..Palette::default() };
        assert!(!p.readability().is_empty());
    }
}
