//! Tiled copy-on-write texel storage for paint layers and masks (the
//! design of TextureCraft's `texel` crate, MIT / Apache-2.0).
//!
//! An image is a grid of 64 x 64 tiles behind `Arc`s. Cloning an image
//! (an undo snapshot) copies only the pointers; the first write to a tile
//! copies that one tile. Tiles that were never painted are not allocated.

mod serde_impl;
#[cfg(test)]
mod tests;

use bevy::math::{Vec2, Vec3};
use std::sync::Arc;

/// Tile edge in texels.
pub const TILE: u32 = 64;
const N: usize = (TILE * TILE) as usize;

/// Indices into [`Texel::pbr`] and [`Texel::cov`].
pub const ROUGH: usize = 0;
pub const METAL: usize = 1;
pub const HEIGHT: usize = 2;
pub const EMISSION: usize = 3;

/// One texel of a paint layer: every PBR channel with its own coverage,
/// so a stroke can paint colour only, or colour + height + roughness +
/// metallic + emission at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Texel {
    /// sRGB colour; `rgba[3]` is the colour coverage.
    pub rgba: [u8; 4],
    /// Roughness, metallic, height (128 = level) and emission.
    pub pbr: [u8; 4],
    /// Coverage of each `pbr` channel.
    pub cov: [u8; 4],
}

/// What a tile element must do to be stored and saved.
pub trait Cell: Copy + Default + PartialEq + Send + Sync + 'static {
    const BYTES: usize;
    fn write(&self, out: &mut Vec<u8>);
    fn read(b: &[u8]) -> Self;
}

impl Cell for u8 {
    const BYTES: usize = 1;
    fn write(&self, out: &mut Vec<u8>) {
        out.push(*self);
    }
    fn read(b: &[u8]) -> Self {
        b[0]
    }
}

impl Cell for Texel {
    const BYTES: usize = 12;
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.rgba);
        out.extend_from_slice(&self.pbr);
        out.extend_from_slice(&self.cov);
    }
    fn read(b: &[u8]) -> Self {
        let q = |i: usize| [b[i], b[i + 1], b[i + 2], b[i + 3]];
        Texel { rgba: q(0), pbr: q(4), cov: q(8) }
    }
}

/// A square image of `Cell`s in copy-on-write tiles. Row 0 is the top
/// (v = 1), like [`crate::materials::Texture`].
#[derive(Clone)]
pub struct Tiles<T: Cell> {
    size: u32,
    per_row: u32,
    tiles: Vec<Option<Arc<[T]>>>,
}

impl<T: Cell> std::fmt::Debug for Tiles<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tiles({0}x{0}, {1} tiles)", self.size, self.allocated())
    }
}

