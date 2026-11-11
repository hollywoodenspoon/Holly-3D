//! Weathered Surfaces: materials built with the Procedural Pattern &
//! Weathering rollout (pattern + style + weathering sliders), so each one
//! derives its own normal map, roughness and cavity AO.

use crate::materials::Material;
use crate::materials::proc_engine::{Pattern, ProcSurface};

pub struct ProcSpec {
    pub name: &'static str,
    pub pattern: Pattern,
    pub style: u8,
    pub tiling: f32,
    /// Edge wear, cavity dirt, moss, scorch / rust, wetness (0..1).
    pub weather: [f32; 5],
    /// Surface and gap colours, when not the style's own.
    pub colors: Option<([f32; 3], [f32; 3])>,
}

const fn w(name: &'static str, pattern: Pattern, style: u8, tiling: f32, weather: [f32; 5]) -> ProcSpec {
    ProcSpec { name, pattern, style, tiling, weather, colors: None }
}

impl ProcSpec {
    const fn tint(mut self, a: [f32; 3], b: [f32; 3]) -> ProcSpec {
        self.colors = Some((a, b));
        self
    }

    pub fn material(&self) -> Material {
        let mut m = Material::named(self.name);
        m.set_pattern(self.pattern, self.style);
        let [edge_wear, cavity_dirt, moss, scorch, wetness] = self.weather;
        m.proc = ProcSurface { tiling: self.tiling, edge_wear, cavity_dirt, moss, scorch, wetness, ..m.proc };
        if let Some((a, b)) = self.colors {
            (m.proc.color_a, m.proc.color_b) = (a, b);
        }
        m.base_color = m.proc.color_a;
        m
    }
}

use Pattern as P;

pub const WEATHERED: &[ProcSpec] = &[
    w("Clean Brick Wall", P::Bricks, 0, 1.0, [0.0, 0.0, 0.0, 0.0, 0.0]),
    w("Old Brick Wall", P::Bricks, 0, 1.0, [0.5, 0.45, 0.0, 0.0, 0.0]),
    w("Mossy Brick Wall", P::Bricks, 0, 1.0, [0.3, 0.3, 0.55, 0.0, 0.0]),
    w("Burnt Brick Ruin", P::Bricks, 0, 1.0, [0.6, 0.4, 0.0, 0.7, 0.0]),
    w("Bathroom Tiles", P::Bricks, 1, 2.0, [0.0, 0.15, 0.0, 0.0, 0.0]),
    w("Grimy Subway Tiles", P::Bricks, 1, 2.0, [0.2, 0.7, 0.0, 0.0, 0.2]),
    w("Herringbone Brick Paving", P::Bricks, 2, 1.0, [0.3, 0.3, 0.0, 0.0, 0.0]),
    w("Castle Stone Wall", P::Bricks, 3, 1.0, [0.4, 0.3, 0.0, 0.0, 0.0]),
    w("Overgrown Castle Wall", P::Bricks, 3, 1.0, [0.5, 0.4, 0.7, 0.0, 0.0]),
    w("Siege-Scorched Castle Wall", P::Bricks, 3, 1.0, [0.7, 0.3, 0.0, 0.65, 0.0]),
    w("Wet Cobblestone Street", P::Cobbles, 0, 1.0, [0.2, 0.3, 0.0, 0.0, 0.7]),
    w("Mossy Cobblestones", P::Cobbles, 0, 1.0, [0.2, 0.3, 0.6, 0.0, 0.0]),
    w("Worn Flagstone Floor", P::Cobbles, 1, 1.0, [0.6, 0.35, 0.0, 0.0, 0.0]),
    w("Cracked Cliff Rock", P::Rock, 0, 1.0, [0.3, 0.3, 0.0, 0.0, 0.0]),
    w("Dry Lake Bed", P::Rock, 1, 1.0, [0.0, 0.2, 0.0, 0.0, 0.0]),
    w("Muddy Cracked Earth", P::Rock, 1, 1.0, [0.0, 0.3, 0.0, 0.0, 0.65]),
    w("Lichen Rock Plates", P::Rock, 2, 1.0, [0.3, 0.2, 0.4, 0.0, 0.0]),
    w("Weathered Deck Planks", P::Wood, 0, 1.0, [0.5, 0.4, 0.0, 0.0, 0.0]).tint([0.52, 0.47, 0.4], [0.16, 0.13, 0.1]),
    w("Charred Timber Planks", P::Wood, 0, 1.0, [0.3, 0.2, 0.0, 0.85, 0.0]),
    w("Rain-Soaked Boardwalk", P::Wood, 0, 1.0, [0.2, 0.2, 0.15, 0.0, 0.75]),
    w("Rusty Chainmail", P::Weave, 1, 1.0, [0.0, 0.2, 0.0, 0.5, 0.0]),
    w("Stained Canvas", P::Weave, 0, 1.0, [0.0, 0.5, 0.0, 0.0, 0.0]),
    w("Worn Carbon Fibre", P::Weave, 2, 1.0, [0.5, 0.1, 0.0, 0.0, 0.0]),
    w("Worn Brushed Steel", P::Metal, 0, 1.0, [0.4, 0.3, 0.0, 0.0, 0.0]),
    w("Rusty Diamond Plate", P::Metal, 1, 1.0, [0.5, 0.35, 0.0, 0.55, 0.0]),
    w("Battle-Scarred Armour Plate", P::Metal, 2, 1.0, [0.7, 0.45, 0.0, 0.3, 0.0]).tint([0.42, 0.44, 0.46], [0.2, 0.2, 0.22]),
    w("Wet Asphalt with Puddles", P::Asphalt, 0, 1.0, [0.0, 0.2, 0.0, 0.0, 0.8]),
    w("Old Road with Centre Line", P::Asphalt, 1, 1.0, [0.5, 0.4, 0.0, 0.0, 0.0]),
    w("Scorched Road with Dashed Line", P::Asphalt, 2, 1.0, [0.3, 0.3, 0.0, 0.5, 0.0]),
    w("Meadow Grass", P::Terrain, 0, 1.0, [0.0, 0.25, 0.0, 0.0, 0.0]),
    w("Muddy Trail", P::Terrain, 1, 1.0, [0.0, 0.2, 0.2, 0.0, 0.6]),
    w("Forest Moss", P::Terrain, 2, 1.0, [0.0, 0.3, 0.0, 0.0, 0.25]),
];
