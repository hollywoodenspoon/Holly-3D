//! The sky's cloud settings: the 2D painted layer, whether the placed 3D
//! cloud objects show, and the 2D bake resolution.

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Clouds {
    pub enabled: bool,
    /// Fraction of the sky covered (0..1).
    pub coverage: f32,
    /// Opacity of the cloud layer (0..1).
    pub density: f32,
    /// Cloud deck height (km); higher decks look smaller and flatter.
    pub altitude: f32,
    /// Drift (km per hour of Time of Day) along +X.
    pub wind_speed: f32,
    /// Show the placed 3D cloud objects (Clouds dropdown: 3D / Both).
    pub three_d: bool,
    /// Width of the baked 2D sky (1024 / 2048 / 4096).
    pub res_2d: u32,
}

/// The Environment "Clouds" dropdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudsShown {
    Off,
    TwoD,
    ThreeD,
    Both,
}

impl CloudsShown {
    pub const ALL: [Self; 4] = [Self::Off, Self::TwoD, Self::ThreeD, Self::Both];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::TwoD => "2D Sky Clouds",
            Self::ThreeD => "3D Cloud Objects",
            Self::Both => "Both",
        }
    }
}

impl Clouds {
    pub fn shown(&self) -> CloudsShown {
        match (self.enabled, self.three_d) {
            (false, false) => CloudsShown::Off,
            (true, false) => CloudsShown::TwoD,
            (false, true) => CloudsShown::ThreeD,
            (true, true) => CloudsShown::Both,
        }
    }
    pub fn set_shown(&mut self, s: CloudsShown) {
        (self.enabled, self.three_d) = match s {
            CloudsShown::Off => (false, false),
            CloudsShown::TwoD => (true, false),
            CloudsShown::ThreeD => (false, true),
            CloudsShown::Both => (true, true),
        };
    }
    /// The 2D bake width, clamped to the offered sizes.
    pub fn bake_width(&self) -> u32 {
        self.res_2d.clamp(1024, 4096)
    }
}

impl Default for Clouds {
    fn default() -> Self {
        Self { enabled: true, coverage: 0.4, density: 0.8, altitude: 2.0, wind_speed: 0.5, three_d: true, res_2d: 1024 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    use crate::generators::{WorldKind, clouds::CloudType};
    use crate::render::scene_build::RenderScene;
    use crate::viewport::camera::{ViewCamera, ViewKind};
    use bevy::math::Vec3;

    #[test]
    fn clouds_dropdown_round_trips_and_old_scenes_keep_3d_on() {
        let mut c = Clouds::default();
        for s in CloudsShown::ALL {
            c.set_shown(s);
            assert_eq!(c.shown(), s);
        }
        let old: Clouds = serde_json::from_str(r#"{"enabled":true,"coverage":0.5}"#).unwrap();
        assert!(old.three_d && old.bake_width() == 1024 && old.shown() == CloudsShown::Both);
        assert_eq!(Clouds { res_2d: 99_999, ..old }.bake_width(), 4096);
    }

    #[test]
    fn three_d_off_drops_cloud_objects_from_the_render() {
        let mut ed = Editor::default();
        ed.create_world(WorldKind::Cloud(CloudType::Cumulus), Vec3::ZERO);
        let cam = ViewCamera::new(ViewKind::Perspective);
        let on = RenderScene::build(&ed.scene, cam, false).bvh.tris.len();
        assert!(on > 0);
        ed.scene.environment.sky.clouds.set_shown(CloudsShown::TwoD);
        assert_eq!(RenderScene::build(&ed.scene, cam, false).bvh.tris.len(), 0);
    }
}
