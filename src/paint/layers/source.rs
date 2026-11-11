//! What a fill layer or a material brush paints with: flat PBR values, or
//! any material (one of the library presets) evaluated at each texel.

use crate::materials::Material;
use crate::materials::shading::{base_surface, linear_to_srgb, material_height};
use crate::paint::texel::{EMISSION, HEIGHT, METAL, ROUGH, Sample};
use bevy::math::{Vec2, Vec3, Vec4};

/// Indices into a channel mask `[colour, roughness, metallic, height, emission]`.
pub const CH_COLOR: usize = 0;
pub const CH_ROUGH: usize = 1;
pub const CH_METAL: usize = 2;
pub const CH_HEIGHT: usize = 3;
pub const CH_EMISSION: usize = 4;

/// Emission 1.0 on a layer glows this many times its colour.
pub const EMIT_GAIN: f32 = 4.0;

/// Flat PBR values with a switch per channel.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Flat {
    /// sRGB.
    pub color: [f32; 3],
    pub rough: f32,
    pub metal: f32,
    /// 0..1; 0.5 is level with the surface, below digs in, above stands out.
    pub height: f32,
    /// 0..1 glow of `color`.
    pub emission: f32,
    /// Which of colour, roughness, metallic, height, emission are written.
    pub channels: [bool; 5],
}

impl Default for Flat {
    fn default() -> Self {
        Self { color: [0.85, 0.18, 0.15], rough: 0.6, metal: 0.0, height: 0.5, emission: 0.0, channels: [true, false, false, false, false] }
    }
}

impl Flat {
    /// Colour, roughness and metallic (the usual look of a solid coat).
    pub fn coat(color: [f32; 3], rough: f32, metal: f32) -> Self {
        Self { color, rough, metal, height: 0.5, emission: 0.0, channels: [true, true, true, false, false] }
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self.channels[CH_HEIGHT] = true;
        self
    }

    pub fn sample(&self) -> Sample {
        let on = |i: usize| if self.channels[i] { 1.0 } else { 0.0 };
        let mut s = Sample { color: Vec3::from_array(self.color), a: on(CH_COLOR), pbr: [0.0; 4], cov: [0.0; 4] };
        (s.pbr[ROUGH], s.cov[ROUGH]) = (self.rough, on(CH_ROUGH));
        (s.pbr[METAL], s.cov[METAL]) = (self.metal, on(CH_METAL));
        (s.pbr[HEIGHT], s.cov[HEIGHT]) = (self.height, on(CH_HEIGHT));
        (s.pbr[EMISSION], s.cov[EMISSION]) = (self.emission, on(CH_EMISSION));
        s
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Source {
    Flat(Flat),
    /// A whole material: its colour, roughness, metallic, relief and glow.
    Material { mat: Box<Material>, tiling: f32 },
}

impl Default for Source {
    fn default() -> Self {
        Source::Flat(Flat::default())
    }
}

impl Source {
    /// A material source (its own paint layers are not carried along).
    pub fn material(mut mat: Material) -> Self {
        mat.paint = None;
        Source::Material { mat: Box::new(mat), tiling: 1.0 }
    }

    pub fn label(&self) -> String {
        match self {
            Source::Flat(_) => "Color".into(),
            Source::Material { mat, .. } => mat.name.clone(),
        }
    }

    /// The source at `uv` with full coverage of the channels it writes.
    pub fn sample(&self, uv: Vec2, p: Vec3) -> Sample {
        match self {
            Source::Flat(f) => f.sample(),
            Source::Material { mat, tiling } => {
                let uv = uv * tiling.max(1e-3);
                let s = base_surface(mat, uv, p, Vec4::ONE);
                let glow = s.emissive.max_element();
                // A glowing material paints its glow colour.
                let color = if glow > 1e-4 && s.base.max_element() < 1e-4 { s.emissive / glow } else { s.base };
                let color = Vec3::new(linear_to_srgb(color.x), linear_to_srgb(color.y), linear_to_srgb(color.z));
                let height = material_height(mat, uv, p);
                Sample {
                    color,
                    a: 1.0,
                    pbr: [s.roughness, s.metallic, height.unwrap_or(0.5), (glow / EMIT_GAIN).min(1.0)],
                    cov: [1.0, 1.0, if height.is_some() { 1.0 } else { 0.0 }, 1.0],
                }
            }
        }
    }
}
