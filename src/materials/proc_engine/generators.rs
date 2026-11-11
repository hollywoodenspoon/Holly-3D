//! The tileable generators as one table: each [`TileKind`] has four knobs
//! (`scale`, `detail`, `width`, `warp`) and returns a height plus a random
//! id per tile. Counts are rounded to whole numbers, so every generator
//! repeats seamlessly over the 0..1 UV square at any resolution.

use super::lattice::{fbm, ridged};
use super::{fibers, masonry, voronoi};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TileKind {
    /// Running-bond bricks with mortar.
    #[default]
    Bricks,
    /// Stack bond / square tiles.
    StackTiles,
    Herringbone,
    /// Coursed rubble: rows of stones of random length ("medieval random").
    Coursed,
    Planks,
    /// Rounded stones.
    Cobbles,
    /// Flat rock plates / flagstones.
    Plates,
    /// Warped Voronoi cracks with side branches.
    Cracks,
    /// Dried-mud crackle that fades out in places.
    Crackle,
    Cells,
    WoodGrain,
    BrushedMetal,
    DiamondPlate,
    Weave,
    Chainmail,
    CarbonFiber,
    Scratches,
    Asphalt,
    Fbm,
    Ridged,
}

/// One pixel of a generator: height 0..1 and the tile's random id 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Px {
    pub h: f32,
    pub id: f32,
}

