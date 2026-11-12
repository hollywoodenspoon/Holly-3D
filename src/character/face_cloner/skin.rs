//! The cloned skin: the photos baked over the character's current skin
//! (its MakeHuman skin texture, or the plain Character Pack skin colour).

use super::bake::{self, Baked, View};
use super::clone::view_weight;
use super::frames::Shot;
use super::mask;
use super::math::Similarity;
use crate::character::makehuman::library;
use crate::character::params::CharacterParams;
use crate::materials::texture::{TexHandle, Texture};
use bevy::math::Vec3;

/// Size of the skin texture made when the character has no skin texture.
pub const PLAIN_SIZE: u32 = 1024;

/// The skin the photos are baked over.
pub fn base_skin(p: &CharacterParams) -> Texture {
    let tex = p.mh.skin.as_ref().and_then(|s| {
        let mm = library::material(&format!("skins/{s}/{s}.mhmat"))?;
        library::texture(&format!("skins/{s}/{}", mm.diffuse_texture?))
    });
    match tex {
        Some(t) => (*t.tex).clone(),
        None => {
            let c = crate::materials::multi::skin_material().base_color.map(super::delight::to_srgb);
            Texture::new(PLAIN_SIZE, PLAIN_SIZE, [c[0], c[1], c[2], 255])
        }
    }
}

/// The picture of `s` at its analysed size, read again from its file.
pub fn picture(s: &Shot) -> Option<image::RgbImage> {
    let img = image::open(&s.name).ok()?.to_rgb8();
    Some(if img.dimensions() == s.size { img } else { image::imageops::resize(&img, s.size.0, s.size.1, image::imageops::FilterType::Triangle) })
}

/// What the skin bake did, for the report.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkinReport {
    pub baked: Baked,
    /// Photos baked, and how many of them MobileSAM masked.
    pub photos: usize,
    pub sam_masks: usize,
}

/// Bakes `shots` (with their fitted `cams`) and the given pictures onto
/// the head of `coords`, over `p`'s current skin.
pub fn bake_pictures(p: &CharacterParams, coords: &[Vec3], head: (Vec3, f32), shots: &[(&Shot, Similarity, image::RgbImage)]) -> Option<(TexHandle, SkinReport)> {
    let masks: Vec<mask::Mask> = shots.iter().map(|(s, _, img)| mask::skin_mask(s, img)).collect();
    let views: Vec<View> = shots.iter().zip(&masks).map(|((s, cam, img), m)| View { cam: *cam, img, mask: m, weight: view_weight(s.yaw) }).collect();
    if views.is_empty() {
        return None;
    }
    let (tex, baked) = bake::bake(coords, head, &views, &base_skin(p));
    (baked.covered > 0.0).then(|| (TexHandle::new(tex), SkinReport { baked, photos: views.len(), sam_masks: masks.iter().filter(|m| m.sam).count() }))
}

/// Reads the pictures of `shots` from disk and bakes them (None when none could be read).
pub fn bake_shots(p: &CharacterParams, coords: &[Vec3], head: (Vec3, f32), shots: &[&Shot], cams: &[Similarity]) -> Option<(TexHandle, SkinReport)> {
    let read: Vec<(&Shot, Similarity, image::RgbImage)> = shots.iter().zip(cams).filter_map(|(s, c)| Some((*s, *c, picture(s)?))).collect();
    bake_pictures(p, coords, head, &read)
}
