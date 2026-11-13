//! Multi/Sub-Object material: a list of sub-materials keyed by polygon
//! material ID (IDs above the highest wrap around the
//! list, a sub-material that is switched off renders as the default gray),
//! plus the pre-built Medieval Castle, Modern City and Alpine Terrain packs.

use super::{Material, Preset, ShadingModel};
use std::sync::LazyLock;

/// Highest sub-material count `Set Number` accepts.
pub const MAX_SUBS: usize = 100;

/// Fresh Multi/Sub-Object materials start with 10 slots.
pub const DEFAULT_SUBS: usize = 10;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubMaterial {
    /// Polygon material ID this sub-material renders (1-based).
    pub id: u16,
    pub name: String,
    /// `[ On/Off ]`: off renders the polygons with the default material.
    pub enabled: bool,
    pub mat: Material,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MultiSub {
    pub subs: Vec<SubMaterial>,
}

/// What a disabled sub-material (or an empty list) renders as.
static OFF: LazyLock<Material> = LazyLock::new(|| Material::named("Default"));

fn sub(id: u16, mat: Material) -> SubMaterial {
    SubMaterial { id, name: mat.name.clone(), enabled: true, mat }
}

impl MultiSub {
    /// `n` default sub-materials with IDs 1..=n.
    pub fn new(n: usize) -> Self {
        let mut m = Self::default();
        m.set_number(n);
        m
    }

    /// `[ Set Number ]`: grows with fresh defaults (next free IDs) or drops
    /// sub-materials from the end.
    pub fn set_number(&mut self, n: usize) {
        let n = n.clamp(1, MAX_SUBS);
        self.subs.truncate(n);
        while self.subs.len() < n {
            let id = self.subs.iter().map(|s| s.id).max().unwrap_or(0) + 1;
            let shade = 0.35 + 0.5 * ((id as f32 * 0.618).fract());
            let mat = Material { base_color: [shade, shade * 0.95, shade * 0.9], ..Material::named(format!("Material #{id}")) };
            self.subs.push(sub(id, mat));
        }
    }

    /// Index of the sub-material that renders material ID `id`: an exact
    /// match, else the ID wrapped around the list.
    pub fn index_for(&self, id: u16) -> Option<usize> {
        if self.subs.is_empty() {
            return None;
        }
        self.subs.iter().position(|s| s.id == id).or_else(|| Some((id.max(1) as usize - 1) % self.subs.len()))
    }

    /// Material for polygons with material ID `id`.
    pub fn resolve(&self, id: u16) -> &Material {
        match self.index_for(id).map(|i| &self.subs[i]) {
            Some(s) if s.enabled => &s.mat,
            _ => &OFF,
        }
    }
}

impl Material {
    pub fn is_multi(&self) -> bool {
        self.multi.is_some()
    }

    /// Material that renders polygons with material ID `id` (the material
    /// itself unless it is a Multi/Sub-Object).
    pub fn for_id(&self, id: u16) -> &Material {
        match &self.multi {
            Some(ms) => ms.resolve(id),
            None => self,
        }
    }

    /// Material the editor's parameter rollouts edit: sub-material `sub` of
    /// a Multi/Sub-Object, else the material itself.
    pub fn target(&self, sub: usize) -> &Material {
        match &self.multi {
            Some(ms) if !ms.subs.is_empty() => &ms.subs[sub.min(ms.subs.len() - 1)].mat,
            _ => self,
        }
    }

    pub fn target_mut(&mut self, sub: usize) -> &mut Material {
        if self.multi.as_ref().is_some_and(|ms| !ms.subs.is_empty()) {
            let ms = self.multi.as_mut().expect("checked");
            let i = sub.min(ms.subs.len() - 1);
            return &mut ms.subs[i].mat;
        }
        self
    }

    /// Standard -> Multi/Sub-Object: the old material is kept as ID 1.
    pub fn make_multi(&mut self) {
        if self.multi.is_some() {
            return;
        }
        let mut ms = MultiSub::new(DEFAULT_SUBS);
        let mut old = self.clone();
        old.name = old.name.split(" - ").last().unwrap_or(&old.name).to_string();
        ms.subs[0] = sub(1, old);
        self.multi = Some(ms);
    }

    /// Multi/Sub-Object -> Standard: sub-material `keep` becomes the material.
    pub fn make_standard(&mut self, keep: usize) {
        let Some(ms) = self.multi.take() else { return };
        if let Some(s) = ms.subs.get(keep.min(ms.subs.len().saturating_sub(1))) {
            let (name, show) = (self.name.clone(), self.show_in_viewport);
            *self = s.mat.clone();
            (self.name, self.show_in_viewport) = (name, show);
        }
    }
}

/// Pre-built Multi/Sub-Object packs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiPreset {
    MedievalCastle,
    ModernCity,
    AlpineTerrain,
    OceanWater,
    PlanetStudio,
    Settlement,
    /// Settlement with installed photo textures where available.
    SettlementPhoto,
    /// PR 8 Smoke & Fire plumes: fire core, smoke, wisps.
    SmokeFire,
    /// Phase 10A Character Pack: skin, eyes, hair, cloth, metal, leather.
    Character,
}

