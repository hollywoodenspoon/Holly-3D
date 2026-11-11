//! A material's "Procedural Pattern & Weathering" recipe and its cook: the
//! pattern's height and colour, the weathering layers, then normal, cavity
//! AO and roughness derived from the finished height map.

use super::cache::{self, Cooked, Key};
use super::filters::{cavity_ao, normal_from_height, unpack_normal};
use super::generators::{self, Px};
use super::grid::{Grid, par_map};
use super::styles::{Extra, Pattern, Style};
use super::{fibers, weather};
use bevy::math::{Vec2, Vec3};
use std::sync::Arc;

/// Side of a cooked surface, in pixels.
pub const COOK_SIZE: u32 = if cfg!(test) { 64 } else { 512 };

/// The recipe: every field is a spinner, slider or checkbox of the rollout.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ProcSurface {
    pub pattern: Pattern,
    /// Style within the pattern (see [`Pattern::styles`]).
    pub style: u8,
    pub tiling: f32,
    pub seed: u32,
    /// Gap / mortar width.
    pub gap: f32,
    /// Gap / bevel depth.
    pub bevel: f32,
    pub color_variation: f32,
    /// Surface relief: the strength of the derived normal map and AO.
    pub relief: f32,
    /// Surface and gap / secondary colours (sRGB).
    pub color_a: [f32; 3],
    pub color_b: [f32; 3],
    /// Weathering sliders, 0..1.
    pub edge_wear: f32,
    pub cavity_dirt: f32,
    pub moss: f32,
    pub scorch: f32,
    pub wetness: f32,
    /// Auto-derive normal, roughness and cavity AO from the height map.
    pub auto_pbr: bool,
}

impl Default for ProcSurface {
    fn default() -> Self {
        Self {
            pattern: Pattern::None,
            style: 0,
            tiling: 1.0,
            seed: 0,
            gap: 0.1,
            bevel: 0.15,
            color_variation: 0.3,
            relief: 1.0,
            color_a: [0.6, 0.6, 0.6],
            color_b: [0.3, 0.3, 0.3],
            edge_wear: 0.0,
            cavity_dirt: 0.0,
            moss: 0.0,
            scorch: 0.0,
            wetness: 0.0,
            auto_pbr: true,
        }
    }
}

impl ProcSurface {
    /// A recipe for `pattern` / `style` with that style's gap, bevel and colours.
    pub fn styled(pattern: Pattern, style: u8) -> ProcSurface {
        let mut p = ProcSurface { pattern, style, ..Default::default() };
        if let Some(s) = p.style_def() {
            if s.user_gap {
                (p.gap, p.bevel) = (s.knobs[2], s.knobs[3]);
            }
            (p.color_a, p.color_b) = s.colors;
        }
        p
    }

    /// Switches to another pattern / style with its gap, bevel and colours,
    /// keeping the tiling, seed, relief and weathering.
    pub fn restyle(&mut self, pattern: Pattern, style: u8) {
        let s = ProcSurface::styled(pattern, style);
        (self.pattern, self.style, self.gap, self.bevel, self.color_a, self.color_b) = (s.pattern, s.style, s.gap, s.bevel, s.color_a, s.color_b);
    }

    pub fn active(&self) -> bool {
        self.pattern != Pattern::None
    }

    pub fn style_def(&self) -> Option<&'static Style> {
        self.pattern.style(self.style)
    }

    /// The generator knobs this recipe runs with.
    pub fn knobs(&self, s: &Style) -> [f32; 4] {
        let t = self.tiling.clamp(0.25, 64.0);
        let mut k = s.knobs;
        k[0] *= t;
        if s.detail_scales {
            k[1] *= t;
        }
        if s.user_gap {
            (k[2], k[3]) = (self.gap, self.bevel);
        }
        k
    }

    /// Content key: the recipe plus the material's base roughness and metalness.
    pub fn key(&self, rough: f32, metal: f32, size: u32) -> u64 {
        let mut k = Key::default();
        k.u(0x50c5).u(self.pattern as u64).u(self.style as u64).u(self.seed as u64).u(self.auto_pbr as u64).u(size as u64);
        k.f(&[self.tiling, self.gap, self.bevel, self.color_variation, self.relief, rough, metal]);
        k.f(&self.color_a).f(&self.color_b);
        k.f(&[self.edge_wear, self.cavity_dirt, self.moss, self.scorch, self.wetness]).0
    }
}

/// The working layers of a cook, one value per pixel.
pub struct Layers {
    pub size: u32,
    pub h: Vec<f32>,
    pub color: Vec<Vec3>,
    pub rough: Vec<f32>,
    pub metal: Vec<f32>,
}

/// A cooked surface: everything shading needs at a UV.
pub struct ProcMaps {
    /// sRGB colour, height in alpha.
    pub base: Grid,
    /// R = cavity AO, G = roughness, B = metalness.
    pub orm: Grid,
    /// Tangent-space normal map.
    pub normal: Grid,
}

impl Cooked for ProcMaps {
    fn bytes(&self) -> usize {
        self.base.bytes() + self.orm.bytes() + self.normal.bytes()
    }
}

