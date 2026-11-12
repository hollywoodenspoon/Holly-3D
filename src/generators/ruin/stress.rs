//! Load-path stress model. A grid of columns spans the footprint, one per
//! ~5 m bay, each with unit capacity per storey. Blasts (seeded, more and
//! stronger with the destruction level) strip capacity from the columns
//! near them, falling off with distance. Storey `s` carries the floors
//! above it, so its demand is `(storeys - s) / storeys` scaled by the
//! level's load factor; the lowest storey whose mean remaining capacity is
//! below its demand is the fracture storey. The building tilts toward the
//! capacity-weighted side that lost the most support.

use super::DestructionLevel as L;
use crate::mesh::Aabb;
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Storey height assumed for an arbitrary mesh.
pub const STOREY: f32 = 3.2;

pub struct Stress {
    /// Blast centres (object space).
    pub blasts: Vec<Vec3>,
    /// Number of storeys over the mesh height.
    pub storeys: usize,
    /// Storey that failed (0 = ground floor), if any.
    pub failed: Option<usize>,
    /// Unit direction toward the side that lost support.
    pub tilt: Vec2,
    /// 0..1: how lopsided the loss of support is.
    pub asym: f32,
}

impl Stress {
    pub fn new(b: &Aabb, level: L, r: &mut ChaCha8Rng) -> Self {
        let size = b.max - b.min;
        let storeys = ((size.z / STOREY).round() as usize).max(1);
        let (blasts_n, power, load) = match level {
            L::Intact => (0, 0.0, 0.0),
            L::Light => (1, 0.35, 0.3),
            L::Heavy => (2, 0.65, 0.55),
            L::Collapse => (3, 0.95, 0.8),
            L::Ruin | L::Rubble => (4, 1.2, 1.0),
        };
        // Blasts hit the facades in the lower two thirds.
        let blasts: Vec<Vec3> = (0..blasts_n)
            .map(|_| {
                let side = r.gen_range(0..4);
                let t = r.gen_range(0.1..0.9);
                let xy = match side {
                    0 => Vec2::new(b.min.x + size.x * t, b.min.y),
                    1 => Vec2::new(b.max.x, b.min.y + size.y * t),
                    2 => Vec2::new(b.min.x + size.x * t, b.max.y),
                    _ => Vec2::new(b.min.x, b.min.y + size.y * t),
                };
                xy.extend(b.min.z + size.z * r.gen_range(0.0..0.66))
            })
            .collect();
        let reach = (size.x.max(size.y) * 0.45).max(4.0);
        let (nx, ny) = (((size.x / 5.0).ceil() as usize).clamp(2, 12), ((size.y / 5.0).ceil() as usize).clamp(2, 12));
        let centre = Vec2::new(b.min.x + b.max.x, b.min.y + b.max.y) * 0.5;
        let mut failed = None;
        let (mut lost_at, mut lost) = (Vec2::ZERO, 0.0);
        for s in 0..storeys {
            let z = b.min.z + (s as f32 + 0.5) * size.z / storeys as f32;
            let mut cap = 0.0;
            for i in 0..nx {
                for j in 0..ny {
                    let p = Vec2::new(b.min.x + size.x * (i as f32 + 0.5) / nx as f32, b.min.y + size.y * (j as f32 + 0.5) / ny as f32);
                    let hit: f32 = blasts.iter().map(|q| power * (1.0 - p.extend(z).distance(*q) / reach).max(0.0)).sum();
                    let c = (1.0 - hit).max(0.0);
                    cap += c;
                    if s == 0 || failed.is_none() {
                        lost_at += (p - centre) * (1.0 - c);
                        lost += 1.0 - c;
                    }
                }
            }
            let cap = cap / (nx * ny) as f32;
            let demand = load * (storeys - s) as f32 / storeys as f32;
            if failed.is_none() && cap < demand {
                failed = Some(s);
            }
        }
        let tilt = if lost > 0.0 { (lost_at / lost).normalize_or(Vec2::NEG_Y) } else { Vec2::NEG_Y };
        let asym = if lost > 0.0 { ((lost_at / lost).length() / (size.truncate().length() * 0.5).max(0.1)).clamp(0.0, 1.0) } else { 0.0 };
        Self { blasts, storeys, failed, tilt, asym }
    }

    /// Height of the fracture (top of the failed storey's slab), defaulting
    /// to the weakest third of the building when nothing failed outright.
    pub fn fracture(&self, b: &Aabb, r: &mut ChaCha8Rng) -> f32 {
        let h = b.max.z - b.min.z;
        let s = self.failed.unwrap_or_else(|| r.gen_range(0..=(self.storeys / 3)));
        let z = b.min.z + h * (s as f32 + 1.0) / self.storeys as f32;
        z.clamp(b.min.z + h * 0.18, b.min.z + h * 0.7)
    }
}
