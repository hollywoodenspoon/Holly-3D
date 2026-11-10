//! The Material Library: about 280 one-click materials in categories
//! (ground, roads, walls, roofs, metals, wood, stone, glass, plastic,
//! fabric, nature, glow, toon, sci-fi). Each is a short spec built into a
//! full [`Material`] with procedural maps, so they all work with no files.

mod build;
mod ground;
mod hard;
mod soft;
#[cfg(test)]
mod tests;

use super::maps::{MapChannel, MapNode, NoiseKind};
use super::patterns::{BrickKind, GrungeKind};
use super::{Material, OutlineParams, Preset, ShadingModel, ToonParams};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Classic,
    Ground,
    Roads,
    Walls,
    Roofs,
    Metals,
    Wood,
    Stone,
    Glass,
    Plastic,
    Fabric,
    Nature,
    Glow,
    Toon,
    SciFi,
}

impl Category {
    pub const ALL: [Category; 15] = [
        Category::Classic,
        Category::Ground,
        Category::Roads,
        Category::Walls,
        Category::Roofs,
        Category::Metals,
        Category::Wood,
        Category::Stone,
        Category::Glass,
        Category::Plastic,
        Category::Fabric,
        Category::Nature,
        Category::Glow,
        Category::Toon,
        Category::SciFi,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Classic => "Classic",
            Category::Ground => "Ground & Terrain",
            Category::Roads => "Roads & Urban",
            Category::Walls => "Walls & Facades",
            Category::Roofs => "Roofs",
            Category::Metals => "Metals",
            Category::Wood => "Wood",
            Category::Stone => "Stone & Concrete",
            Category::Glass => "Glass & Gems",
            Category::Plastic => "Plastic & Rubber",
            Category::Fabric => "Fabric & Leather",
            Category::Nature => "Nature & Organic",
            Category::Glow => "Glow / Emissive",
            Category::Toon => "Toon & Stylised",
            Category::SciFi => "Sci-Fi",
        }
    }

    fn specs(self) -> &'static [Spec] {
        match self {
            Category::Classic => &[],
            Category::Ground => ground::GROUND,
            Category::Roads => ground::ROADS,
            Category::Walls => build::WALLS,
            Category::Roofs => build::ROOFS,
            Category::Metals => hard::METALS,
            Category::Wood => hard::WOOD,
            Category::Stone => hard::STONE,
            Category::Glass => soft::GLASS,
            Category::Plastic => soft::PLASTIC,
            Category::Fabric => soft::FABRIC,
            Category::Nature => soft::NATURE,
            Category::Glow => soft::GLOW,
            Category::Toon => soft::TOON,
            Category::SciFi => soft::SCIFI,
        }
    }

    /// Everything in this category.
    pub fn items(self) -> Vec<Item> {
        match self {
            Category::Classic => Preset::ALL.into_iter().map(Item::Classic).collect(),
            c => c.specs().iter().map(Item::Lib).collect(),
        }
    }
}

/// One library material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    /// The original seven Quick Presets.
    Classic(Preset),
    Lib(&'static Spec),
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::Classic(p) => p.label(),
            Item::Lib(s) => s.name,
        }
    }

    pub fn material(self) -> Material {
        match self {
            Item::Classic(p) => p.material(),
            Item::Lib(s) => s.material(),
        }
    }
}

/// Every library item with its category, in browser order.
pub fn all() -> Vec<(Category, Item)> {
    Category::ALL.into_iter().flat_map(|c| c.items().into_iter().map(move |i| (c, i))).collect()
}

/// Finds an item by name (case-insensitive).
pub fn find(name: &str) -> Option<Item> {
    all().into_iter().map(|(_, i)| i).find(|i| i.name().eq_ignore_ascii_case(name))
}

/// The procedural look of a spec.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pat {
    Plain,
    /// Fractal noise from the colour to `c2`.
    Noise(f32, [f32; 3]),
    /// Cellular (Worley) noise: pebbles, scales, foam.
    Cells(f32, [f32; 3]),
    /// Kind, columns, rows, mortar gap, mortar colour.
    Bricks(BrickKind, f32, f32, f32, [f32; 3]),
    /// Count, width, dashes, corrugated, line colour (the spec colour is the ground).
    Stripes(f32, f32, f32, bool, [f32; 3]),
    /// Size, crack width, crack colour.
    Cracks(f32, f32, [f32; 3]),
    /// Kind, size, amount, dirt colour.
    Grunge(GrungeKind, f32, f32, [f32; 3]),
    /// Rings, dark wood colour.
    Wood(f32, [f32; 3]),
}

/// Shading beyond plain PBR.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fx {
    None,
    /// IOR and transmission.
    Glass(f32, f32),
    /// Emission multiplier.
    Glow(f32),
    /// Light bands and ink outline.
    Toon(u32, bool),
    Unlit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub name: &'static str,
    pub color: [f32; 3],
    pub metal: f32,
    pub rough: f32,
    pub pat: Pat,
    pub fx: Fx,
}

/// A plain-PBR spec; chain `.metal()` / `.fx()` for more.
pub const fn s(name: &'static str, color: [f32; 3], rough: f32, pat: Pat) -> Spec {
    Spec { name, color, metal: 0.0, rough, pat, fx: Fx::None }
}

impl Spec {
    pub const fn metal(mut self, m: f32) -> Self {
        self.metal = m;
        self
    }

    pub const fn fx(mut self, fx: Fx) -> Self {
        self.fx = fx;
        self
    }