/// One spinner of a generator: label (empty = unused), range and default.
pub type Knob = (&'static str, f32, f32, f32);
const NONE: Knob = ("", 0.0, 1.0, 0.0);

impl TileKind {
    pub const ALL: [TileKind; 20] = [
        TileKind::Bricks,
        TileKind::StackTiles,
        TileKind::Herringbone,
        TileKind::Coursed,
        TileKind::Planks,
        TileKind::Cobbles,
        TileKind::Plates,
        TileKind::Cracks,
        TileKind::Crackle,
        TileKind::Cells,
        TileKind::WoodGrain,
        TileKind::BrushedMetal,
        TileKind::DiamondPlate,
        TileKind::Weave,
        TileKind::Chainmail,
        TileKind::CarbonFiber,
        TileKind::Scratches,
        TileKind::Asphalt,
        TileKind::Fbm,
        TileKind::Ridged,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TileKind::Bricks => "Bricks (Running Bond)",
            TileKind::StackTiles => "Tiles (Stack Bond)",
            TileKind::Herringbone => "Herringbone",
            TileKind::Coursed => "Coursed Stone (Medieval)",
            TileKind::Planks => "Planks",
            TileKind::Cobbles => "Cobbles",
            TileKind::Plates => "Rock Plates / Flagstones",
            TileKind::Cracks => "Voronoi Cracks",
            TileKind::Crackle => "Crackle",
            TileKind::Cells => "Cells",
            TileKind::WoodGrain => "Wood Grain",
            TileKind::BrushedMetal => "Brushed Metal",
            TileKind::DiamondPlate => "Diamond Plate",
            TileKind::Weave => "Weave",
            TileKind::Chainmail => "Chainmail",
            TileKind::CarbonFiber => "Carbon Fibre",
            TileKind::Scratches => "Scratches",
            TileKind::Asphalt => "Asphalt",
            TileKind::Fbm => "Fractal Noise",
            TileKind::Ridged => "Ridged Noise",
        }
    }

    /// What `scale`, `detail`, `width` and `warp` mean for this kind.
    pub fn knobs(self) -> [Knob; 4] {
        const GAP: Knob = ("Gap / Mortar Width", 0.0, 0.6, 0.1);
        const BEVEL: Knob = ("Bevel", 0.005, 1.0, 0.15);
        match self {
            TileKind::Bricks | TileKind::StackTiles => [("Columns", 1.0, 32.0, 4.0), ("Rows", 1.0, 64.0, 8.0), GAP, BEVEL],
            TileKind::Herringbone => [("Tiling", 1.0, 8.0, 2.0), ("Plank Length", 2.0, 8.0, 3.0), ("Gap", 0.0, 0.5, 0.08), BEVEL],
            TileKind::Coursed => [("Stones per Row", 1.0, 16.0, 3.0), ("Rows", 1.0, 64.0, 6.0), GAP, BEVEL],
            TileKind::Planks => [("Boards per Row", 1.0, 16.0, 2.0), ("Rows", 1.0, 64.0, 8.0), ("Gap", 0.0, 0.4, 0.04), ("Bevel", 0.005, 1.0, 0.08)],
            TileKind::Cobbles => [("Stones", 1.0, 32.0, 6.0), ("Height Variation", 0.0, 1.0, 0.4), ("Gap", 0.0, 0.5, 0.12), ("Bevel", 0.01, 0.5, 0.2)],
            TileKind::Plates => [("Plates", 1.0, 32.0, 5.0), ("Height Variation", 0.0, 1.0, 0.3), ("Gap", 0.0, 0.5, 0.06), ("Warp", 0.0, 2.0, 0.4)],
            TileKind::Cracks => [("Cells", 1.0, 32.0, 5.0), ("Branches", 0.0, 1.0, 0.5), ("Crack Width", 0.005, 0.5, 0.05), ("Warp", 0.0, 2.0, 0.6)],
            TileKind::Crackle => [("Cells", 1.0, 32.0, 6.0), ("Amount", 0.0, 1.0, 0.8), ("Crack Width", 0.005, 0.5, 0.06), ("Disorder", 0.0, 2.0, 0.5)],
            TileKind::Cells => [("Cells", 1.0, 64.0, 8.0), ("Flatness", 0.0, 1.0, 0.3), ("Gap", 0.0, 0.5, 0.05), ("Jitter", 0.0, 1.0, 1.0)],
            TileKind::WoodGrain => [("Rings", 1.0, 40.0, 10.0), ("Stretch", 1.0, 32.0, 8.0), ("Sharpness", 0.1, 6.0, 2.0), ("Fibres", 0.0, 1.0, 0.6)],
            TileKind::BrushedMetal => [("Streaks", 8.0, 512.0, 160.0), ("Streak Length", 1.0, 16.0, 2.0), ("Sharpness", 0.5, 8.0, 2.0), ("Depth", 0.0, 1.0, 0.6)],
            TileKind::DiamondPlate => [("Tiling", 2.0, 32.0, 8.0), ("Length", 0.2, 1.0, 0.7), ("Width", 0.05, 0.6, 0.22), NONE],
            TileKind::Weave => [("Threads", 2.0, 128.0, 16.0), NONE, ("Thread Width", 0.1, 1.0, 0.85), NONE],
            TileKind::Chainmail => [("Rings", 2.0, 64.0, 12.0), NONE, ("Wire Width", 0.05, 0.5, 0.18), ("Ring Size", 0.5, 1.2, 0.9)],
            TileKind::CarbonFiber => [("Tows", 1.0, 32.0, 6.0), ("Fibres per Tow", 2.0, 16.0, 6.0), ("Gap", 0.0, 0.4, 0.08), NONE],
            TileKind::Scratches => [("Density", 1.0, 32.0, 8.0), ("Length", 0.1, 1.0, 0.6), ("Width", 0.002, 0.1, 0.012), ("Angle Spread", 0.0, 1.0, 0.25)],
            TileKind::Asphalt => [("Aggregate", 4.0, 128.0, 48.0), ("Pits", 0.0, 1.0, 0.4), ("Stone Size", 0.0, 0.5, 0.15), ("Roughness", 0.0, 1.0, 0.5)],
            TileKind::Fbm | TileKind::Ridged => [("Cells", 1.0, 64.0, 4.0), ("Octaves", 1.0, 8.0, 5.0), ("Roughness", 0.1, 0.9, 0.5), ("Contrast", 0.0, 1.0, 0.0)],
        }
    }

    /// The four knob defaults.
    pub fn defaults(self) -> [f32; 4] {
        self.knobs().map(|k| k.3)
    }

    /// True for tiles with gaps (bricks, stones, planks); false for
    /// continuous textures (grain, noise, metal).
    pub fn has_gaps(self) -> bool {
        use TileKind::*;
        matches!(self, Bricks | StackTiles | Herringbone | Coursed | Planks | Cobbles | Plates | Cells)
    }
}

fn contrast(v: f32, c: f32) -> f32 {
    ((v - 0.5) * (1.0 + c * 4.0) + 0.5).clamp(0.0, 1.0)
}

