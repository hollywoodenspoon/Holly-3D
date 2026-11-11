//! Maps rollout channels and the procedural / bitmap map nodes that feed
//! them (also the nodes shown in the Slate view).

use super::patterns::{self as pat, BrickKind, GrungeKind};
use super::proc_engine::{TileKind, adjust, generators, node_key};
use super::texture::TexHandle;
use bevy::math::{Vec2, Vec3, Vec4};
use noise::{Fbm, NoiseFn, Perlin, Worley};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MapChannel {
    BaseColor = 0,
    Metallic,
    Roughness,
    Bump,
    Opacity,
    Emission,
}

impl MapChannel {
    pub const ALL: [MapChannel; 6] =
        [MapChannel::BaseColor, MapChannel::Metallic, MapChannel::Roughness, MapChannel::Bump, MapChannel::Opacity, MapChannel::Emission];
    pub fn label(self) -> &'static str {
        match self {
            MapChannel::BaseColor => "Base Color / Diffuse",
            MapChannel::Metallic => "Metallic",
            MapChannel::Roughness => "Roughness",
            MapChannel::Bump => "Bump / Normal",
            MapChannel::Opacity => "Opacity",
            MapChannel::Emission => "Emission",
        }
    }
    /// File-name keywords used by Import PBR Texture Folder.
    pub fn keywords(self) -> &'static [&'static str] {
        match self {
            MapChannel::BaseColor => &["basecolor", "base_color", "albedo", "diffuse", "color", "col"],
            MapChannel::Metallic => &["metallic", "metalness", "metal"],
            MapChannel::Roughness => &["roughness", "rough"],
            MapChannel::Bump => &["normal", "nrm", "bump", "height", "disp"],
            MapChannel::Opacity => &["opacity", "alpha", "mask"],
            MapChannel::Emission => &["emission", "emissive", "emit"],
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MapSlot {
    pub enabled: bool,
    /// 0..=100 like the 3ds Max Maps rollout amount spinners.
    pub amount: f32,
    pub node: Option<MapNode>,
}

impl Default for MapSlot {
    fn default() -> Self {
        Self { enabled: true, amount: 100.0, node: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum NoiseKind {
    #[default]
    Perlin,
    Fractal,
    Cellular,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MapNode {
    Bitmap { path: String, tex: Option<TexHandle> },
    Noise { kind: NoiseKind, scale: f32, seed: u32, color_a: [f32; 3], color_b: [f32; 3] },
    Checker { tiles: f32, color_a: [f32; 3], color_b: [f32; 3] },
    Gradient { color_a: [f32; 3], color_b: [f32; 3], color_c: [f32; 3], mid: f32, radial: bool },
    Wood { scale: f32, rings: f32, grain: f32, light: [f32; 3], dark: [f32; 3] },
    ColorCorrect { input: Box<MapNode>, brightness: f32, contrast: f32, saturation: f32, hue: f32, tint: [f32; 3] },
    /// Bricks, tiles, planks and cobbles (UV space). A = bricks, B = mortar.
    Bricks { kind: BrickKind, cols: f32, rows: f32, gap: f32, vary: f32, seed: u32, color_a: [f32; 3], color_b: [f32; 3] },
    /// Road lines and corrugated metal (UV space). A = line, B = ground.
    Stripes { count: f32, width: f32, dash: f32, soft: bool, along_u: bool, color_a: [f32; 3], color_b: [f32; 3] },
    /// Voronoi cracks / flagstones (3D). A = surface, B = crack.
    Cracks { scale: f32, width: f32, vary: f32, seed: u32, color_a: [f32; 3], color_b: [f32; 3] },
    /// Dirt, rust, scratches and stains (3D). A = clean, B = dirt.
    Grunge { kind: GrungeKind, scale: f32, amount: f32, seed: u32, color_a: [f32; 3], color_b: [f32; 3] },
    /// Tileable generator on the whole-number lattice (UV space); the four
    /// knobs mean what `TileKind::knobs` says. A = low, B = high.
    Tile { kind: TileKind, scale: f32, detail: f32, width: f32, warp: f32, seed: u32, color_a: [f32; 3], color_b: [f32; 3] },
    /// Remaps the input range, midtone and output range.
    Levels { input: Box<MapNode>, in_low: f32, in_mid: f32, in_high: f32, out_low: f32, out_high: f32 },
    /// `top` over `bottom` wherever a layer at `level` stands above the
    /// `height` map, so it fills low places first.
    HeightBlend { top: Box<MapNode>, bottom: Box<MapNode>, height: Box<MapNode>, level: f32, contrast: f32 },
    /// Smears `input` downhill along the slopes of `slope`.
    SlopeBlur { input: Box<MapNode>, slope: Box<MapNode>, intensity: f32, samples: u32 },
    /// Tangent-space normal map of the `input` height map.
    NormalFromHeight { input: Box<MapNode>, intensity: f32 },
}

impl MapNode {
    pub const KINDS: [&'static str; 15] = [
        "Bitmap",
        "Noise",
        "Checker",
        "Gradient Ramp",
        "Wood",
        "Color Correction",
        "Bricks / Tiles",
        "Stripes",
        "Cracks / Voronoi",
        "Grunge / Wear",
        "Tileable Pattern",
        "Levels",
        "Height Blend",
        "Slope Blur",
        "Normal From Height",
    ];

    /// Index of this node's kind in [`Self::KINDS`].
    pub fn kind_index(&self) -> usize {
        match self {
            MapNode::Bitmap { .. } => 0,
            MapNode::Noise { .. } => 1,
            MapNode::Checker { .. } => 2,
            MapNode::Gradient { .. } => 3,
            MapNode::Wood { .. } => 4,
            MapNode::ColorCorrect { .. } => 5,
            MapNode::Bricks { .. } => 6,
            MapNode::Stripes { .. } => 7,
            MapNode::Cracks { .. } => 8,
            MapNode::Grunge { .. } => 9,
            MapNode::Tile { .. } => 10,
            MapNode::Levels { .. } => 11,
            MapNode::HeightBlend { .. } => 12,
            MapNode::SlopeBlur { .. } => 13,
            MapNode::NormalFromHeight { .. } => 14,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        Self::KINDS[self.kind_index()]
    }

    /// A Tileable Pattern node of `kind` with its default knobs.
    pub fn tile(kind: TileKind, color_a: [f32; 3], color_b: [f32; 3]) -> MapNode {
        let k = kind.defaults();
        MapNode::Tile { kind, scale: k[0], detail: k[1], width: k[2], warp: k[3], seed: 0, color_a, color_b }
    }

    /// The nodes wired into this one, with their socket names.
    pub fn inputs_mut(&mut self) -> Vec<(&'static str, &mut MapNode)> {
        match self {
            MapNode::ColorCorrect { input, .. } | MapNode::Levels { input, .. } | MapNode::NormalFromHeight { input, .. } => vec![("Input", &mut **input)],
            MapNode::HeightBlend { top, bottom, height, .. } => vec![("Top", &mut **top), ("Bottom", &mut **bottom), ("Height", &mut **height)],
            MapNode::SlopeBlur { input, slope, .. } => vec![("Input", &mut **input), ("Slope", &mut **slope)],
            _ => vec![],
        }
    }

    /// How many input sockets this kind of node has.
    pub fn input_count(&self) -> usize {
        match self {
            MapNode::ColorCorrect { .. } | MapNode::Levels { .. } | MapNode::NormalFromHeight { .. } => 1,
            MapNode::SlopeBlur { .. } => 2,
            MapNode::HeightBlend { .. } => 3,
            _ => 0,
        }
    }

    /// A new node of `KINDS[i]` (Color Correction and the filters wrap `input`).
    pub fn new_kind(i: usize, input: Option<MapNode>) -> MapNode {
        let (w, k) = ([1.0, 1.0, 1.0], [0.0, 0.0, 0.0]);
        match i {
            0 => MapNode::Bitmap { path: String::new(), tex: None },
            1 => MapNode::Noise { kind: NoiseKind::Fractal, scale: 4.0, seed: 0, color_a: k, color_b: w },
            2 => MapNode::Checker { tiles: 8.0, color_a: [0.15, 0.15, 0.15], color_b: [0.85, 0.85, 0.85] },
            3 => MapNode::Gradient { color_a: k, color_b: [0.5, 0.5, 0.5], color_c: w, mid: 0.5, radial: false },
            4 => MapNode::Wood { scale: 1.0, rings: 12.0, grain: 0.6, light: [0.78, 0.56, 0.34], dark: [0.45, 0.26, 0.12] },
            6 => MapNode::Bricks {
                kind: BrickKind::Running,
                cols: 4.0,
                rows: 8.0,
                gap: 0.1,
                vary: 0.3,
                seed: 0,
                color_a: [0.62, 0.27, 0.18],
                color_b: [0.78, 0.76, 0.72],
            },
            7 => MapNode::Stripes { count: 1.0, width: 0.06, dash: 0.0, soft: false, along_u: false, color_a: w, color_b: [0.12; 3] },
            8 => MapNode::Cracks { scale: 3.0, width: 0.06, vary: 0.2, seed: 0, color_a: [0.6, 0.48, 0.34], color_b: [0.12, 0.09, 0.07] },
            9 => MapNode::Grunge { kind: GrungeKind::Dirt, scale: 2.0, amount: 0.4, seed: 0, color_a: w, color_b: [0.3, 0.25, 0.2] },
            10 => MapNode::tile(TileKind::Bricks, [0.0; 3], w),
            11..=14 => {
                let bricks = || MapNode::tile(TileKind::Bricks, [0.0; 3], w);
                let input = Box::new(input.unwrap_or_else(bricks));
                match i {
                    11 => MapNode::Levels { input, in_low: 0.0, in_mid: 0.5, in_high: 1.0, out_low: 0.0, out_high: 1.0 },
                    12 => {
                        let moss = MapNode::tile(TileKind::Fbm, [0.14, 0.24, 0.07], [0.32, 0.42, 0.13]);
                        MapNode::HeightBlend { top: Box::new(moss), height: input.clone(), bottom: input, level: 0.35, contrast: 0.8 }
                    }
                    13 => MapNode::SlopeBlur { slope: input.clone(), input, intensity: 8.0, samples: 12 },
                    _ => MapNode::NormalFromHeight { input, intensity: 1.0 },
                }
            }
            _ => MapNode::ColorCorrect {
                input: Box::new(input.unwrap_or_else(|| MapNode::new_kind(1, None))),
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue: 0.0,
                tint: w,
            },
        }
    }

    /// Evaluates the map at `uv` / object-space `p` (3D maps). Returns sRGB + alpha.
    pub fn sample(&self, uv: Vec2, p: Vec3) -> Vec4 {
        let rgb = |c: [f32; 3]| Vec3::from_array(c);
        let out = match self {
            MapNode::Bitmap { tex, .. } => return tex.as_ref().map_or(Vec4::new(1.0, 0.0, 1.0, 1.0), |t| t.tex.sample(uv)),
            MapNode::Noise { kind, scale, seed, color_a, color_b } => {
                let q = (p * *scale + Vec3::splat(*seed as f32 * 17.13)).as_dvec3().to_array();
                let t = match kind {
                    NoiseKind::Perlin => perlin().get(q) * 0.5 + 0.5,
                    NoiseKind::Fractal => fbm().get(q) * 0.5 + 0.5,
                    NoiseKind::Cellular => worley_get(q) * 0.5 + 0.5,
                } as f32;
                rgb(*color_a).lerp(rgb(*color_b), t.clamp(0.0, 1.0))
            }
            MapNode::Checker { tiles, color_a, color_b } => {
                let c = (uv * *tiles).floor();
                if (c.x + c.y).rem_euclid(2.0) < 1.0 { rgb(*color_a) } else { rgb(*color_b) }
            }
            MapNode::Gradient { color_a, color_b, color_c, mid, radial } => {
                let t = if *radial { ((uv - Vec2::splat(0.5)).length() * 2.0).min(1.0) } else { uv.y.clamp(0.0, 1.0) };
                let m = mid.clamp(0.01, 0.99);
                if t < m { rgb(*color_a).lerp(rgb(*color_b), t / m) } else { rgb(*color_b).lerp(rgb(*color_c), (t - m) / (1.0 - m)) }
            }
            MapNode::Wood { scale, rings, grain, light, dark } => {
                let q = p * *scale;
                let n = fbm().get((q * Vec3::new(2.0, 2.0, 0.4)).as_dvec3().to_array()) as f32;
                let r = q.truncate().length() * *rings + n * *grain * 4.0;
                let band = (r.fract() * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                rgb(*light).lerp(rgb(*dark), band.powf(2.5))
            }
            MapNode::ColorCorrect { input, brightness, contrast, saturation, hue, tint } => {
                let s = input.sample(uv, p);
                let c = color_correct(s.truncate(), *brightness, *contrast, *saturation, *hue) * rgb(*tint);
                return c.clamp(Vec3::ZERO, Vec3::ONE).extend(s.w);
            }
            MapNode::Bricks { kind, cols, rows, gap, vary, seed, color_a, color_b } => {
                pat::bricks(*kind, *cols, *rows, *gap, *vary, *seed, rgb(*color_a), rgb(*color_b), uv)
            }
            MapNode::Stripes { count, width, dash, soft, along_u, color_a, color_b } => {
                pat::stripes(*count, *width, *dash, *soft, *along_u, rgb(*color_a), rgb(*color_b), uv)
            }
            MapNode::Cracks { scale, width, vary, seed, color_a, color_b } => {
                pat::cracks(*scale, *width, *vary, *seed, rgb(*color_a), rgb(*color_b), p)
            }
            MapNode::Grunge { kind, scale, amount, seed, color_a, color_b } => {
                pat::grunge(*kind, *scale, *amount, *seed, rgb(*color_a), rgb(*color_b), p)
            }
            MapNode::Tile { kind, scale, detail, width, warp, seed, color_a, color_b } => {
                let h = generators::height(*kind, uv.x, uv.y, [*scale, *detail, *width, *warp], *seed);
                rgb(*color_a).lerp(rgb(*color_b), h)
            }
            MapNode::Levels { input, in_low, in_mid, in_high, out_low, out_high } => {
                let s = input.sample(uv, p);
                let f = |v: f32| adjust::levels(v, *in_low, *in_mid, *in_high, *out_low, *out_high).clamp(0.0, 1.0);
                return Vec4::new(f(s.x), f(s.y), f(s.z), s.w);
            }
            MapNode::HeightBlend { top, bottom, height, level, contrast } => {
                let k = adjust::height_blend(*level, height.sample_scalar(uv, p), *contrast);
                return bottom.sample(uv, p).lerp(top.sample(uv, p), k);
            }
            MapNode::SlopeBlur { .. } | MapNode::NormalFromHeight { .. } => return node_key::grid(self).sample(uv),
        };
        out.extend(1.0)
    }

    /// Scalar value (luminance) for grayscale channels.
    pub fn sample_scalar(&self, uv: Vec2, p: Vec3) -> f32 {
        let c = self.sample(uv, p);
        c.x * 0.2126 + c.y * 0.7152 + c.z * 0.0722
    }
}

fn perlin() -> &'static Perlin {
    static P: OnceLock<Perlin> = OnceLock::new();
    P.get_or_init(|| Perlin::new(7))
}

fn fbm() -> &'static Fbm<Perlin> {
    static F: OnceLock<Fbm<Perlin>> = OnceLock::new();
    F.get_or_init(|| Fbm::new(11))
}

fn worley_get(q: [f64; 3]) -> f64 {
    // `Worley` holds an `Rc` distance function, so it cannot be a shared static.
    thread_local!(static W: Worley = Worley::new(3));
    W.with(|w| w.get(q))
}

/// Brightness / contrast / saturation / hue (degrees) correction.
pub fn color_correct(c: Vec3, brightness: f32, contrast: f32, saturation: f32, hue: f32) -> Vec3 {
    let c = (c - Vec3::splat(0.5)) * contrast + Vec3::splat(0.5 + brightness);
    let l = c.dot(Vec3::new(0.2126, 0.7152, 0.0722));
    let c = Vec3::splat(l).lerp(c, saturation);
    if hue == 0.0 {
        return c;
    }
    // Rotate around the gray axis (Rodrigues).
    let k = Vec3::ONE.normalize();
    let (s, co) = hue.to_radians().sin_cos();
    c * co + k.cross(c) * s + k * k.dot(c) * (1.0 - co)
}
