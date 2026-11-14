//! F10 Render Setup -> Volumes rollout settings.

/// How many transmittance estimates a shadow ray through a volume averages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShadowQuality {
    Fast,
    Balanced,
    Smooth,
}

impl ShadowQuality {
    pub const ALL: [Self; 3] = [Self::Fast, Self::Balanced, Self::Smooth];
    pub fn label(self) -> &'static str {
        match self {
            Self::Fast => "Fast (1 estimate)",
            Self::Balanced => "Balanced (2 estimates)",
            Self::Smooth => "Smooth (4 estimates)",
        }
    }
    pub fn estimates(self) -> u32 {
        match self {
            Self::Fast => 1,
            Self::Balanced => 2,
            Self::Smooth => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct VolumeRender {
    /// Max Volume Bounces: scattering events a path may take inside volumes.
    pub max_bounces: u32,
    /// Cloud Bounce Boost 0..=1: approximates the bright, soft look of the
    /// many bounces in thick clouds by adding weaker, wider light octaves.
    pub boost: f32,
    pub shadow_quality: ShadowQuality,
}

impl Default for VolumeRender {
    fn default() -> Self {
        Self { max_bounces: 8, boost: 0.5, shadow_quality: ShadowQuality::Balanced }
    }
}
