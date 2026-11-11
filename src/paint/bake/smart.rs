//! 1-click Smart Mask layers: a fill layer whose mask is generated from
//! the baked mesh maps, broken up with 3D noise so it never looks stamped.

use super::MeshMaps;
use crate::paint::layers::Flat;
use crate::paint::texel::Tiles;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SmartKind {
    /// Convex edges: worn paint, chipped metal.
    EdgeWear,
    /// Creases and occluded corners: dirt, rust.
    CavityDirt,
    /// Surfaces facing up: dust, snow, moss.
    TopDown,
    /// The lower part of the object: soot, mud splash.
    BottomUp,
}

impl SmartKind {
    pub const ALL: [SmartKind; 4] = [SmartKind::EdgeWear, SmartKind::CavityDirt, SmartKind::TopDown, SmartKind::BottomUp];

    pub fn label(self) -> &'static str {
        match self {
            SmartKind::EdgeWear => "Smart Edge Wear / Metal Chipping",
            SmartKind::CavityDirt => "Smart Cavity Dirt / Rust",
            SmartKind::TopDown => "Smart Top-Down Dust / Snow / Moss",
            SmartKind::BottomUp => "Smart Bottom Soot / Mud Splash",
        }
    }

    /// Ready-made looks of this mask: (name, what the layer is filled with).
    pub fn looks(self) -> &'static [(&'static str, Flat)] {
        match self {
            SmartKind::EdgeWear => &EDGE,
            SmartKind::CavityDirt => &CAVITY,
            SmartKind::TopDown => &TOP,
            SmartKind::BottomUp => &BOTTOM,
        }
    }
}

const fn look(color: [f32; 3], rough: f32, metal: f32, height: f32) -> Flat {
    Flat { color, rough, metal, height, emission: 0.0, channels: [true, true, true, true, false] }
}

static EDGE: [(&str, Flat); 2] = [("Edge Wear", look([0.62, 0.6, 0.57], 0.55, 0.0, 0.46)), ("Metal Chipping", look([0.78, 0.78, 0.8], 0.3, 1.0, 0.42))];
static CAVITY: [(&str, Flat); 2] = [("Cavity Dirt", look([0.16, 0.13, 0.1], 0.92, 0.0, 0.52)), ("Rust", look([0.45, 0.2, 0.08], 0.88, 0.0, 0.55))];
static TOP: [(&str, Flat); 3] =
    [("Dust", look([0.62, 0.57, 0.5], 0.95, 0.0, 0.52)), ("Snow", look([0.93, 0.95, 0.98], 0.6, 0.0, 0.62)), ("Moss", look([0.2, 0.33, 0.1], 0.9, 0.0, 0.58))];
static BOTTOM: [(&str, Flat); 2] = [("Soot", look([0.05, 0.05, 0.05], 0.95, 0.0, 0.5)), ("Mud Splash", look([0.27, 0.19, 0.12], 0.8, 0.0, 0.56))];

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SmartMask {
    pub kind: SmartKind,
    /// 0..1: how far the effect spreads.
    pub amount: f32,
    /// 0..1: soft transition to hard edge.
    pub contrast: f32,
    /// 0..1: how much the 3D noise breaks the mask up.
    pub breakup: f32,
    /// Noise features per object size.
    pub scale: f32,
    pub seed: u32,
    /// Index into [`SmartKind::looks`].
    pub look: usize,
}

impl SmartMask {
    pub fn new(kind: SmartKind) -> Self {
        Self { kind, amount: 0.5, contrast: 0.6, breakup: 0.6, scale: 6.0, seed: 1, look: 0 }
    }

    pub fn look_name(&self) -> &'static str {
        let looks = self.kind.looks();
        looks[self.look.min(looks.len() - 1)].0
    }

    pub fn fill(&self) -> Flat {
        let looks = self.kind.looks();
        looks[self.look.min(looks.len() - 1)].1.clone()
    }

    /// Mask value 0..1 for one texel of the maps.
    fn value(&self, maps: &MeshMaps, i: usize, noise: f32) -> f32 {
        let (curv, ao, n, h) = (maps.curvature[i], maps.ao[i], maps.normal[i], maps.height[i]);
        // How strongly the shape asks for the effect here.
        let field = match self.kind {
            SmartKind::EdgeWear => ((curv - 0.5) * 2.0).max(0.0),
            SmartKind::CavityDirt => ((0.5 - curv) * 2.0).max(0.0).max((1.0 - ao) * 1.4),
            SmartKind::TopDown => (n.z * 0.5 + 0.5).powi(2) * (0.6 + 0.4 * ao),
            SmartKind::BottomUp => (1.0 - h).powi(2) * (0.75 - 0.25 * n.z),
        };
        // Noise pushes the threshold around; Amount lowers it.
        let v = field + (noise - 0.5) * self.breakup.clamp(0.0, 1.0);
        let level = 1.0 - self.amount.clamp(0.0, 1.0);
        let soft = (1.0 - self.contrast.clamp(0.0, 1.0)) * 0.45 + 0.02;
        let t = ((v - level + soft) / (2.0 * soft)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Builds the mask at the maps' resolution. Texels outside UV islands
    /// take the nearest island value, so seams stay closed.
    pub fn build(&self, maps: &MeshMaps) -> Tiles<u8> {
        let n = maps.size as usize;
        let fbm = Fbm::<Perlin>::new(self.seed).set_octaves(4);
        let (lo, hi) = maps.pos.iter().zip(&maps.valid).filter(|(_, v)| **v).fold(
            (bevy::math::Vec3::splat(f32::MAX), bevy::math::Vec3::splat(f32::MIN)),
            |(lo, hi), (p, _)| (lo.min(*p), hi.max(*p)),
        );
        let k = self.scale.max(0.01) / (hi - lo).max_element().max(1e-6);
        let values = crate::materials::proc_engine::grid::par_map(n * n, |i| {
            if !maps.valid[i] {
                return 0.0;
            }
            let q = (maps.pos[i] - lo) * k;
            let noise = (fbm.get([q.x as f64, q.y as f64, q.z as f64]) as f32 * 0.5 + 0.5).clamp(0.0, 1.0);
            self.value(maps, i, noise)
        });
        // Two rounds of dilation into the padding.
        let mut filled = maps.valid.clone();
        let mut vals = values;
        for _ in 0..2 {
            let (prev_v, prev_f) = (vals.clone(), filled.clone());
            for y in 0..n {
                for x in 0..n {
                    if prev_f[y * n + x] {
                        continue;
                    }
                    let near = [(0i64, -1i64), (0, 1), (-1, 0), (1, 0)]
                        .iter()
                        .map(|(dx, dy)| ((y as i64 + dy).rem_euclid(n as i64) * n as i64 + (x as i64 + dx).rem_euclid(n as i64)) as usize)
                        .filter(|&j| prev_f[j])
                        .map(|j| prev_v[j])
                        .fold(None, |m: Option<f32>, v| Some(m.map_or(v, |m| m.max(v))));
                    if let Some(v) = near {
                        vals[y * n + x] = v;
                        filled[y * n + x] = true;
                    }
                }
            }
        }
        let mut t = Tiles::new(maps.size);
        t.fill_with(|x, y| (vals[(y as usize) * n + x as usize] * 255.0 + 0.5) as u8);
        t
    }
}
