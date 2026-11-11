//! The Compact editor's pattern list: each [`Pattern`] has a few styles,
//! and each style says which generator it runs, how Tiling drives it, and
//! its one-click colours, roughness and metalness.

use super::generators::TileKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Pattern {
    /// No procedural surface: the material's own colour and maps.
    #[default]
    None,
    Bricks,
    Cobbles,
    Rock,
    Wood,
    Weave,
    Metal,
    Asphalt,
    Terrain,
}

/// Something drawn on top of the generator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extra {
    None,
    /// Wood grain running along each board.
    Grain,
    /// A painted road line down the middle of the tile (dashes per tile, 0 = solid).
    Line(f32, [f32; 3]),
}

pub struct Style {
    pub label: &'static str,
    pub kind: TileKind,
    /// Generator knobs at Tiling 1: `[scale, detail, width, warp]`.
    pub knobs: [f32; 4],
    /// Tiling also multiplies the detail knob (rows follow columns).
    pub detail_scales: bool,
    /// Gap and Bevel spinners drive the width and warp knobs.
    pub user_gap: bool,
    /// Surface colour and gap / secondary colour (sRGB).
    pub colors: ([f32; 3], [f32; 3]),
    pub rough: f32,
    pub metal: f32,
    pub extra: Extra,
}

const fn st(label: &'static str, kind: TileKind, knobs: [f32; 4], a: [f32; 3], b: [f32; 3], rough: f32) -> Style {
    Style { label, kind, knobs, detail_scales: false, user_gap: false, colors: (a, b), rough, metal: 0.0, extra: Extra::None }
}

impl Style {
    const fn rows(mut self) -> Style {
        self.detail_scales = true;
        self
    }
    const fn gaps(mut self) -> Style {
        self.user_gap = true;
        self
    }
    const fn metal(mut self, m: f32) -> Style {
        self.metal = m;
        self
    }
    const fn extra(mut self, e: Extra) -> Style {
        self.extra = e;
        self
    }
}

use TileKind as K;
const MORTAR: [f32; 3] = [0.74, 0.72, 0.67];
const TAR: [f32; 3] = [0.09, 0.09, 0.1];

const BRICKS: &[Style] = &[
    st("Running Bond Brick", K::Bricks, [4.0, 8.0, 0.1, 0.15], [0.6, 0.26, 0.17], MORTAR, 0.85).rows().gaps(),
    st("Stack Bond Tiles", K::StackTiles, [4.0, 4.0, 0.06, 0.08], [0.86, 0.84, 0.8], [0.5, 0.5, 0.48], 0.35).rows().gaps(),
    st("Herringbone Paving", K::Herringbone, [2.0, 3.0, 0.08, 0.15], [0.56, 0.3, 0.22], [0.42, 0.4, 0.37], 0.85).gaps(),
    st("Medieval Coursed Stone", K::Coursed, [3.0, 6.0, 0.1, 0.2], [0.58, 0.56, 0.5], [0.36, 0.34, 0.3], 0.9).rows().gaps(),
];
const COBBLES: &[Style] = &[
    st("Cobblestones", K::Cobbles, [6.0, 0.4, 0.12, 0.2], [0.5, 0.47, 0.43], [0.24, 0.22, 0.19], 0.8).gaps(),
    st("Flagstones", K::Plates, [4.0, 0.3, 0.06, 0.4], [0.56, 0.53, 0.47], [0.28, 0.26, 0.22], 0.85).gaps(),
];
const ROCK: &[Style] = &[
    st("Cracked Rock", K::Cracks, [5.0, 0.5, 0.05, 0.6], [0.47, 0.44, 0.41], [0.12, 0.11, 0.1], 0.9).gaps(),
    st("Cracked Earth", K::Crackle, [6.0, 0.9, 0.06, 0.5], [0.62, 0.48, 0.33], [0.2, 0.14, 0.09], 0.95).gaps(),
    st("Rock Plates", K::Plates, [5.0, 0.8, 0.04, 0.9], [0.4, 0.38, 0.36], [0.14, 0.13, 0.12], 0.85).gaps(),
];
const WOOD: &[Style] = &[
    st("Floor Planks", K::Planks, [2.0, 8.0, 0.04, 0.08], [0.66, 0.48, 0.29], [0.2, 0.13, 0.08], 0.6).rows().gaps().extra(Extra::Grain),
    st("Solid Wood Grain", K::WoodGrain, [10.0, 8.0, 2.0, 0.6], [0.72, 0.53, 0.33], [0.42, 0.27, 0.14], 0.55),
];
const WEAVE: &[Style] = &[
    st("Woven Fabric", K::Weave, [32.0, 0.0, 0.85, 0.0], [0.62, 0.56, 0.46], [0.24, 0.21, 0.17], 0.95),
    st("Chainmail", K::Chainmail, [16.0, 0.0, 0.18, 0.9], [0.66, 0.67, 0.7], [0.07, 0.07, 0.08], 0.4).metal(1.0),
    st("Carbon Fibre", K::CarbonFiber, [8.0, 6.0, 0.08, 0.0], [0.14, 0.14, 0.15], [0.02, 0.02, 0.02], 0.3),
];
const METAL: &[Style] = &[
    st("Brushed Metal", K::BrushedMetal, [160.0, 2.0, 2.0, 0.6], [0.74, 0.74, 0.76], [0.52, 0.52, 0.54], 0.3).metal(1.0),
    st("Diamond Plate", K::DiamondPlate, [8.0, 0.7, 0.22, 0.0], [0.76, 0.76, 0.78], [0.5, 0.5, 0.52], 0.35).metal(1.0),
    st("Scratched Metal", K::Scratches, [8.0, 0.6, 0.012, 0.25], [0.86, 0.86, 0.88], [0.56, 0.56, 0.58], 0.3).metal(1.0),
];
const ASPHALT: &[Style] = &[
    st("Asphalt", K::Asphalt, [48.0, 0.4, 0.15, 0.5], [0.24, 0.24, 0.25], TAR, 0.9),
    st("Asphalt + White Centre Line", K::Asphalt, [48.0, 0.4, 0.15, 0.5], [0.24, 0.24, 0.25], TAR, 0.9).extra(Extra::Line(0.0, [0.92, 0.92, 0.9])),
    st("Asphalt + Dashed Yellow Line", K::Asphalt, [48.0, 0.4, 0.15, 0.5], [0.24, 0.24, 0.25], TAR, 0.9).extra(Extra::Line(3.0, [0.92, 0.72, 0.12])),
];
const TERRAIN: &[Style] = &[
    st("Grass", K::Fbm, [12.0, 5.0, 0.6, 0.3], [0.34, 0.52, 0.16], [0.16, 0.3, 0.08], 0.9),
    st("Dirt", K::Fbm, [5.0, 6.0, 0.55, 0.2], [0.5, 0.39, 0.27], [0.3, 0.22, 0.14], 0.95),
    st("Moss", K::Ridged, [8.0, 5.0, 0.6, 0.0], [0.34, 0.46, 0.16], [0.14, 0.24, 0.07], 0.95),
];

