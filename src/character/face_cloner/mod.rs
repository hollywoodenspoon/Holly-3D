//! Face Cloner: fits the MakeHuman head to a person from photos or
//! exported video frames. MediaPipe's 478-point face mesh is found in
//! every image, the frames are culled and sorted into views, and the
//! head's own morph targets (then a fine RBF warp) are solved so the
//! projected head matches every view.

pub mod correspond;
pub mod math;
pub mod model;
pub mod render;

#[cfg(test)]
mod tests;
