//! Painted materials in the GPU viewport: the layer composite is kept as
//! four textures (colour, ORM, normal, emission). A stroke re-composites
//! and re-uploads only the tiles it touched.

use super::mat_sync::{MatTextures, to_image};
use crate::editor::paint_layers::geo_for;
use crate::materials::{Material, Texture};
use crate::paint::bake::GeoMap;
use crate::paint::layers::composite::{Composite, Dirty};
use crate::paint::texel::TILE;
use crate::scene::SceneObject;
use bevy::prelude::*;

/// Viewport state of one painted material.
pub struct PaintGpu {
    composite: Composite,
    /// Geometry map of the painted object and the key it was built for.
    geo: Option<GeoMap>,
    geo_key: u64,
    /// Colour, ORM, normal, emission.
    handles: Option<[Handle<Image>; 4]>,
}

/// Copies the texels of `tiles` from `t` into the image's pixel data.
fn patch(img: &mut Image, t: &Texture, tiles: &[usize]) {
    let Some(data) = img.data.as_mut() else { return };
    let (n, per_row) = (t.width as usize, (t.width / TILE) as usize);
    if data.len() != n * n * 4 {
        return;
    }
    for &tile in tiles {
        let (ox, oy) = ((tile % per_row) * TILE as usize, (tile / per_row) * TILE as usize);
        for y in oy..oy + TILE as usize {
            let row = &t.data[y * n + ox..y * n + ox + TILE as usize];
            data[(y * n + ox) * 4..(y * n + ox + TILE as usize) * 4].copy_from_slice(row.as_flattened());
        }
    }
}

/// Changes when the geometry map of `o` under `m` would (0 = none needed).
fn geo_key(o: &SceneObject, m: &Material) -> u64 {
    let triplanar = m.projection.on() || m.multi.as_ref().is_some_and(|ms| ms.subs.iter().any(|s| s.mat.projection.on()));
    if !triplanar && !m.is_multi() {
        return 0;
    }
    let s = super::triplanar_uv::scale_of(o);
    [s.x, s.y, s.z].iter().fold(o.version.wrapping_mul(31).wrapping_add(1), |h, v| h.wrapping_mul(0x100_0000_01b3) ^ v.to_bits() as u64)
}

impl PaintGpu {
    pub fn new(size: u32) -> Self {
        Self { composite: Composite::new(size), geo: None, geo_key: u64::MAX, handles: None }
    }

    /// Brings the textures up to date with painted material `m` on object
    /// `o` and returns them for the viewport material.
    pub fn sync(&mut self, m: &Material, o: Option<&SceneObject>, images: &mut Assets<Image>) -> MatTextures {
        let size = m.paint.as_ref().map_or(TILE, |p| p.size);
        let key = o.map_or(0, |o| geo_key(o, m));
        if key != self.geo_key {
            self.geo = o.filter(|_| key != 0).and_then(|o| geo_for(o, m, size));
            self.geo_key = key;
        }
        let dirty = self.composite.update(m, self.geo.as_ref(), key);
        let c = &self.composite;
        let textures = [(&c.color, true), (&c.orm, false), (&c.normal, false), (&c.emissive, true)];
        match (&self.handles, &dirty) {
            (Some(handles), Dirty::Tiles(tiles)) => {
                for (h, (t, _)) in handles.iter().zip(textures) {
                    if let Some(mut img) = images.get_mut(h) {
                        patch(&mut img, t, tiles);
                    }
                }
            }
            (Some(_), Dirty::None) => {}
            _ => self.handles = Some(textures.map(|(t, srgb)| images.add(to_image(t, srgb)))),
        }
        let [base, orm, normal, emissive] = self.handles.clone().expect("handles set above");
        MatTextures { base: Some(base), occlusion: Some(orm.clone()), metal_rough: Some(orm), emissive: Some((emissive, c.peak)), normal: Some(normal) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::PaintStack;

    #[test]
    fn a_stroke_patches_only_its_tiles_into_the_uploaded_image() {
        let mut m = Material { paint: Some(PaintStack::new(128)), ..Default::default() };
        let mut images = Assets::<Image>::default();
        let mut gpu = PaintGpu::new(128);
        let tex = gpu.sync(&m, None, &mut images);
        let base = tex.base.clone().unwrap();
        let before = images.get(&base).unwrap().data.clone().unwrap();
        assert_eq!(before.len(), 128 * 128 * 4);
        // Paint one texel in the second tile of the first row.
        let stack = m.paint.as_mut().unwrap();
        if let crate::paint::LayerKind::Paint(t) = &mut stack.layers[0].kind {
            t.set(70, 3, crate::paint::texel::Texel { rgba: [255, 0, 0, 255], ..Default::default() });
        }
        stack.touch();
        let again = gpu.sync(&m, None, &mut images);
        assert_eq!(again.base.unwrap(), base, "the same GPU image is patched, not replaced");
        let after = images.get(&base).unwrap().data.clone().unwrap();
        let px = |d: &[u8], x: usize, y: usize| [d[(y * 128 + x) * 4], d[(y * 128 + x) * 4 + 1], d[(y * 128 + x) * 4 + 2]];
        assert_eq!(px(&after, 70, 3), [255, 0, 0]);
        assert_eq!(px(&after, 10, 100), px(&before, 10, 100), "untouched tiles keep their pixels");
    }
}
