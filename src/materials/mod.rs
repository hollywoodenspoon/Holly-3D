//! Materials: the material library (24 Compact Material Editor
//! slots), shading models, procedural / bitmap maps and the universal ink
//! outline settings shared by every material.

pub mod bake;
pub mod ghost_pack;
pub mod import;
pub mod library;
pub mod maps;
pub mod multi;
pub mod patterns;
pub mod presets;
pub mod principled;
pub mod proc_engine;
pub mod relief;
pub mod preview;
pub mod shading;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_multi;
pub(crate) mod tex_serde;
pub mod texture;
pub mod triplanar;

pub use maps::{MapChannel, MapNode, MapSlot};
pub use multi::{MultiPreset, MultiSub, SubMaterial};
pub use presets::Preset;
pub use texture::{TexHandle, Texture};
pub use triplanar::{Mapping, Projection};

use slotmap::{SlotMap, new_key_type};

new_key_type! {
    pub struct MatId;
}

pub const SLOT_COUNT: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ShadingModel {
    #[default]
    Physical,
    Toon,
    Crystal,
    Unlit,
}

impl ShadingModel {
    pub const ALL: [ShadingModel; 4] = [ShadingModel::Physical, ShadingModel::Toon, ShadingModel::Crystal, ShadingModel::Unlit];
    pub fn label(self) -> &'static str {
        match self {
            ShadingModel::Physical => "Standard / Physical PBR",
            ShadingModel::Toon => "Cel / Toon Ramp",
            ShadingModel::Crystal => "Refractive Crystal / Glass",
            ShadingModel::Unlit => "Unlit / Emissive",
        }
    }
}

/// Cel / Toon ramp parameters.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ToonParams {
    /// Number of light bands (2 = classic anime two-tone).
    pub steps: u32,
    /// Color multiplied into the base color on the shadow band (sRGB).
    pub shadow_tint: [f32; 3],
    pub rim: f32,
    pub rim_color: [f32; 3],
    pub specular: f32,
    /// Manga screentone: the shadow band renders as a halftone dot grid.
    pub screentone: bool,
    /// Halftone cell size in pixels.
    pub screentone_scale: f32,
}

impl Default for ToonParams {
    fn default() -> Self {
        Self { steps: 2, shadow_tint: [0.55, 0.5, 0.72], rim: 0.0, rim_color: [1.0, 1.0, 1.0], specular: 0.0, screentone: false, screentone_scale: 6.0 }
    }
}

/// Universal ink outline (inverted hull + sharp-edge lines).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct OutlineParams {
    pub enabled: bool,
    /// 0..=10 (scene units x 0.005 in the viewport hull; pixels in renders).
    pub width: f32,
    pub color: [f32; 3],
    pub crease_lines: bool,
    /// Dihedral angle (degrees) above which an edge draws as an ink line.
    pub crease_angle: f32,
    /// Interior pen lines: soft-curvature valleys / ridges and material
    /// seams such as Panel Groove floors.
    pub pen_lines: bool,
}

impl Default for OutlineParams {
    fn default() -> Self {
        Self { enabled: false, width: 2.0, color: [0.05, 0.05, 0.07], crease_lines: false, crease_angle: 40.0, pen_lines: false }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Material {
    pub name: String,
    pub model: ShadingModel,
    /// Diffuse / base color (sRGB 0..1).
    pub base_color: [f32; 3],
    pub opacity: f32,
    pub metallic: f32,
    pub roughness: f32,
    /// Specular transmission (glass / crystal).
    pub transmission: f32,
    pub ior: f32,
    /// Thickness used for absorption (scene units).
    pub thickness: f32,
    /// Subsurface / absorption color (sRGB) light picks up inside the volume.
    pub attenuation_color: [f32; 3],
    /// Luminance / Emission color (sRGB).
    pub emissive: [f32; 3],
    /// Emission Multiplier (0..100); above 0 the surface is an area light in F9.
    pub emissive_strength: f32,
    pub toon: ToonParams,
    pub outline: OutlineParams,
    /// Maps rollout: one slot per [`MapChannel`].
    pub maps: [MapSlot; 6],
    /// Free (unwired) nodes on the Node Graph canvas.
    pub slate_nodes: Vec<MapNode>,
    /// "Show Shaded Material in Viewport".
    pub show_in_viewport: bool,
    /// Multi/Sub-Object: sub-materials by polygon material ID.
    #[serde(default)]
    pub multi: Option<MultiSub>,
    /// "Procedural Pattern & Weathering" rollout (inactive with no pattern).
    #[serde(default)]
    pub proc: proc_engine::ProcSurface,
    /// "Preserve Texture Appearance (Decouple Luminance)": the Emission
    /// Multiplier still lights the scene, but the camera sees the surface
    /// in its own colours at `direct_camera_brightness`.
    #[serde(default)]
    pub preserve_emission_texture: bool,
    /// Brightness of the surface as the camera sees it (0..2) while
    /// `preserve_emission_texture` is on.
    #[serde(default = "one")]
    pub direct_camera_brightness: f32,
    /// UV channel or World / Triplanar mapping of the maps and patterns.
    #[serde(default)]
    pub projection: Projection,
    /// Paint layers over this material (it stays "Layer 0: Base Material").
    #[serde(default)]
    pub paint: Option<crate::paint::PaintStack>,
    /// Principled extras: SSS, anisotropy, clearcoat, sheen, absorption, dispersion.
    #[serde(default)]
    pub principled: principled::Principled,
    /// Shadow & Reflection Catcher: renders transparent except for the
    /// shadows, AO and reflections other objects put on it.
    #[serde(default)]
    pub shadow_catcher: bool,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            model: ShadingModel::Physical,
            base_color: [0.6, 0.6, 0.6],
            opacity: 1.0,
            metallic: 0.0,
            roughness: 0.6,
            transmission: 0.0,
            ior: 1.5,
            thickness: 0.2,
            attenuation_color: [1.0, 1.0, 1.0],
            emissive: [1.0, 1.0, 1.0],
            emissive_strength: 0.0,
            toon: ToonParams::default(),
            outline: OutlineParams::default(),
            maps: Default::default(),
            slate_nodes: vec![],
            show_in_viewport: true,
            multi: None,
            proc: Default::default(),
            preserve_emission_texture: false,
            direct_camera_brightness: 1.0,
            projection: Default::default(),
            paint: None,
            principled: Default::default(),
            shadow_catcher: false,
        }
    }
}

