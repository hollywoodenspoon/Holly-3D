//! One-click Easy Pose presets. Values are written for the character's
//! left side; `both` mirrors them onto the right.

use super::pose::{mirror, Pose};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PosePreset {
    APose,
    TPose,
    Relaxed,
    Walk,
    Run,
    Sit,
    Kneel,
    Wave,
    Fight,
    Victory,
    Cast,
}

impl PosePreset {
    pub const ALL: [PosePreset; 11] =
        [Self::APose, Self::TPose, Self::Relaxed, Self::Walk, Self::Run, Self::Sit, Self::Kneel, Self::Wave, Self::Fight, Self::Victory, Self::Cast];

    pub fn label(self) -> &'static str {
        match self {
            Self::APose => "A-Pose (Rest)",
            Self::TPose => "T-Pose",
            Self::Relaxed => "Relaxed Stand",
            Self::Walk => "Walk Stride",
            Self::Run => "Run",
            Self::Sit => "Sit",
            Self::Kneel => "Kneel",
            Self::Wave => "Wave Hello",
            Self::Fight => "Fighting Stance",
            Self::Victory => "Victory (Arms Up)",
            Self::Cast => "Spell Cast",
        }
    }

    pub fn pose(self) -> Pose {
        let mut p = Pose::default();
        let both = |p: &mut Pose, b: &str, r: [f32; 3]| p.set_mirrored(&format!("{b}.L"), r);
        let left = |p: &mut Pose, b: &str, r: [f32; 3]| p.set(&format!("{b}.L"), r);
        let right = |p: &mut Pose, b: &str, r: [f32; 3]| p.set(&format!("{b}.R"), mirror(r));
        match self {
            Self::APose => {}
            Self::TPose => both(&mut p, "upperarm01", [0.0, 0.0, 50.0]),
            Self::Relaxed => {
                both(&mut p, "upperarm01", [0.0, 0.0, -12.0]);
                both(&mut p, "lowerarm01", [-15.0, 0.0, 0.0]);
                p.set("head", [5.0, 0.0, 0.0]);
            }
            Self::Walk => {
                both(&mut p, "upperarm01", [0.0, 0.0, -25.0]);
                left(&mut p, "upperarm01", [20.0, 0.0, -25.0]);
                right(&mut p, "upperarm01", [-20.0, 0.0, -25.0]);
                both(&mut p, "lowerarm01", [-15.0, 0.0, 0.0]);
                left(&mut p, "upperleg01", [-25.0, 0.0, 0.0]);
                left(&mut p, "lowerleg01", [10.0, 0.0, 0.0]);
                right(&mut p, "upperleg01", [15.0, 0.0, 0.0]);
                right(&mut p, "lowerleg01", [25.0, 0.0, 0.0]);
            }
            Self::Run => {
                p.set("spine02", [15.0, 0.0, 0.0]);
                left(&mut p, "upperarm01", [40.0, 0.0, -30.0]);
                right(&mut p, "upperarm01", [-50.0, 0.0, -30.0]);
                both(&mut p, "lowerarm01", [-80.0, 0.0, 0.0]);
                left(&mut p, "upperleg01", [-60.0, 0.0, 0.0]);
                left(&mut p, "lowerleg01", [90.0, 0.0, 0.0]);
                right(&mut p, "upperleg01", [25.0, 0.0, 0.0]);
                right(&mut p, "lowerleg01", [40.0, 0.0, 0.0]);
            }
            Self::Sit => {
                both(&mut p, "upperleg01", [-90.0, 0.0, 0.0]);
                both(&mut p, "lowerleg01", [90.0, 0.0, 0.0]);
                both(&mut p, "upperarm01", [-20.0, 0.0, -30.0]);
                both(&mut p, "lowerarm01", [-40.0, 0.0, 0.0]);
            }
            Self::Kneel => {
                left(&mut p, "upperleg01", [-90.0, 0.0, 0.0]);
                left(&mut p, "lowerleg01", [90.0, 0.0, 0.0]);
                right(&mut p, "upperleg01", [5.0, 0.0, 0.0]);
                right(&mut p, "lowerleg01", [95.0, 0.0, 0.0]);
                both(&mut p, "upperarm01", [0.0, 0.0, -20.0]);
            }
            Self::Wave => {
                left(&mut p, "upperarm01", [0.0, 0.0, -12.0]);
                right(&mut p, "upperarm01", [0.0, 0.0, 115.0]);
                right(&mut p, "lowerarm01", [-30.0, 0.0, 20.0]);
                p.set("head", [0.0, 0.0, -8.0]);
            }
            Self::Fight => {
                p.set("spine02", [0.0, 20.0, 0.0]);
                p.set("head", [0.0, -15.0, 0.0]);
                both(&mut p, "upperarm01", [-50.0, 0.0, -30.0]);
                both(&mut p, "lowerarm01", [-100.0, 0.0, 0.0]);
                left(&mut p, "upperleg01", [-20.0, 0.0, 10.0]);
                left(&mut p, "lowerleg01", [20.0, 0.0, 0.0]);
                right(&mut p, "upperleg01", [15.0, 0.0, 10.0]);
                right(&mut p, "lowerleg01", [25.0, 0.0, 0.0]);
            }
            Self::Victory => {
                both(&mut p, "upperarm01", [0.0, 0.0, 140.0]);
                both(&mut p, "lowerarm01", [-10.0, 0.0, 0.0]);
                p.set("head", [-10.0, 0.0, 0.0]);
            }
            Self::Cast => {
                p.set("spine02", [-5.0, -15.0, 0.0]);
                left(&mut p, "upperarm01", [-80.0, 0.0, 20.0]);
                left(&mut p, "lowerarm01", [-20.0, 0.0, 0.0]);
                right(&mut p, "upperarm01", [-30.0, 0.0, -20.0]);
                right(&mut p, "lowerarm01", [-60.0, 0.0, 0.0]);
                left(&mut p, "upperleg01", [-15.0, 0.0, 0.0]);
                right(&mut p, "upperleg01", [10.0, 0.0, 0.0]);
            }
        }
        p
    }
}
