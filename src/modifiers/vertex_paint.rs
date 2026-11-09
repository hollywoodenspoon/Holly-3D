//! VertexPaint modifier: a paint layer of per-vertex RGBA (alpha =
//! coverage) composited over the input's vertex colors with a Layer Opacity
//! (3ds Max VertexPaint, no UVs required).

use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Vec3, Vec4};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct VertexPaintMod {
    /// Painted color (rgb, sRGB) + coverage (a) per input vertex.
    pub layer: HashMap<VertexId, Vec4>,
    /// 0..=1 (PaintBox "Layer Opacity").
    pub layer_opacity: f32,
}

impl Default for VertexPaintMod {
    fn default() -> Self {
        Self { layer: HashMap::new(), layer_opacity: 1.0 }
    }
}

/// Porter-Duff "over": `src` color at coverage `w` on top of `dst`.
pub fn over(dst: Vec4, src: Vec3, w: f32) -> Vec4 {
    let w = w.clamp(0.0, 1.0);
    let a = w + dst.w * (1.0 - w);
    if a <= 1e-6 {
        return Vec4::ZERO;
    }
    let rgb = (src * w + dst.truncate() * dst.w * (1.0 - w)) / a;
    rgb.extend(a)
}

impl VertexPaintMod {
    /// Color of `v` after the layer at full layer opacity.
    pub fn painted(&self, input: Vec4, v: VertexId) -> Vec3 {
        let base = input.truncate();
        self.layer.get(&v).map_or(base, |l| base.lerp(l.truncate(), l.w))
    }

    pub fn apply(&self, mesh: &mut EditablePoly) {
        for (vid, v) in mesh.vertices.iter_mut() {
            if let Some(l) = self.layer.get(&vid) {
                let t = l.w * self.layer_opacity;
                let rgb = v.color.truncate().lerp(l.truncate(), t);
                v.color = rgb.extend(v.color.w);
            }
        }
    }
}