/// One shading sample of a cooked surface.
#[derive(Clone, Copy, Debug)]
pub struct ProcSample {
    pub base: Vec3,
    pub height: f32,
    pub ao: f32,
    pub rough: f32,
    pub metal: f32,
    pub normal: Vec3,
}

impl ProcMaps {
    pub fn at(&self, uv: Vec2) -> ProcSample {
        let (b, o) = (self.base.sample(uv), self.orm.sample(uv));
        ProcSample { base: b.truncate(), height: b.w, ao: o.x, rough: o.y, metal: o.z, normal: unpack_normal(self.normal.sample(uv)) }
    }
}

fn varied(c: [f32; 3], amount: f32, id: f32) -> Vec3 {
    (Vec3::from_array(c) * (1.0 + (id - 0.5) * amount * 0.9)).clamp(Vec3::ZERO, Vec3::ONE)
}

/// Cooks a recipe at `size` × `size`. `rough` and `metal` are the
/// material's own values; the style and weathering vary them.
pub fn cook(p: &ProcSurface, rough: f32, metal: f32, size: u32) -> ProcMaps {
    let n = size.max(4) as usize;
    let Some(style) = p.style_def() else {
        let flat = |c: [f32; 4]| Grid { size: n as u32, data: vec![c; n * n] };
        return ProcMaps { base: flat([p.color_a[0], p.color_a[1], p.color_a[2], 0.5]), orm: flat([1.0, rough, metal, 1.0]), normal: flat([0.5, 0.5, 1.0, 1.0]) };
    };
    let knobs = p.knobs(style);
    let gaps = style.kind.has_gaps();
    let uv = |i: usize| (((i % n) as f32 + 0.5) / n as f32, ((i / n) as f32 + 0.5) / n as f32);
    // Pattern: height, colour and base roughness / metalness per pixel.
    let px: Vec<(Px, Vec3, f32, f32)> = par_map(n * n, |i| {
        let (u, v) = uv(i);
        let mut px = generators::sample(style.kind, u, v, knobs, p.seed);
        let face = if gaps { super::lattice::smoothstep(0.0, 0.06, px.h) } else { px.h };
        let mut surface = varied(p.color_a, p.color_variation, px.id);
        let (mut r, mut m) = (rough + (if gaps { 0.9 - rough } else { 0.0 }) * (1.0 - face) * 0.6, if gaps { metal * face } else { metal });
        match style.extra {
            Extra::None => {}
            Extra::Grain => surface *= 0.72 + 0.28 * fibers::wood_grain(v + px.id * 7.3, u, 6.0, 8.0, 2.0, 0.6, p.seed ^ 0x77),
            Extra::Line(dashes, paint) => {
                let on = (u - 0.5).abs() < 0.035 && (dashes <= 0.0 || (v * dashes).fract() < 0.5);
                if on {
                    // Paint fills the small pits, so the line is smoother and a little proud.
                    (px.h, r, m) = (px.h * 0.4 + 0.62, 0.6, 0.0);
                    surface = Vec3::from_array(paint) * (0.85 + 0.15 * px.h);
                    return (px, surface, r, m);
                }
            }
        }
        (px, Vec3::from_array(p.color_b).lerp(surface, face), r, m)
    });
    let mut l = Layers {
        size: n as u32,
        h: px.iter().map(|x| x.0.h).collect(),
        color: px.iter().map(|x| x.1).collect(),
        rough: px.iter().map(|x| x.2).collect(),
        metal: px.iter().map(|x| x.3).collect(),
    };
    weather::apply(p, metal, &mut l);
    // Derived channels.
    let height = Grid::gray_from(l.size, &l.h);
    let (normal, ao) = if p.auto_pbr && p.relief > 0.0 {
        (normal_from_height(&height, p.relief * 0.35), cavity_ao(l.size, &l.h, 0.08, (p.relief * 0.1).min(0.4)))
    } else {
        (Grid { size: l.size, data: vec![[0.5, 0.5, 1.0, 1.0]; n * n] }, vec![1.0; n * n])
    };
    let data = |f: &dyn Fn(usize) -> [f32; 4]| (0..n * n).map(f).collect::<Vec<_>>();
    let base = data(&|i| [l.color[i].x, l.color[i].y, l.color[i].z, l.h[i]]);
    let orm = data(&|i| {
        // Recesses and occluded joints read rougher than raised faces.
        // Glossy areas (puddles, polished metal) keep their shine.
        let vary = if p.auto_pbr { ((0.5 - l.h[i]) * 0.08 + (1.0 - ao[i]) * 0.25) * (l.rough[i] * 2.0).min(1.0) } else { 0.0 };
        [ao[i], (l.rough[i] + vary).clamp(0.02, 1.0), l.metal[i].clamp(0.0, 1.0), 1.0]
    });
    ProcMaps { base: Grid { size: l.size, data: base }, orm: Grid { size: l.size, data: orm }, normal }
}

/// The cooked maps of a recipe, from the cache (cooking now on a miss).
/// `None` when the recipe has no pattern.
pub fn maps(p: &ProcSurface, rough: f32, metal: f32) -> Option<Arc<ProcMaps>> {
    p.active().then(|| cache::cooked(p.key(rough, metal, COOK_SIZE), || cook(p, rough, metal, COOK_SIZE)))
}