    /// The pattern as a Base Color map node and its bump strength (0..100).
    fn node(&self) -> Option<(MapNode, f32)> {
        let c = self.color;
        let seed = self.name.bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32)) % 997;
        Some(match self.pat {
            Pat::Plain => return None,
            Pat::Noise(scale, c2) => (MapNode::Noise { kind: NoiseKind::Fractal, scale, seed, color_a: c, color_b: c2 }, 25.0),
            Pat::Cells(scale, c2) => (MapNode::Noise { kind: NoiseKind::Cellular, scale, seed, color_a: c, color_b: c2 }, 45.0),
            Pat::Bricks(kind, cols, rows, gap, c2) => {
                (MapNode::Bricks { kind, cols, rows, gap, vary: 0.35, seed, color_a: c, color_b: c2 }, if gap > 0.0 { 60.0 } else { 20.0 })
            }
            Pat::Stripes(count, width, dash, soft, c2) => (
                MapNode::Stripes { count, width, dash, soft, along_u: false, color_a: c2, color_b: c },
                if soft { 80.0 } else { 8.0 },
            ),
            Pat::Cracks(scale, width, c2) => (MapNode::Cracks { scale, width, vary: 0.25, seed, color_a: c, color_b: c2 }, 60.0),
            Pat::Grunge(kind, scale, amount, c2) => (MapNode::Grunge { kind, scale, amount, seed, color_a: c, color_b: c2 }, 20.0),
            Pat::Wood(rings, dark) => (MapNode::Wood { scale: 1.0, rings, grain: 0.7, light: c, dark }, 12.0),
        })
    }

    pub fn material(&self) -> Material {
        let mut m = Material::named(self.name);
        (m.base_color, m.metallic, m.roughness) = (self.color, self.metal, self.rough);
        if let Some((node, bump)) = self.node() {
            if let MapNode::Grunge { kind, scale, amount, seed, .. } = node {
                // Wear is rougher, and rust or dirt on metal is not metal.
                let g = |a: f32, b: f32| MapNode::Grunge { kind, scale, amount, seed, color_a: [a; 3], color_b: [b; 3] };
                m.map_mut(MapChannel::Roughness).node = Some(g(self.rough, (self.rough + 0.4).min(1.0)));
                if self.metal > 0.0 {
                    m.map_mut(MapChannel::Metallic).node = Some(g(self.metal, 0.0));
                }
            }
            m.map_mut(MapChannel::BaseColor).node = Some(node.clone());
            let b = m.map_mut(MapChannel::Bump);
            (b.node, b.amount) = (Some(node), bump);
        }
        match self.fx {
            Fx::None => {}
            Fx::Glass(ior, t) => {
                m.model = ShadingModel::Crystal;
                (m.transmission, m.ior, m.thickness) = (t, ior, 0.3);
                m.attenuation_color = self.color;
            }
            Fx::Glow(k) => {
                m.emissive = self.color;
                m.emissive_strength = k;
            }
            Fx::Toon(steps, ink) => {
                m.model = ShadingModel::Toon;
                m.toon = ToonParams { steps, ..ToonParams::default() };
                m.outline = OutlineParams { enabled: ink, crease_lines: ink, ..OutlineParams::default() };
            }
            Fx::Unlit => m.model = ShadingModel::Unlit,
        }
        m
    }
}

// Short pattern constructors for the data tables.
pub const fn noise(scale: f32, c2: [f32; 3]) -> Pat {
    Pat::Noise(scale, c2)
}
pub const fn cells(scale: f32, c2: [f32; 3]) -> Pat {
    Pat::Cells(scale, c2)
}
pub const fn bricks(cols: f32, rows: f32, gap: f32, c2: [f32; 3]) -> Pat {
    Pat::Bricks(BrickKind::Running, cols, rows, gap, c2)
}
pub const fn tiles(cols: f32, rows: f32, gap: f32, c2: [f32; 3]) -> Pat {
    Pat::Bricks(BrickKind::Stack, cols, rows, gap, c2)
}
pub const fn planks(cols: f32, rows: f32, gap: f32, c2: [f32; 3]) -> Pat {
    Pat::Bricks(BrickKind::Planks, cols, rows, gap, c2)
}
pub const fn cobbles(cols: f32, rows: f32, gap: f32, c2: [f32; 3]) -> Pat {
    Pat::Bricks(BrickKind::Cobbles, cols, rows, gap, c2)
}
pub const fn lines(count: f32, width: f32, dash: f32, c2: [f32; 3]) -> Pat {
    Pat::Stripes(count, width, dash, false, c2)
}
pub const fn corrugated(count: f32, c2: [f32; 3]) -> Pat {
    Pat::Stripes(count, 0.5, 0.0, true, c2)
}
pub const fn cracks(scale: f32, width: f32, c2: [f32; 3]) -> Pat {
    Pat::Cracks(scale, width, c2)
}
pub const fn dirt(scale: f32, amount: f32, c2: [f32; 3]) -> Pat {
    Pat::Grunge(GrungeKind::Dirt, scale, amount, c2)
}
pub const fn rust(scale: f32, amount: f32, c2: [f32; 3]) -> Pat {
    Pat::Grunge(GrungeKind::Rust, scale, amount, c2)
}
pub const fn scratches(scale: f32, amount: f32, c2: [f32; 3]) -> Pat {
    Pat::Grunge(GrungeKind::Scratches, scale, amount, c2)
}
pub const fn stains(scale: f32, amount: f32, c2: [f32; 3]) -> Pat {
    Pat::Grunge(GrungeKind::Stains, scale, amount, c2)
}
pub const fn grain(rings: f32, dark: [f32; 3]) -> Pat {
    Pat::Wood(rings, dark)
}
pub const PLAIN: Pat = Pat::Plain;