fn one() -> f32 {
    1.0
}

impl Material {
    /// True when the camera should see the unblown surface instead of the
    /// material's full emission.
    pub fn decoupled(&self) -> bool {
        self.preserve_emission_texture && self.emissive_strength > 0.0
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Self::default() }
    }

    pub fn map(&self, ch: MapChannel) -> &MapSlot {
        &self.maps[ch as usize]
    }

    pub fn map_mut(&mut self, ch: MapChannel) -> &mut MapSlot {
        &mut self.maps[ch as usize]
    }

    /// Picks a procedural pattern and style (one click): the style's
    /// colours, gap and bevel, plus its roughness and metalness.
    pub fn set_pattern(&mut self, pattern: proc_engine::Pattern, style: u8) {
        self.proc.restyle(pattern, style);
        if let Some(s) = self.proc.style_def() {
            (self.roughness, self.metallic) = (s.rough, s.metal);
        }
    }

    /// Slot whose map is active (enabled with a node).
    pub fn active_map(&self, ch: MapChannel) -> Option<(&MapNode, f32)> {
        let s = self.map(ch);
        s.node.as_ref().filter(|_| s.enabled && s.amount > 0.0).map(|n| (n, s.amount / 100.0))
    }
}

/// Scene material library + the 24 editor sample slots.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MaterialLib {
    pub mats: SlotMap<MatId, Material>,
    pub slots: Vec<MatId>,
}

impl Default for MaterialLib {
    fn default() -> Self {
        let mut mats = SlotMap::with_key();
        let slots = (0..SLOT_COUNT)
            .map(|i| {
                let m = match Preset::ALL.get(i) {
                    Some(p) => p.material(),
                    None => Material::named("Default"),
                };
                mats.insert(Self::numbered(i, m))
            })
            .collect();
        Self { mats, slots }
    }
}

impl MaterialLib {
    /// "01 - Name" (slot naming).
    fn numbered(i: usize, mut m: Material) -> Material {
        m.name = format!("{:02} - {}", i + 1, m.name);
        m
    }

    pub fn slot(&self, i: usize) -> MatId {
        self.slots[i.min(SLOT_COUNT - 1)]
    }

    /// Reset Material Slot: a fresh default in the slot (scene objects keep theirs).
    pub fn reset_slot(&mut self, i: usize) -> MatId {
        let id = self.mats.insert(Self::numbered(i, Material::named("Default")));
        self.slots[i] = id;
        id
    }

    /// Make Material Copy: the slot gets an independent ("cool") copy.
    pub fn copy_slot(&mut self, i: usize) -> MatId {
        let mut m = self.mats[self.slot(i)].clone();
        if !m.name.ends_with(" Copy") {
            m.name.push_str(" Copy");
        }
        let id = self.mats.insert(m);
        self.slots[i] = id;
        id
    }

    /// Puts an existing material into slot `i` (Pick Material from Object).
    pub fn put_in_slot(&mut self, i: usize, id: MatId) {
        if self.mats.contains_key(id) {
            self.slots[i] = id;
        }
    }

    pub fn slot_of(&self, id: MatId) -> Option<usize> {
        self.slots.iter().position(|&s| s == id)
    }
}
