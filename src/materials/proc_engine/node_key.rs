//! Content keys for map nodes, and nodes baked to tiling grids through the
//! cache. The image filters (Slope Blur, Normal From Height) need
//! neighbouring pixels, so they are cooked from their inputs' grids; the
//! same sub-graph wired into two filters is baked once.

use super::cache::{self, Key};
use super::filters;
use super::grid::Grid;
use crate::materials::bake::uv_point;
use crate::materials::maps::MapNode;
use std::sync::Arc;

/// Side of a baked node grid, in pixels.
pub const GRID: u32 = 256;

fn all(k: &mut Key, a: u64, seed: u32, fs: &[f32], colors: &[&[f32; 3]]) {
    k.u(a).u(seed as u64).f(fs);
    colors.iter().for_each(|c| _ = k.f(*c));
}

fn feed(k: &mut Key, n: &MapNode) {
    k.u(n.kind_index() as u64 + 1);
    match n {
        MapNode::Bitmap { path, tex } => {
            path.bytes().for_each(|b| _ = k.u(b as u64));
            // Painting copies the texture, so its address stands for its content.
            k.u(tex.as_ref().map_or(0, |t| Arc::as_ptr(&t.tex) as usize as u64));
        }
        MapNode::Noise { kind, scale, seed, color_a, color_b } => all(k, *kind as u64, *seed, &[*scale], &[color_a, color_b]),
        MapNode::Checker { tiles, color_a, color_b } => all(k, 0, 0, &[*tiles], &[color_a, color_b]),
        MapNode::Gradient { color_a, color_b, color_c, mid, radial } => all(k, *radial as u64, 0, &[*mid], &[color_a, color_b, color_c]),
        MapNode::Wood { scale, rings, grain, light, dark } => all(k, 0, 0, &[*scale, *rings, *grain], &[light, dark]),
        MapNode::ColorCorrect { input, brightness, contrast, saturation, hue, tint } => {
            all(k, 0, 0, &[*brightness, *contrast, *saturation, *hue], &[tint]);
            feed(k, input);
        }
        MapNode::Bricks { kind, cols, rows, gap, vary, seed, color_a, color_b } => all(k, *kind as u64, *seed, &[*cols, *rows, *gap, *vary], &[color_a, color_b]),
        MapNode::Stripes { count, width, dash, soft, along_u, color_a, color_b } => {
            all(k, *soft as u64 * 2 + *along_u as u64, 0, &[*count, *width, *dash], &[color_a, color_b]);
        }
        MapNode::Cracks { scale, width, vary, seed, color_a, color_b } => all(k, 0, *seed, &[*scale, *width, *vary], &[color_a, color_b]),
        MapNode::Grunge { kind, scale, amount, seed, color_a, color_b } => all(k, *kind as u64, *seed, &[*scale, *amount], &[color_a, color_b]),
        MapNode::Tile { kind, scale, detail, width, warp, seed, color_a, color_b } => {
            all(k, *kind as u64, *seed, &[*scale, *detail, *width, *warp], &[color_a, color_b]);
        }
        MapNode::Levels { input, in_low, in_mid, in_high, out_low, out_high } => {
            k.f(&[*in_low, *in_mid, *in_high, *out_low, *out_high]);
            feed(k, input);
        }
        MapNode::HeightBlend { top, bottom, height, level, contrast } => {
            k.f(&[*level, *contrast]);
            [top, bottom, height].into_iter().for_each(|n| feed(k, n));
        }
        MapNode::SlopeBlur { input, slope, intensity, samples } => {
            k.u(*samples as u64).f(&[*intensity]);
            feed(k, input);
            feed(k, slope);
        }
        MapNode::NormalFromHeight { input, intensity } => {
            k.f(&[*intensity]);
            feed(k, input);
        }
    }
}

/// The content key of a node: equal for equal nodes, different (with
/// overwhelming likelihood) after any edit to it or to its inputs.
pub fn content_key(n: &MapNode) -> u64 {
    let mut k = Key::default();
    feed(&mut k, n);
    k.0
}

/// A node baked over the 0..1 UV square (3D maps use the same UV-derived
/// point as the viewport bake). Filters are cooked from their inputs' grids.
pub fn grid(node: &MapNode) -> Arc<Grid> {
    cache::cooked(content_key(node), || match node {
        MapNode::SlopeBlur { input, slope, intensity, samples } => filters::slope_blur(&grid(input), &grid(slope), *intensity, *samples),
        MapNode::NormalFromHeight { input, intensity } => filters::normal_from_height(&grid(input), *intensity),
        _ => Grid::bake(GRID, |uv| node.sample(uv, uv_point(uv))),
    })
}
