//! Direct 3D model painting: 3ds Max VertexPaint (per-vertex color layer,
//! no UVs needed) and texture projection paint onto a Base Color canvas.
//! Pure brush math lives here; the editor wires strokes to the viewport.

pub mod bake;
pub mod brush;
pub mod layers;
pub mod raster;
#[cfg(test)]
mod tests;
pub mod texel;
pub mod vertex;

pub use bake::{BakeSettings, MeshMaps, SmartKind, SmartMask};
pub use brush::{Alpha, LayerStroke, Stamp};
pub use layers::{BlendMode, Flat, LayerKind, PaintLayer, PaintStack, Source};
pub use raster::{PaintTri, has_uvs, paint_tris, uv_overlap};
pub use vertex::VertexStroke;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PaintMode {
    /// VertexPaint modifier layer (no UVs required).
    Vertex,
    /// Projection paint into the material's paint layers.
    Texture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BrushTool {
    Paint,
    Blur,
    /// Removes paint from the layer; what is below shows again.
    Erase,
}

impl BrushTool {
    pub fn label(self) -> &'static str {
        match self {
            BrushTool::Paint => "Paint",
            BrushTool::Blur => "Blur",
            BrushTool::Erase => "Erase",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Brush {
    /// Brush radius in world units (3ds Max PaintBox "Size").
    pub size: f32,
    /// 0..=1 maximum coverage per stroke.
    pub opacity: f32,
    /// 0 = hard edge, 1 = soft from the center.
    pub falloff: f32,
    /// sRGB.
    pub color: [f32; 3],
    pub tool: BrushTool,
    /// Brush alpha or damage stamp (texture paint).
    #[serde(default)]
    pub alpha: Alpha,
}

impl Default for Brush {
    fn default() -> Self {
        Self { size: 0.35, opacity: 1.0, falloff: 0.5, color: [0.85, 0.18, 0.15], tool: BrushTool::Paint, alpha: Alpha::SoftRound }
    }
}

impl Brush {
    /// Coverage at distance `d` from the brush center (0 outside).
    pub fn weight(&self, d: f32) -> f32 {
        falloff_weight(d, self.size, self.falloff) * self.opacity.clamp(0.0, 1.0)
    }
}

/// Smooth radial falloff: 1 inside `(1 - falloff) * r`, easing to 0 at `r`.
pub fn falloff_weight(d: f32, r: f32, falloff: f32) -> f32 {
    if r <= 0.0 || d >= r {
        return 0.0;
    }
    let t = d / r;
    let f = falloff.clamp(0.0, 1.0);
    let inner = 1.0 - f;
    if t <= inner || f <= 1e-4 {
        return 1.0;
    }
    let x = 1.0 - (t - inner) / f;
    x * x * (3.0 - 2.0 * x)
}

/// PaintBox state (settings persist between paint sessions).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaintSettings {
    /// Paint mode is on: viewport left-drags paint instead of selecting.
    pub active: bool,
    pub mode: PaintMode,
    pub brush: Brush,
    /// "Selected Faces Only" mask (selection passed up the stack).
    pub selected_only: bool,
    /// Texture size of new paint layers.
    pub canvas_size: u32,
    /// Folder `[ Export Textures ]` writes to.
    pub save_path: String,
    /// Roughness, metallic, height and emission the colour brush paints,
    /// and which channels it writes (its colour is `brush.color`).
    #[serde(default)]
    pub look: Flat,
    /// Material preset the brush paints with instead of a flat colour.
    #[serde(default)]
    pub material: Option<Box<crate::materials::Material>>,
    #[serde(default = "one")]
    pub material_tiling: f32,
    #[serde(default)]
    pub bake: BakeSettings,
}

fn one() -> f32 {
    1.0
}

impl PaintSettings {
    /// What texture-paint strokes and new fill layers paint with.
    pub fn source(&self) -> Source {
        match &self.material {
            Some(m) => Source::Material { mat: m.clone(), tiling: self.material_tiling },
            None => Source::Flat(Flat { color: self.brush.color, ..self.look.clone() }),
        }
    }
}

impl Default for PaintSettings {
    fn default() -> Self {
        Self {
            active: false,
            mode: PaintMode::Vertex,
            brush: Brush::default(),
            selected_only: false,
            canvas_size: 2048,
            save_path: String::new(),
            look: Flat::default(),
            material: None,
            material_tiling: 1.0,
            bake: BakeSettings::default(),
        }
    }
}

/// Quick swatches shown in the PaintBox palette (sRGB).
pub const PALETTE: [[f32; 3]; 12] = [
    [1.0, 1.0, 1.0],
    [0.0, 0.0, 0.0],
    [0.5, 0.5, 0.5],
    [0.85, 0.18, 0.15],
    [0.95, 0.55, 0.12],
    [0.98, 0.85, 0.2],
    [0.3, 0.7, 0.25],
    [0.15, 0.6, 0.65],
    [0.2, 0.4, 0.9],
    [0.55, 0.3, 0.8],
    [0.9, 0.45, 0.65],
    [0.45, 0.3, 0.2],
];
