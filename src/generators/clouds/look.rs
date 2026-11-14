//! Clouds V2: how a cloud renders. By default a Cloud or Cloud Field is a
//! true volume in F9 renders: its puffs become soft density, worn down by
//! erosion noise into billows and wisps. `Render As: Mesh` keeps the light
//! preview mesh in renders instead.

/// What the renderers draw for a cloud object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RenderAs {
    /// Soft participating medium (clouds V2).
    #[default]
    Volume,
    /// The preview mesh with its surface material.
    Mesh,
}

impl RenderAs {
    pub const ALL: [Self; 2] = [Self::Volume, Self::Mesh];
    pub fn label(self) -> &'static str {
        match self {
            Self::Volume => "Volume",
            Self::Mesh => "Mesh",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Self::Volume => "Render the cloud as soft density that light scatters through",
            Self::Mesh => "Render the light preview mesh with its surface material",
        }
    }
}

/// The volume look of a cloud (the "Cloud Volume" rows of the rollout).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CloudLook {
    pub render_as: RenderAs,
    /// Edge Softness 0.05..=1: share of each puff's radius that fades out.
    pub edge_softness: f32,
    /// Erosion 0..=1: how much noise eats into the thin outer density.
    pub erosion: f32,
    /// Wispiness 0..=1: extra erosion towards the top, for torn wisps.
    pub wispiness: f32,
    /// Density multiplier on the volume material's Density.
    pub density: f32,
}

impl Default for CloudLook {
    fn default() -> Self {
        Self { render_as: RenderAs::Volume, edge_softness: 0.35, erosion: 0.6, wispiness: 0.3, density: 1.0 }
    }
}

impl CloudLook {
    /// Erosion strength at height fraction `h` (0 base, 1 top): how deep
    /// into the cloud the noise may cut (below 1 so cores stay).
    pub fn strength(&self, h: f32) -> f32 {
        (self.erosion.clamp(0.0, 1.0) * (0.6 + self.wispiness.clamp(0.0, 1.0) * h.clamp(0.0, 1.0))).min(0.95)
    }
}
