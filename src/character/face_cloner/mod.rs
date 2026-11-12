//! Face Cloner: fits the MakeHuman head to a person from photos or
//! exported video frames. MediaPipe's 478-point face mesh is found in
//! every image, the frames are culled and sorted into views, and the
//! head's own morph targets (then a fine RBF warp) are solved so the
//! projected head matches every view.

pub mod bake;
pub mod calibrate;
pub mod clone;
pub mod correspond;
pub mod delight;
pub mod fit;
pub mod frames;
pub mod job;
pub mod mask;
pub mod math;
pub mod model;
pub mod render;
pub mod sam;
pub mod skin;
pub mod warp;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_fit;
#[cfg(test)]
mod tests_skin;

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
    /// The photos baked into the skin (hm08 UVs), when the clone made one.
    pub texture: Option<crate::materials::texture::TexHandle>,
    /// Show the baked skin (off = the character's own skin).
    pub use_texture: bool,
}

impl Default for FaceClone {
    fn default() -> Self {
        Self { deltas: Vec::new(), strength: 1.0, views: 0, texture: None, use_texture: true }
    }
}

impl FaceClone {
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    /// The baked skin, if there is one and it is switched on.
    pub fn skin(&self) -> Option<&crate::materials::texture::TexHandle> {
        self.texture.as_ref().filter(|_| self.use_texture)
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