impl Pattern {
    pub const ALL: [Pattern; 9] = [
        Pattern::None,
        Pattern::Bricks,
        Pattern::Cobbles,
        Pattern::Rock,
        Pattern::Wood,
        Pattern::Weave,
        Pattern::Metal,
        Pattern::Asphalt,
        Pattern::Terrain,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Pattern::None => "None (Solid / Bitmap)",
            Pattern::Bricks => "Bricks & Masonry",
            Pattern::Cobbles => "Cobblestone & Flagstone",
            Pattern::Rock => "Voronoi Rock & Cracks",
            Pattern::Wood => "Wood Planks & Grain",
            Pattern::Weave => "Weave / Chainmail / Fabric",
            Pattern::Metal => "Brushed / Diamond Metal",
            Pattern::Asphalt => "Asphalt & Road Markings",
            Pattern::Terrain => "Terrain Grass / Dirt / Moss",
        }
    }

    /// The styles of this pattern (empty for `None`).
    pub fn styles(self) -> &'static [Style] {
        match self {
            Pattern::None => &[],
            Pattern::Bricks => BRICKS,
            Pattern::Cobbles => COBBLES,
            Pattern::Rock => ROCK,
            Pattern::Wood => WOOD,
            Pattern::Weave => WEAVE,
            Pattern::Metal => METAL,
            Pattern::Asphalt => ASPHALT,
            Pattern::Terrain => TERRAIN,
        }
    }

    /// Style `i` of this pattern (clamped), or `None` for no pattern.
    pub fn style(self, i: u8) -> Option<&'static Style> {
        let s = self.styles();
        s.get((i as usize).min(s.len().saturating_sub(1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pattern_but_none_has_named_styles_with_valid_knobs() {
        let mut names = std::collections::HashSet::new();
        for p in Pattern::ALL {
            assert_eq!(p.styles().is_empty(), p == Pattern::None, "{}", p.label());
            assert_eq!(p.style(0).is_none(), p == Pattern::None);
            for s in p.styles() {
                assert!(names.insert(s.label), "duplicate style {}", s.label);
                assert!(s.knobs[0] >= 1.0 && (0.0..=1.0).contains(&s.rough) && (0.0..=1.0).contains(&s.metal));
                for (k, (_, lo, hi, _)) in s.knobs.iter().zip(s.kind.knobs()) {
                    assert!(*k >= lo.min(0.0) && *k <= hi, "{}: knob {k} outside {lo}..{hi}", s.label);
                }
            }
        }
        assert_eq!(Pattern::Bricks.style(99).unwrap().label, "Medieval Coursed Stone", "out-of-range styles clamp");
    }
}