fn pbr(name: &str, rgb: [f32; 3], metallic: f32, roughness: f32) -> Material {
    Material { base_color: rgb, metallic, roughness, ..Material::named(name) }
}

fn glow(name: &str, rgb: [f32; 3], strength: f32) -> Material {
    Material { base_color: rgb, emissive: rgb, emissive_strength: strength, roughness: 0.4, ..Material::named(name) }
}

fn glass(name: &str, tint: [f32; 3], attenuation: [f32; 3], roughness: f32) -> Material {
    let mut m = Preset::Crystal.material();
    m.name = name.into();
    (m.base_color, m.attenuation_color, m.roughness, m.ior) = (tint, attenuation, roughness, 1.33);
    m
}

impl MultiPreset {
    pub const ALL: [MultiPreset; 9] = [
        Self::MedievalCastle,
        Self::ModernCity,
        Self::AlpineTerrain,
        Self::OceanWater,
        Self::PlanetStudio,
        Self::Settlement,
        Self::SettlementPhoto,
        Self::SmokeFire,
        Self::Character,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::MedievalCastle => "Medieval Castle Pack",
            Self::ModernCity => "Modern City Pack",
            Self::AlpineTerrain => "Alpine Terrain Pack",
            Self::OceanWater => "Ocean Water Pack",
            Self::PlanetStudio => "Planet Studio Pack",
            Self::Settlement => "Settlement Pack",
            Self::SettlementPhoto => "Settlement Pack (Photo Textures)",
            Self::SmokeFire => "Smoke & Fire Pack",
            Self::Character => "Character Pack",
        }
    }

    /// The pack's sub-materials, IDs 1..=N.
    pub fn build(self) -> MultiSub {
        let mats: Vec<Material> = match self {
            Self::MedievalCastle => vec![
                pbr("Castle Stone", [0.52, 0.5, 0.46], 0.0, 0.85),
                pbr("Mortar", [0.3, 0.28, 0.26], 0.0, 0.95),
                Material { name: "Oak Beams".into(), ..Preset::Wood.material() },
                pbr("Wrought Iron", [0.22, 0.22, 0.24], 0.85, 0.55),
                pbr("Slate Roof", [0.24, 0.27, 0.32], 0.0, 0.6),
                pbr("Banner Cloth", [0.62, 0.08, 0.1], 0.0, 0.9),
                glow("Torch Flame", [1.0, 0.55, 0.18], 8.0),
                glass("Leaded Glass", [0.85, 0.9, 0.8], [0.7, 0.8, 0.6], 0.12),
            ],
            Self::ModernCity => vec![
                pbr("Concrete", [0.58, 0.58, 0.56], 0.0, 0.9),
                pbr("Curtain-Wall Glass", [0.25, 0.35, 0.42], 0.6, 0.05),
                Material { name: "Brushed Aluminium".into(), ..Preset::BrushedSteel.material() },
                pbr("Asphalt", [0.1, 0.1, 0.11], 0.0, 0.95),
                pbr("Road Paint", [0.95, 0.85, 0.3], 0.0, 0.6),
                glow("Neon Sign", [0.2, 0.9, 1.0], 12.0),
                glow("Office Windows", [1.0, 0.86, 0.6], 4.0),
                pbr("Red Brick", [0.55, 0.22, 0.15], 0.0, 0.85),
            ],
            // IDs 1-4 match the Terrain generator's auto Material IDs.
            Self::AlpineTerrain => vec![
                pbr("Alpine Grass", [0.32, 0.48, 0.18], 0.0, 0.9),
                pbr("Granite Cliff", [0.45, 0.44, 0.43], 0.0, 0.8),
                pbr("Snow", [0.95, 0.96, 1.0], 0.0, 0.55),
                pbr("Riverbed Sand", [0.72, 0.64, 0.48], 0.0, 0.92),
                pbr("Pine Needles", [0.12, 0.26, 0.14], 0.0, 0.85),
                glass("Glacier Ice", [0.85, 0.95, 1.0], [0.55, 0.8, 0.95], 0.08),
                glass("Lake Water", [0.6, 0.78, 0.82], [0.2, 0.45, 0.5], 0.02),
                pbr("Dirt Path", [0.38, 0.28, 0.18], 0.0, 0.95),
            ],
            // The Ocean generator's IDs; the water tint and foam live in the
            // vertex colors, so both bases are white.
            Self::OceanWater => vec![pbr("Ocean Water", [1.0, 1.0, 1.0], 0.0, 0.06), pbr("Wave Foam", [1.0, 1.0, 1.0], 0.0, 0.85)],
            // Planet Studio layers: surface colors come from vertex colors,
            // cloud / atmosphere / ring density from vertex alpha.
            Self::PlanetStudio => vec![
                pbr("Ocean / Lowland", [1.0, 1.0, 1.0], 0.0, 0.2),
                pbr("Land", [1.0, 1.0, 1.0], 0.0, 0.9),
                pbr("Highland Rock", [1.0, 1.0, 1.0], 0.0, 0.85),
                pbr("Ice Caps", [1.0, 1.0, 1.0], 0.0, 0.45),
                Material { opacity: 0.999, ..pbr("Cloud Shell", [1.0, 1.0, 1.0], 0.0, 1.0) },
                Material { model: ShadingModel::Unlit, opacity: 0.999, ..pbr("Atmosphere Halo", [1.0, 1.0, 1.0], 0.0, 1.0) },
                Material { opacity: 0.999, ..pbr("Planetary Rings", [1.0, 1.0, 1.0], 0.0, 0.8) },
            ],
            // `generators::kit::ids`: Settlement, Medieval Architect, Battlefield.
            Self::Settlement => super::ghost_pack::materials(false),
            Self::SettlementPhoto => super::ghost_pack::materials(true),
            // Plume tint and density live in the vertex colours.
            Self::SmokeFire => vec![
                glow("Fire Core / Embers", [1.0, 0.5, 0.15], 8.0),
                Material { opacity: 0.999, ..pbr("Smoke", [1.0, 1.0, 1.0], 0.0, 1.0) },
                Material { opacity: 0.999, ..pbr("Smoke Wisps", [1.0, 1.0, 1.0], 0.0, 1.0) },
            ],
            // `character::params::ids`.
            Self::Character => vec![
                skin_material(),
                pbr("Eyes", [0.32, 0.22, 0.14], 0.0, 0.08),
                pbr("Hair", [0.18, 0.11, 0.06], 0.0, 0.55),
                pbr("Cloth", [0.3, 0.34, 0.45], 0.0, 0.85),
                Material { name: "Armor Steel".into(), ..Preset::BrushedSteel.material() },
                pbr("Leather", [0.36, 0.22, 0.12], 0.0, 0.7),
            ],
        };
        // Generated buildings and terrain have no UV layout: their packs
        // use World / Triplanar mapping, so patterns and photos tile.
        let triplanar = matches!(self, Self::MedievalCastle | Self::ModernCity | Self::AlpineTerrain | Self::Settlement | Self::SettlementPhoto);
        let place = |mut m: Material| {
            if triplanar {
                m.projection = super::Projection::triplanar(2.0);
            }
            m
        };
        MultiSub { subs: mats.into_iter().enumerate().map(|(i, m)| sub(i as u16 + 1, place(m))).collect() }
    }

    /// Turns `m` into this pack (keeps its name and viewport flag).
    pub fn apply(self, m: &mut Material) {
        let (name, show) = (m.name.clone(), m.show_in_viewport);
        *m = Material::named(self.label());
        (m.name, m.show_in_viewport) = (name, show);
        m.multi = Some(self.build());
    }
}

/// The Character Pack skin (subsurface scattering on).
pub fn skin_material() -> Material {
    let mut m = pbr("Skin", [0.86, 0.66, 0.55], 0.0, 0.5);
    m.principled.sss_weight = 0.35;
    m.principled.sss_color = [0.9, 0.35, 0.25];
    m.principled.sss_scale = 0.01;
    m
}
