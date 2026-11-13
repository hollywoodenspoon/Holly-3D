//! Live maps: materials whose maps the GPU evaluates per hit instead of
//! baking them over a UV grid. These are the 3D procedural maps (noise,
//! cracks, grunge, wood) and maps that never repeat under World /
//! Triplanar projection (bricks), which a grid cannot hold. The kernel
//! (`wgsl/live.wgsl`) ports [`MapNode::sample`] and the pattern maps
//! exactly, with the noise crate's own permutation tables uploaded.
//!
//! Record ([`HEAD`] + 5 × [`SLOT`] words): base colour (sRGB), metallic,
//! roughness, emission (linear × strength), emission-map gain, word index
//! of the noise tables; then one slot per channel (base colour, metallic,
//! roughness, bump, emission): kind, sub-kind, seed, amount, 4 parameters,
//! colour A, colour B.

use crate::materials::Material;
use crate::materials::maps::{MapChannel, MapNode, NoiseKind};
use crate::materials::proc_engine::Pattern;
use noise::permutationtable::{NoiseHasher, PermutationTable};

pub const HEAD: usize = 16;
pub const SLOT: usize = 16;
/// The channels a record holds, in slot order.
pub const CHANNELS: [MapChannel; 5] = [MapChannel::BaseColor, MapChannel::Metallic, MapChannel::Roughness, MapChannel::Bump, MapChannel::Emission];
pub const WORDS: usize = HEAD + SLOT * CHANNELS.len();

/// Seeds of the noise tables, in the order the kernel indexes them:
/// Noise Perlin, Noise Fractal's six octaves, Noise Cellular, the pattern maps' Perlin.
pub const SEEDS: [u32; 9] = [7, 11, 12, 13, 14, 15, 16, 3, 29];

/// The permutation tables, 256 words each.
pub fn tables() -> Vec<u32> {
    SEEDS
        .iter()
        .flat_map(|&s| {
            let t = PermutationTable::new(s);
            (0..256isize).map(move |i| t.hash(&[i]) as u32)
        })
        .collect()
}

/// Kind codes the kernel switches on (0 = no map).
fn kind(n: &MapNode) -> Option<u32> {
    Some(match n {
        MapNode::Noise { .. } => 1,
        MapNode::Bricks { .. } => 2,
        MapNode::Cracks { .. } => 3,
        MapNode::Grunge { .. } => 4,
        MapNode::Wood { .. } => 5,
        MapNode::Checker { .. } => 6,
        MapNode::Gradient { .. } => 7,
        MapNode::Stripes { .. } => 8,
        _ => return None,
    })
}

/// The GPU can evaluate every map of `m` per hit.
pub fn supported(m: &Material) -> bool {
    let plain = m.proc.pattern == Pattern::None && m.paint.is_none() && m.slate_nodes.is_empty();
    let opacity = m.active_map(MapChannel::Opacity).is_some();
    let maps = m.maps.iter().filter(|s| s.enabled && s.amount > 0.0).filter_map(|s| s.node.as_ref());
    plain && !opacity && maps.clone().next().is_some() && maps.into_iter().all(|n| kind(n).is_some())
}

/// The material renders with live maps on the GPU: it cannot be baked, but every map can be evaluated.
pub fn live(m: &Material) -> bool {
    super::bake::why_not(m).is_some() && supported(m)
}

/// The bump channel tilts normals.
pub fn tilts(m: &Material) -> bool {
    m.active_map(MapChannel::Bump).is_some()
}

fn slot(n: &MapNode, amount: f32) -> [u32; SLOT] {
    let f = |x: f32| x.to_bits();
    let (sub, seed, s, a, b) = match n {
        MapNode::Noise { kind, scale, seed, color_a, color_b } => {
            let k = match kind {
                NoiseKind::Perlin => 0,
                NoiseKind::Fractal => 1,
                NoiseKind::Cellular => 2,
            };
            (k, *seed, [*scale, 0.0, 0.0, 0.0], color_a, color_b)
        }
        MapNode::Bricks { kind, cols, rows, gap, vary, seed, color_a, color_b } => (*kind as u32, *seed, [*cols, *rows, *gap, *vary], color_a, color_b),
        MapNode::Cracks { scale, width, vary, seed, color_a, color_b } => (0, *seed, [*scale, *width, *vary, 0.0], color_a, color_b),
        MapNode::Grunge { kind, scale, amount, seed, color_a, color_b } => (*kind as u32, *seed, [*scale, *amount, 0.0, 0.0], color_a, color_b),
        MapNode::Wood { scale, rings, grain, light, dark } => (0, 0, [*scale, *rings, *grain, 0.0], light, dark),
        MapNode::Checker { tiles, color_a, color_b } => (0, 0, [*tiles, 0.0, 0.0, 0.0], color_a, color_b),
        MapNode::Gradient { color_a, color_b, color_c, mid, radial } => {
            // Colour C rides in the parameters.
            (u32::from(*radial), 0, [*mid, color_c[0], color_c[1], color_c[2]], color_a, color_b)
        }
        MapNode::Stripes { count, width, dash, soft, along_u, color_a, color_b } => {
            (u32::from(*soft) | u32::from(*along_u) << 1, 0, [*count, *width, *dash, 0.0], color_a, color_b)
        }
        _ => return [0; SLOT],
    };
    let k = kind(n).unwrap_or(0);
    #[rustfmt::skip]
    let w = [k, sub, seed, f(amount), f(s[0]), f(s[1]), f(s[2]), f(s[3]),
        f(a[0]), f(a[1]), f(a[2]), f(b[0]), f(b[1]), f(b[2]), 0, 0];
    w
}

/// `m`'s record; `tables` is the word index of the noise tables in the store.
pub fn record(m: &Material, tables: u32) -> Vec<u32> {
    let f = |x: f32| x.to_bits();
    let e = crate::materials::shading::lin3(m.emissive) * m.emissive_strength;
    let c = m.base_color;
    let mut w = vec![f(c[0]), f(c[1]), f(c[2]), f(m.metallic), f(m.roughness), f(e.x), f(e.y), f(e.z), f(m.emissive_strength.max(1.0)), tables];
    w.resize(HEAD, 0);
    for ch in CHANNELS {
        w.extend(m.active_map(ch).map_or([0; SLOT], |(n, a)| slot(n, a)));
    }
    w
}
