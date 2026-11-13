//! One-click material presets (Quick Preset dropdown).

use super::maps::{MapChannel, MapNode, NoiseKind};
use super::{Material, OutlineParams, ShadingModel, ToonParams};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Preset {
    Clay,
    BrushedSteel,
    PolishedGold,
    MatteRubber,
    Wood,
    Crystal,
    CelShaded,
}

impl Preset {
    pub const ALL: [Preset; 7] =
        [Preset::Clay, Preset::BrushedSteel, Preset::PolishedGold, Preset::MatteRubber, Preset::Wood, Preset::Crystal, Preset::CelShaded];

    pub fn label(self) -> &'static str {
        match self {
            Preset::Clay => "Default Modeling Clay",
            Preset::BrushedSteel => "Brushed Steel / Gunmetal",
            Preset::PolishedGold => "Polished Gold / Chrome",
            Preset::MatteRubber => "Matte Rubber / Vulcanized Plastic",
            Preset::Wood => "Natural Wood / Procedural Grain",
            Preset::Crystal => "Translucent Crystal / Quartz Gem",
            Preset::CelShaded => "Cel-Shaded Anime / Tactics Style",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Preset::Clay => "Clay",
            Preset::BrushedSteel => "Brushed Steel",
            Preset::PolishedGold => "Polished Gold",
            Preset::MatteRubber => "Matte Rubber",
            Preset::Wood => "Natural Wood",
            Preset::Crystal => "Quartz Crystal",
            Preset::CelShaded => "Cel Shaded",
        }
    }

    pub fn material(self) -> Material {
        let mut m = Material::named(self.short());
        self.apply(&mut m);
        m
    }

    /// Applies the preset's look, keeping the material's name and viewport flag.
    pub fn apply(self, m: &mut Material) {
        let keep = (m.name.clone(), m.show_in_viewport);
        *m = Material::default();
        (m.name, m.show_in_viewport) = keep;
        match self {
            Preset::Clay => {
                m.base_color = [0.72, 0.68, 0.62];
                m.roughness = 0.7;
            }
            Preset::BrushedSteel => {
                m.base_color = [0.56, 0.57, 0.6];
                (m.metallic, m.roughness) = (0.90, 0.25);
            }
            Preset::PolishedGold => {
                m.base_color = [1.0, 0.77, 0.34];
                (m.metallic, m.roughness) = (1.0, 0.05);
            }
            Preset::MatteRubber => {
                m.base_color = [0.12, 0.12, 0.13];
                (m.metallic, m.roughness) = (0.0, 0.85);
            }
            Preset::Wood => {
                m.base_color = [0.62, 0.42, 0.24];
                m.roughness = 0.55;
                m.map_mut(MapChannel::BaseColor).node = Some(MapNode::new_kind(4, None));
                m.map_mut(MapChannel::Roughness).node =
                    Some(MapNode::Noise { kind: NoiseKind::Fractal, scale: 6.0, seed: 2, color_a: [0.45; 3], color_b: [0.75; 3] });
                m.map_mut(MapChannel::Roughness).amount = 60.0;
            }
            Preset::Crystal => {
                m.model = ShadingModel::Crystal;
                m.base_color = [0.93, 0.95, 1.0];
                (m.transmission, m.ior, m.roughness) = (0.92, 1.55, 0.06);
                m.thickness = 0.35;
                m.attenuation_color = [0.82, 0.74, 0.98];
            }
            Preset::CelShaded => {
                m.model = ShadingModel::Toon;
                m.base_color = [0.95, 0.55, 0.42];
                m.roughness = 0.5;
                m.toon = ToonParams { steps: 2, shadow_tint: [0.55, 0.45, 0.7], rim: 0.6, rim_color: [1.0, 0.95, 0.85], specular: 0.4, ..Default::default() };
                m.outline = OutlineParams { enabled: true, width: 2.0, color: [0.08, 0.05, 0.1], crease_lines: true, crease_angle: 45.0, pen_lines: true };
            }
        }
    }
}
