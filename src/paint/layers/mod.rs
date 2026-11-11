//! Non-destructive paint layers over a material (TextureCraft's `compose`
//! design, MIT / Apache-2.0). The material itself is "Layer 0: Base
//! Material" and is never written to; paint layers and fill layers sit on
//! top of it, each with a blend mode, an opacity and a visibility toggle.
//! Erasing a layer shows the untouched base again.

pub mod blend;
pub mod composite;
mod source;
#[cfg(test)]
mod tests;

pub use source::{CH_COLOR, CH_EMISSION, CH_HEIGHT, CH_METAL, CH_ROUGH, Flat, Source};

use super::bake::{MeshMaps, SmartMask};
use super::texel::{Sample, Texel, Tiles};
use bevy::math::{Vec2, Vec3};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    /// Additive: glows and light.
    Add,
    Screen,
    /// The layer fills the low places of the surface below first; the
    /// opacity is how high it has risen.
    HeightBlend,
}

impl BlendMode {
    pub const ALL: [BlendMode; 5] = [BlendMode::Normal, BlendMode::Multiply, BlendMode::Add, BlendMode::Screen, BlendMode::HeightBlend];
    pub fn label(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Add => "Add / Glow",
            BlendMode::Screen => "Screen",
            BlendMode::HeightBlend => "Height-Blend",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LayerKind {
    /// Hand-painted texels.
    Paint(Tiles<Texel>),
    /// One look everywhere, shown through a mask (`None` = everywhere).
    Fill { source: Source, mask: Option<Tiles<u8>> },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaintLayer {
    pub name: String,
    pub kind: LayerKind,
    pub blend: BlendMode,
    /// 0..=1.
    pub opacity: f32,
    pub visible: bool,
    /// Smart mask settings, kept so the mask can be rebuilt.
    #[serde(default)]
    pub smart: Option<SmartMask>,
}

impl PaintLayer {
    pub fn paint(name: impl Into<String>, size: u32) -> Self {
        Self { name: name.into(), kind: LayerKind::Paint(Tiles::new(size)), blend: BlendMode::Normal, opacity: 1.0, visible: true, smart: None }
    }

    pub fn fill(name: impl Into<String>, source: Source, mask: Option<Tiles<u8>>) -> Self {
        Self { name: name.into(), kind: LayerKind::Fill { source, mask }, blend: BlendMode::Normal, opacity: 1.0, visible: true, smart: None }
    }

    pub fn is_fill(&self) -> bool {
        matches!(self.kind, LayerKind::Fill { .. })
    }

    /// The layer at `uv` before its opacity and blend mode (`p` is the
    /// surface point, for 3D maps of a fill material).
    pub fn sample(&self, uv: Vec2, p: Vec3) -> Option<Sample> {
        match &self.kind {
            LayerKind::Paint(t) => t.sample(uv),
            LayerKind::Fill { source, mask } => {
                let m = mask.as_ref().map_or(1.0, |m| m.sample(uv));
                (m > 0.0).then(|| source.sample(uv, p).faded(m))
            }
        }
    }

    /// Texel memory held, in bytes.
    pub fn bytes(&self) -> usize {
        match &self.kind {
            LayerKind::Paint(t) => t.bytes(),
            LayerKind::Fill { mask, .. } => mask.as_ref().map_or(0, |m| m.bytes()),
        }
    }
}

/// The paint layers of one material. Layer 0 (the base material) is
/// implicit; `layers[0]` is the first layer above it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PaintStack {
    /// Texture size of paint layers and of the viewport composite.
    pub size: u32,
    pub layers: Vec<PaintLayer>,
    /// Layer strokes go to.
    pub active: usize,
    /// Baked mesh maps of the painted object (not saved; re-baked on demand).
    #[serde(skip)]
    pub mesh_maps: Option<Arc<MeshMaps>>,
    #[serde(skip, default = "crate::scene::next_version")]
    id: u64,
    #[serde(skip, default = "crate::scene::next_version")]
    rev: u64,
}

/// Stacks compare by identity and revision: every edit must call
/// [`PaintStack::touch`], and comparing two materials never walks texels.
impl PartialEq for PaintStack {
    fn eq(&self, o: &Self) -> bool {
        self.id == o.id && self.rev == o.rev
    }
}

impl PaintStack {
    /// A stack with one empty "Paint Layer" over the base material.
    pub fn new(size: u32) -> Self {
        let size = Tiles::<u8>::new(size).size();
        Self {
            size,
            layers: vec![PaintLayer::paint("Paint Layer", size)],
            active: 0,
            mesh_maps: None,
            id: crate::scene::next_version(),
            rev: crate::scene::next_version(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Marks the stack as edited (viewport and renders pick it up).
    pub fn touch(&mut self) {
        self.rev = crate::scene::next_version();
    }

    pub fn active_layer(&self) -> Option<&PaintLayer> {
        self.layers.get(self.active)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut PaintLayer> {
        self.touch();
        self.layers.get_mut(self.active)
    }

    /// Adds `layer` above the active one and makes it active.
    pub fn add(&mut self, layer: PaintLayer) -> usize {
        let at = (self.active + 1).min(self.layers.len());
        self.layers.insert(at, layer);
        self.active = at;
        self.touch();
        at
    }

    /// Deletes layer `i` (the base material cannot be deleted).
    pub fn remove(&mut self, i: usize) {
        if i < self.layers.len() {
            self.layers.remove(i);
            self.active = self.active.min(self.layers.len().saturating_sub(1));
            self.touch();
        }
    }

    /// Moves layer `i` one step up (`true`) or down the stack.
    pub fn shift(&mut self, i: usize, up: bool) {
        let j = if up { i + 1 } else { i.wrapping_sub(1) };
        if i < self.layers.len() && j < self.layers.len() {
            self.layers.swap(i, j);
            if self.active == i {
                self.active = j;
            } else if self.active == j {
                self.active = i;
            }
            self.touch();
        }
    }

    /// A free name like "Paint Layer 2".
    pub fn unique_name(&self, base: &str) -> String {
        let taken = |n: &str| self.layers.iter().any(|l| l.name == n);
        if !taken(base) {
            return base.to_string();
        }
        (2..).map(|k| format!("{base} {k}")).find(|n| !taken(n)).expect("endless names")
    }

    /// True when any visible layer uses Height-Blend (the base height is
    /// then needed while folding).
    pub fn needs_base_height(&self) -> bool {
        self.layers.iter().any(|l| l.visible && l.blend == BlendMode::HeightBlend)
    }

    /// Texel memory of all layers, in bytes.
    pub fn bytes(&self) -> usize {
        self.layers.iter().map(|l| l.bytes()).sum()
    }
}
