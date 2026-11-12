//! Face Cloner: fits the MakeHuman head to a person from photos or
//! exported video frames. MediaPipe's 478-point face mesh is found in
//! every image, the frames are culled and sorted into views, and the
//! head's own morph targets (then a fine RBF warp) are solved so the
//! projected head matches every view.

pub mod calibrate;
pub mod clone;
pub mod correspond;
pub mod fit;
pub mod frames;
pub mod job;
pub mod math;
pub mod model;
pub mod render;
pub mod warp;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_fit;

use bevy::math::Vec3;

/// A cloned face stored on the character: the fine per-vertex warp (the
/// slider part lives in `CharacterParams::micro` like any other slider).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FaceClone {
    /// hm08 vertex index -> offset in decimetres (head and upper neck only).
    pub deltas: Vec<(u32, [f32; 3])>,
    /// How much of the warp to show (0 = sliders only, 1 = full).
    pub strength: f32,
    /// How many photos / frames the fit used (0 = no cloned face).
    pub views: usize,
}

impl Default for FaceClone {
    fn default() -> Self {
        Self { deltas: Vec::new(), strength: 1.0, views: 0 }
    }
}

impl FaceClone {
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    /// Adds the warp to hm08-order coordinates.
    pub fn apply(&self, coords: &mut [Vec3]) {
        for (i, d) in &self.deltas {
            if let Some(c) = coords.get_mut(*i as usize) {
                *c += Vec3::from_array(*d) * self.strength;
            }
        }
    }
}
