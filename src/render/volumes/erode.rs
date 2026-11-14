//! Clouds V2: a cloud volume's puffs plus the erosion that wears their
//! soft union into billows and wisps. The same noise runs in the GPU kernel
//! (`gpu_pathtrace/wgsl/volobj.wgsl`), so both renderers see one cloud.
//!
//! Erosion only ever lowers density (it multiplies the puff union by a
//! 0..=1 cut), so the puff majorants stay valid.

use super::density::Blob;
use crate::generators::clouds::CloudLook;
use crate::render::sky_noise::{hash3, vnoise};
use bevy::math::Vec3;

/// F10 Render Setup -> Volumes -> Noise Detail: billow octaves of erosion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoiseDetail {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl NoiseDetail {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low (fastest)",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra (slowest)",
        }
    }
    pub fn octaves(self) -> u32 {
        self as u32 + 1
    }
}

/// The puff cloud of one volume object.
#[derive(Clone, Debug, PartialEq)]
pub struct Cloud {
    pub blobs: Vec<Blob>,
    pub look: CloudLook,
    /// Noise frequency (1 / feature size in metres).
    pub inv_scale: f32,
    /// Per-seed shift in noise space so every cloud wears differently.
    pub offset: Vec3,
    /// Base and top heights (local z) for Wispiness.
    pub z0: f32,
    pub z1: f32,
    /// Billow octaves (Noise Detail).
    pub octaves: u32,
}

impl Cloud {
    /// A cloud with erosion features about `feature` metres across.
    pub fn new(blobs: Vec<Blob>, look: CloudLook, feature: f32, seed: u32) -> Self {
        let s = seed as f32;
        let offset = Vec3::new((s * 0.618_034).fract(), (s * 0.414_214).fract(), (s * 0.732_051).fract()) * 97.0;
        Self { blobs, look, inv_scale: 1.0 / feature.max(1e-4), offset, z0: 0.0, z1: 1.0, octaves: NoiseDetail::default().octaves() }
    }

    pub fn edge(&self) -> f32 {
        self.look.edge_softness.clamp(0.05, 1.0)
    }

    /// Density at `p` from the puffs listed for its majorant cell.
    pub fn density(&self, p: Vec3, list: &[u32]) -> f32 {
        let e = self.edge();
        let (mut keep, mut keep_g) = (1.0, 1.0);
        for &k in list {
            let b = &self.blobs[k as usize];
            keep *= 1.0 - b.at(p, e);
            keep_g *= 1.0 - b.at(p, 1.0);
        }
        self.erode(p, 1.0 - keep, 1.0 - keep_g)
    }

    /// Erosion of the union density `b` at `p`, where `g` (the union of
    /// fully soft puffs) measures depth inside the cloud: the noise cuts
    /// down to depth `x = s * n`, so the gaps between Worley cells become
    /// clefts between billows while deep cores stay whole.
    pub fn erode(&self, p: Vec3, b: f32, g: f32) -> f32 {
        if b <= 0.0 || self.look.erosion <= 0.0 {
            return b;
        }
        let h = (p.z - self.z0) / (self.z1 - self.z0).max(1e-6);
        let x = self.look.strength(h) * noise(p * self.inv_scale + self.offset, self.octaves);
        b * cut(g - x)
    }
}

/// Width of the smooth step at the bottom of an erosion cut (in depth).
const CUT: f32 = 0.2;

/// 0 below the cut, 1 above it, smooth across `CUT`.
fn cut(v: f32) -> f32 {
    let t = (v / CUT).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Distance to the nearest jittered feature point (Worley F1), 0..=1.
fn worley(q: Vec3) -> f32 {
    let i = q.floor();
    let (x, y, z) = (i.x as i32, i.y as i32, i.z as i32);
    let mut best: f32 = 9.0;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (cx, cy, cz) = (x + dx, y + dy, z + dz);
                let j = Vec3::new(hash3(cx, cy, cz), hash3(cx + 71, cy, cz), hash3(cx, cy + 113, cz));
                let c = Vec3::new(cx as f32, cy as f32, cz as f32) + j;
                best = best.min((c - q).length_squared());
            }
        }
    }
    best.sqrt().min(1.0)
}

/// Billowy fractal noise 0..=1 (folded value noise).
fn billow(q: Vec3, octaves: u32) -> f32 {
    let (mut s, mut a, mut f, mut n) = (0.0, 0.5, 1.0, 0.0);
    for _ in 0..octaves.max(1) {
        s += (vnoise(q * f) * 2.0 - 1.0).abs() * a;
        n += a;
        a *= 0.5;
        f *= 2.03;
    }
    s / n
}

/// Erosion noise 0..=1: high in the gaps between round Worley cells (so
/// erosion carves cauliflower billows) plus finer billows, contrast
/// stretched so the gaps erode fully and the cell middles barely.
pub fn noise(q: Vec3, octaves: u32) -> f32 {
    let n = 0.55 * worley(q) + 0.45 * billow(q * 2.0, octaves);
    let t = ((n - 0.15) / 0.6).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
