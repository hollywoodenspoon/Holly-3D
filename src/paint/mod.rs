//! Direct 3D model painting: 3ds Max VertexPaint (per-vertex color layer,
//! no UVs needed) and texture projection paint onto a Base Color canvas.
//! Pure brush math lives here; the editor wires strokes to the viewport.

#[cfg(test)]
mod tests;
pub mod texture;
pub mod vertex;

pub use texture::{PaintTri, TextureStroke, paint_tris, uv_overlap};
pub use vertex::VertexStroke;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PaintMode {
    /// VertexPaint modifier layer (no UVs required).
    Vertex,
    /// Projection paint into the material's Base Color bitmap.
    Texture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BrushTool {
    Paint,
    Blur,
    /// Removes paint from the VertexPaint layer (vertex mode only).
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
}

impl Default for Brush {
    fn default() -> Self {
        Self { size: 0.35, opacity: 1.0, falloff: 0.5, color: [0.85, 0.18, 0.15], tool: BrushTool::Paint }
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
    /// Canvas resolution for Auto-Setup Paint Canvas.
    pub canvas_size: u32,
    pub save_path: String,
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
