//! The procedural texture engine: tileable generators on a whole-number
//! lattice, image filters, weathering and a content-addressed cache, with
//! a background worker so spinner edits never block the UI.
//!
//! - [`lattice`]: hashing, value / Perlin / fractal / cellular noise.
//! - [`generators`]: the generator table ([`TileKind`]) over [`masonry`],
//!   [`voronoi`] and [`fibers`].
//! - [`adjust`] and [`filters`]: Levels, Height Blend, Slope Blur, Normal
//!   From Height, cavity AO.
//! - [`surface`], [`styles`], [`weather`]: the Compact editor's "Procedural
//!   Pattern & Weathering" recipe and its cook.
//! - [`cache`], [`node_key`], [`worker`]: content keys, the LRU and
//!   background cooking.
//!
//! The lattice noise, generators, filters and cache design are ported from
//! MaterialCraft (https://github.com/storytold/materialcraft), copyright
//! (c) 2026 ArtCraft Team and the MaterialCraft contributors, MIT OR
//! Apache-2.0. Each file says what it took.

pub mod adjust;
pub mod cache;
pub mod fibers;
pub mod filters;
pub mod generators;
pub mod grid;
pub mod lattice;
pub mod masonry;
pub mod node_key;
pub mod styles;
pub mod surface;
#[cfg(test)]
mod tests;
pub mod voronoi;
pub mod weather;
pub mod worker;

pub use generators::TileKind;
pub use grid::Grid;
pub use styles::Pattern;
pub use surface::{ProcMaps, ProcSurface};
