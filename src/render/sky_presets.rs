//! One-click sky presets (toolbar Sky dropdown, Rendering menu, Create tab):
//! Clear Day, Sunset, Overcast, Night, Deep Space and Nebula.

use super::sky::{Clouds, SkyMode, SkySettings, Space};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkyPreset {
    Off,
    ClearDay,
    Sunset,
    Overcast,
    Night,
    DeepSpace,
    Nebula,
}

impl SkyPreset {
    pub const ALL: [SkyPreset; 7] = [Self::Off, Self::ClearDay, Self::Sunset, Self::Overcast, Self::Night, Self::DeepSpace, Self::Nebula];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "No Sky (Background Color)",
            Self::ClearDay => "Clear Day",
            Self::Sunset => "Sunset",
            Self::Overcast => "Overcast",
            Self::Night => "Night",
            Self::DeepSpace => "Deep Space",
            Self::Nebula => "Nebula",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Off => "⬛",
            Self::ClearDay => "☀",
            Self::Sunset => "🌇",
            Self::Overcast => "☁",
            Self::Night => "🌙",
            Self::DeepSpace => "✨",
            Self::Nebula => "🌌",
        }
    }

    /// Sets the sky's look; keeps the viewport / lighting / sun-link switches.
    pub fn apply(self, s: &mut SkySettings) {
        let keep = (s.link_sun, s.lighting, s.show_in_viewport, s.sun_azimuth);
        *s = SkySettings::default();
        (s.link_sun, s.lighting, s.show_in_viewport, s.sun_azimuth) = keep;
        let clouds = |coverage: f32, density: f32| Clouds { enabled: true, coverage, density, ..Default::default() };
        match self {
            Self::Off => s.mode = SkyMode::Off,
            Self::ClearDay => {
                (s.mode, s.time_of_day) = (SkyMode::Earth, 13.0);
                s.clouds = clouds(0.15, 0.6);
            }
            Self::Sunset => {
                (s.mode, s.time_of_day) = (SkyMode::Earth, 18.6);
                s.clouds = clouds(0.35, 0.7);
            }
            Self::Overcast => {
                (s.mode, s.time_of_day, s.intensity) = (SkyMode::Earth, 12.0, 0.8);
                s.clouds = clouds(0.95, 0.95);
            }
            Self::Night => {
                (s.mode, s.time_of_day) = (SkyMode::Earth, 23.0);
                s.clouds = clouds(0.2, 0.5);
            }
            Self::DeepSpace => {
                s.mode = SkyMode::Space;
                s.space = Space { nebula: false, star_density: 0.03, ..Default::default() };
            }
            Self::Nebula => {
                s.mode = SkyMode::Space;
                s.space = Space { nebula: true, nebula_density: 0.85, ..Default::default() };
            }
        }
    }

    /// The preset that `s` currently matches, if any.
    pub fn matching(s: &SkySettings) -> Option<Self> {
        Self::ALL.into_iter().find(|p| {
            let mut t = s.clone();
            p.apply(&mut t);
            t == *s
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_sets_its_mode_and_is_recognised() {
        for p in SkyPreset::ALL {
            let mut s = SkySettings { link_sun: false, ..Default::default() };
            p.apply(&mut s);
            let want = match p {
                SkyPreset::Off => SkyMode::Off,
                SkyPreset::DeepSpace | SkyPreset::Nebula => SkyMode::Space,
                _ => SkyMode::Earth,
            };
            assert_eq!(s.mode, want, "{p:?}");
            assert!(!s.link_sun, "keeps the user's switches");
            assert_eq!(SkyPreset::matching(&s), Some(p));
        }
        let mut s = SkySettings::default();
        SkyPreset::Sunset.apply(&mut s);
        assert!(s.sun_dir().z < 0.35, "sunset sun is low");
        SkyPreset::ClearDay.apply(&mut s);
        assert!(s.sun_dir().z > 0.5, "midday sun is high");
    }
}