/// Generator `kind` at `u`, `v`. `k` is `[scale, detail, width, warp]`
/// (see [`TileKind::knobs`]).
pub fn sample(kind: TileKind, u: f32, v: f32, k: [f32; 4], seed: u32) -> Px {
    let whole = |x: f32| x.round().max(1.0);
    let even = |x: f32| (x * 0.5).round().max(1.0) * 2.0;
    let flat = |h: f32| Px { h, id: 0.5 };
    let px = match kind {
        TileKind::Bricks => masonry::bricks(u, v, whole(k[0]), whole(k[1]), k[2], k[3], true, seed),
        TileKind::StackTiles => masonry::bricks(u, v, whole(k[0]), whole(k[1]), k[2], k[3], false, seed),
        TileKind::Herringbone => masonry::herringbone(u, v, whole(k[0]) as i32, (whole(k[1]) as i32).clamp(2, 8), k[2], k[3], seed),
        TileKind::Coursed => masonry::coursed(u, v, whole(k[0]), whole(k[1]), k[2], k[3], 0.8, seed),
        TileKind::Planks => masonry::coursed(u, v, whole(k[0]), whole(k[1]), k[2], k[3], 0.4, seed),
        TileKind::Cobbles => voronoi::stones(u, v, whole(k[0]), k[2], k[3], k[1], seed),
        TileKind::Plates => voronoi::plates(u, v, whole(k[0]), k[2], k[3], k[1], seed),
        TileKind::Cracks => flat(1.0 - voronoi::cracks(u, v, whole(k[0]), k[2], k[3], k[1], seed)),
        TileKind::Crackle => flat(1.0 - voronoi::crackle(u, v, whole(k[0]), k[2], k[3], k[1], seed)),
        TileKind::Cells => voronoi::cells(u, v, whole(k[0]), k[2], k[1], k[3], seed),
        TileKind::WoodGrain => flat(fibers::wood_grain(u, v, k[0].max(1.0), whole(k[1]), k[2].max(0.1), k[3], seed)),
        TileKind::BrushedMetal => flat(fibers::brushed(u, v, whole(k[0]), whole(k[1]), k[2], k[3], seed)),
        TileKind::DiamondPlate => flat(fibers::diamond_plate(u, v, even(k[0]), k[1], k[2])),
        TileKind::Weave => fibers::weave(u, v, even(k[0]), k[2]),
        TileKind::Chainmail => flat(fibers::chainmail(u, v, even(k[0]), k[2], k[3])),
        TileKind::CarbonFiber => fibers::carbon(u, v, whole(k[0]), whole(k[1]), k[2]),
        TileKind::Scratches => flat(fibers::scratches(u, v, whole(k[0]), k[1], k[2], k[3], seed)),
        TileKind::Asphalt => flat(voronoi::asphalt(u, v, whole(k[0]), k[1], k[2], k[3], seed)),
        TileKind::Fbm => flat(contrast(fbm(u, v, whole(k[0]), k[1] as u32, k[2], seed), k[3])),
        TileKind::Ridged => flat(contrast(ridged(u, v, whole(k[0]), k[1] as u32, k[2], seed), k[3])),
    };
    Px { h: if px.h.is_finite() { px.h.clamp(0.0, 1.0) } else { 0.0 }, id: px.id.clamp(0.0, 1.0) }
}

/// Height only (what the Tileable Pattern map node shows).
pub fn height(kind: TileKind, u: f32, v: f32, k: [f32; 4], seed: u32) -> f32 {
    sample(kind, u, v, k, seed).h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_generator_tiles_seamlessly_and_stays_in_range() {
        for kind in TileKind::ALL {
            let k = kind.defaults();
            let (mut lo, mut hi) = (1.0f32, 0.0f32);
            // Wrapping one period further along either axis must give the same picture:
            // the seam differs no more than two neighbours inside the tile do.
            let (mut seam, mut inner) = (0.0f32, 0.0f32);
            let n = 96;
            let at = |x: i32, y: i32| height(kind, (x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32, k, 3);
            for i in 0..n {
                seam += (at(n - 1, i) - at(n, i)).abs() + (at(i, n - 1) - at(i, n)).abs();
                inner += (at(n / 2 - 1, i) - at(n / 2, i)).abs() + (at(i, n / 2 - 1) - at(i, n / 2)).abs();
                // The pattern one whole tile away is identical.
                assert!((at(i, 7) - at(i + n, 7)).abs() < 1e-3 && (at(7, i) - at(7, i - n)).abs() < 1e-3, "{} repeats", kind.label());
                for j in 0..n {
                    let h = at(i, j);
                    assert!((0.0..=1.0).contains(&h));
                    (lo, hi) = (lo.min(h), hi.max(h));
                }
            }
            assert!(seam <= inner * 3.0 + 1.5, "{}: seam {seam} vs inner {inner}", kind.label());
            assert!(hi - lo > 0.25, "{} is not flat: {lo}..{hi}", kind.label());
        }
    }

    #[test]
    fn tiles_have_ids_and_seeds_change_them() {
        let k = TileKind::Bricks.defaults();
        let a = sample(TileKind::Bricks, 0.1, 0.07, k, 1);
        let b = sample(TileKind::Bricks, 0.4, 0.07, k, 1);
        assert!(a.h > 0.0 && b.h > 0.0 && a.id != b.id, "neighbouring bricks differ");
        assert_ne!(a.id, sample(TileKind::Bricks, 0.1, 0.07, k, 2).id);
        assert_eq!(sample(TileKind::Bricks, 0.3, 0.001, k, 1).h, 0.0, "mortar joint");
        assert!(TileKind::Bricks.has_gaps() && !TileKind::WoodGrain.has_gaps());
    }
}