impl<T: Cell> Tiles<T> {
    /// An empty image; `size` is rounded up to whole tiles.
    pub fn new(size: u32) -> Self {
        let per_row = size.max(1).div_ceil(TILE);
        Self { size: per_row * TILE, per_row, tiles: vec![None; (per_row * per_row) as usize] }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Tiles that hold data.
    pub fn allocated(&self) -> usize {
        self.tiles.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.allocated() == 0
    }

    /// Bytes of texel data held (shared tiles counted once per image).
    pub fn bytes(&self) -> usize {
        self.allocated() * N * T::BYTES
    }

    fn locate(&self, x: i64, y: i64) -> (usize, usize) {
        let (x, y) = (x.rem_euclid(self.size as i64) as u32, y.rem_euclid(self.size as i64) as u32);
        (((y / TILE) * self.per_row + x / TILE) as usize, ((y % TILE) * TILE + x % TILE) as usize)
    }

    /// The texel at (x, y), wrapping; the default where nothing was painted.
    pub fn get(&self, x: i64, y: i64) -> T {
        let (t, i) = self.locate(x, y);
        self.tiles[t].as_ref().map_or_else(T::default, |d| d[i])
    }

    /// Writes one texel, copying its tile first if a snapshot shares it.
    pub fn set(&mut self, x: i64, y: i64, v: T) {
        let (t, i) = self.locate(x, y);
        if self.tiles[t].is_none() {
            if v == T::default() {
                return;
            }
            self.tiles[t] = Some(Arc::from(vec![T::default(); N]));
        }
        let tile = self.tiles[t].as_mut().expect("tile allocated above");
        if tile[i] != v {
            Arc::make_mut(tile)[i] = v;
        }
    }

    /// Whole tile `t` (row-major inside the tile), if it holds data.
    pub fn tile(&self, t: usize) -> Option<&[T]> {
        self.tiles.get(t).and_then(|x| x.as_deref())
    }

    /// Top-left texel of tile `t`.
    pub fn tile_origin(&self, t: usize) -> (u32, u32) {
        ((t as u32 % self.per_row) * TILE, (t as u32 / self.per_row) * TILE)
    }

    /// Tiles whose data differs from `other`'s by identity (a pointer
    /// compare per tile), so an unchanged image costs nothing to diff.
    /// Images of different sizes differ everywhere.
    pub fn changed_tiles(&self, other: &Self) -> Vec<usize> {
        if self.size != other.size {
            return (0..self.tiles.len()).collect();
        }
        let same = |a: &Option<Arc<[T]>>, b: &Option<Arc<[T]>>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        (0..self.tiles.len()).filter(|&i| !same(&self.tiles[i], &other.tiles[i])).collect()
    }

    /// Tiles this image shares with `other` (not copied since the snapshot).
    pub fn shared_with(&self, other: &Self) -> usize {
        self.allocated() - self.changed_tiles(other).iter().filter(|&&i| self.tiles[i].is_some()).count()
    }

    /// Drops every tile.
    pub fn clear(&mut self) {
        self.tiles.iter_mut().for_each(|t| *t = None);
    }

    /// Fills every texel with `f(x, y)`; all-default tiles stay unallocated.
    pub fn fill_with(&mut self, f: impl Fn(u32, u32) -> T + Sync) {
        let (per_row, f) = (self.per_row, &f);
        let tiles = crate::materials::proc_engine::grid::par_map(self.tiles.len(), |t| {
            let (ox, oy) = ((t as u32 % per_row) * TILE, (t as u32 / per_row) * TILE);
            let data: Vec<T> = (0..N as u32).map(|i| f(ox + i % TILE, oy + i / TILE)).collect();
            data.iter().any(|v| *v != T::default()).then(|| Arc::<[T]>::from(data))
        });
        self.tiles = tiles;
    }

    /// Texel coordinates (continuous) of a UV: x right, y down from v = 1.
    fn px(&self, uv: Vec2) -> (f32, f32) {
        (uv.x * self.size as f32 - 0.5, (1.0 - uv.y) * self.size as f32 - 0.5)
    }

    /// The four texels around `uv` with their bilinear weights; `None`
    /// when all four fall in unpainted tiles (the common, fast case).
    fn taps(&self, uv: Vec2) -> Option<[(T, f32); 4]> {
        let (x, y) = self.px(uv);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (xi, yi) = (x0 as i64, y0 as i64);
        let mut any = false;
        let mut tap = |dx: i64, dy: i64, w: f32| {
            let (t, i) = self.locate(xi + dx, yi + dy);
            match &self.tiles[t] {
                Some(d) => {
                    any = true;
                    (d[i], w)
                }
                None => (T::default(), w),
            }
        };
        let out = [tap(0, 0, (1.0 - fx) * (1.0 - fy)), tap(1, 0, fx * (1.0 - fy)), tap(0, 1, (1.0 - fx) * fy), tap(1, 1, fx * fy)];
        any.then_some(out)
    }
}

impl Tiles<u8> {
    /// Bilinear mask value 0..1.
    pub fn sample(&self, uv: Vec2) -> f32 {
        self.taps(uv).map_or(0.0, |t| t.iter().map(|(v, w)| *v as f32 * w).sum::<f32>() / 255.0)
    }
}

/// A paint layer read at one point: colour and PBR values with the
/// coverage (0..1) of each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// sRGB.
    pub color: Vec3,
    pub a: f32,
    /// Roughness, metallic, height (0.5 = level) and emission.
    pub pbr: [f32; 4],
    pub cov: [f32; 4],
}

impl Sample {
    /// Scales every coverage (a mask or a layer opacity).
    pub fn faded(mut self, k: f32) -> Self {
        self.a *= k;
        self.cov.iter_mut().for_each(|c| *c *= k);
        self
    }
}

impl Tiles<Texel> {
    /// Bilinear read, weighted by coverage so unpainted neighbours do not
    /// darken the edge of a stroke. `None` where nothing is painted.
    pub fn sample(&self, uv: Vec2) -> Option<Sample> {
        let taps = self.taps(uv)?;
        let (mut color, mut a, mut pbr, mut cov) = (Vec3::ZERO, 0.0, [0.0f32; 4], [0.0f32; 4]);
        for (t, w) in taps {
            let ca = t.rgba[3] as f32 / 255.0 * w;
            color += Vec3::new(t.rgba[0] as f32, t.rgba[1] as f32, t.rgba[2] as f32) / 255.0 * ca;
            a += ca;
            for k in 0..4 {
                let c = t.cov[k] as f32 / 255.0 * w;
                pbr[k] += t.pbr[k] as f32 / 255.0 * c;
                cov[k] += c;
            }
        }
        if a <= 0.0 && cov.iter().all(|c| *c <= 0.0) {
            return None;
        }
        if a > 0.0 {
            color /= a;
        }
        for k in 0..4 {
            if cov[k] > 0.0 {
                pbr[k] /= cov[k];
            }
        }
        Some(Sample { color, a, pbr, cov })
    }
}
