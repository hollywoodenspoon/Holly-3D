//! Brush Strokes ("Painted Normals"): the material's normals are repainted
//! as brush dabs. Each dab tilts the surface one way, so it catches the
//! light as one stroke of colour, with a slight paint ridge across it and
//! optional bristle grooves along it. The dabs sit on a grid that repeats
//! every 1 UV (or one World / Triplanar tile), so strokes stay stuck to the
//! model, bake for the viewport and the GPU path tracer, and tile without
//! seams.

use bevy::math::{Vec2, Vec3};
use std::f32::consts::TAU;

/// Tilt of a dab at Strength 1 (tan 40°).
const MAX_TILT: f32 = 0.84;
/// Half the width of a dab, in grid cells (above 0.5 so dabs overlap).
const HALF_W: f32 = 0.8;
/// Longest stroke (length / width).
pub const MAX_LENGTH: f32 = 4.0;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BrushStrokes {
    /// "Painted Normals" checkbox.
    pub enabled: bool,
    /// Stroke width as a fraction of one UV repeat (or one World tile).
    pub size: f32,
    /// Length / width of a stroke (1 = round dabs).
    pub length: f32,
    /// How far each stroke tilts the surface (0..1; 1 = about 40°).
    pub strength: f32,
    /// Stroke direction in degrees (0 = along U, 90 = along V).
    pub angle: f32,
    /// Random spread of the direction in degrees (180 = any direction).
    pub jitter: f32,
    /// Bristle grooves along each stroke (0..1).
    pub bristles: f32,
    pub seed: u32,
}

impl Default for BrushStrokes {
    fn default() -> Self {
        CONCEPT
    }
}

const fn look(size: f32, length: f32, strength: f32, jitter: f32, bristles: f32) -> BrushStrokes {
    BrushStrokes { enabled: true, size, length, strength, angle: 0.0, jitter, bristles, seed: 1 }
}

const CONCEPT: BrushStrokes = BrushStrokes { enabled: false, ..look(0.06, 2.4, 0.6, 35.0, 0.25) };

/// One-click looks for the Brush Strokes rollout.
pub const PRESETS: [(&str, BrushStrokes); 3] =
    [("Gouache", look(0.08, 1.5, 0.7, 70.0, 0.1)), ("Oil", look(0.05, 3.4, 0.5, 20.0, 0.55)), ("Concept Art", look(0.06, 2.4, 0.6, 35.0, 0.25))];

/// One dab's look: which way it runs and tilts.
struct Dab {
    dir: Vec2,
    tilt: Vec2,
    phase: f32,
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// 10 bits of `h` from bit `at`, as 0..1.
fn bits(h: u32, at: u32) -> f32 {
    ((h >> at) & 1023) as f32 / 1023.0
}

impl Dab {
    /// A cell's key: one hash places and orders its dab (offset x / y and
    /// order in three 10-bit fields), so far-away cells cost one hash.
    fn key(row: u32, cx: i32) -> u32 {
        hash(row ^ (cx as u32).wrapping_mul(0x9e37_79b9))
    }

    fn new(s: &BrushStrokes, key: u32) -> Dab {
        let mut h = key;
        let mut r = || {
            h = hash(h);
            h as f32 / u32::MAX as f32
        };
        let a = s.angle.to_radians() + (r() * 2.0 - 1.0) * s.jitter.clamp(0.0, 180.0).to_radians();
        let (phi, mag) = (r() * TAU, s.strength.clamp(0.0, 1.0) * MAX_TILT * (0.35 + 0.65 * r()));
        Dab { dir: Vec2::from_angle(a), tilt: Vec2::from_angle(phi) * mag, phase: r() * TAU }
    }
}

impl BrushStrokes {
    /// Dabs across one UV repeat.
    pub fn cells(&self) -> i32 {
        (1.0 / self.size.clamp(0.005, 1.0)).round().max(1.0) as i32
    }

    /// The tangent-space normal at `uv` (+Z = untouched).
    pub fn tangent_normal(&self, uv: Vec2) -> Vec3 {
        let n = self.cells();
        let g = uv * n as f32;
        let half_l = HALF_W * self.length.clamp(1.0, MAX_LENGTH);
        let r = half_l.ceil() as i32 + 1;
        let reach2 = (half_l * 1.3).powi(2);
        let (bx, by) = (g.x.floor() as i32, g.y.floor() as i32);
        // Dabs in reach, topmost first (r <= 5, so at most 11 x 11 cells).
        let mut cand = [(0u32, Vec2::ZERO, 0u32); 121];
        let mut k = 0;
        let seed = hash(self.seed.wrapping_add(0x9e37_79b9));
        for cy in by - r..=by + r {
            let row = hash(seed ^ cy.rem_euclid(n) as u32);
            for cx in bx - r..=bx + r {
                let key = Dab::key(row, cx.rem_euclid(n));
                let rel = g - Vec2::new(cx as f32 + bits(key, 0), cy as f32 + bits(key, 10));
                if rel.length_squared() <= reach2 && k < cand.len() {
                    cand[k] = ((key >> 20) & 1023, rel, key);
                    k += 1;
                }
            }
        }
        let cand = &mut cand[..k];
        cand.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(b.2.cmp(&a.2)));
        // The topmost dab covering the point, else the one it is nearest to.
        let mut near: Option<(f32, Dab, f32)> = None;
        let mut top = None;
        for &(_, rel, key) in cand.iter() {
            let d = Dab::new(self, key);
            let (along, across) = (rel.dot(d.dir), rel.dot(d.dir.perp()));
            // Ragged edges: the outline wobbles along the stroke.
            let edge = 1.0 + 0.16 * (along * 4.0 + d.phase).sin() + 0.08 * (along * 9.0 - d.phase).sin();
            let e = ((along / half_l).powi(2) + (across / HALF_W).powi(2)) / edge;
            if e < 1.0 {
                top = Some((e, d, across));
                break;
            }
            if near.as_ref().is_none_or(|t| e < t.0) {
                near = Some((e, d, across));
            }
        }
        let Some((_, d, across)) = top.or(near) else { return Vec3::Z };
        // Paint ridge across the stroke and bristle grooves along it.
        let ridge = -0.3 * self.strength.clamp(0.0, 1.0) * (across / HALF_W).clamp(-1.0, 1.0);
        let groove = 0.35 * self.bristles.clamp(0.0, 1.0) * (across * TAU * 3.5 / HALF_W + d.phase).cos();
        let slope = d.tilt + d.dir.perp() * (ridge + groove);
        Vec3::new(slope.x, slope.y, 1.0).normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(s: &BrushStrokes, res: usize) -> Vec<Vec3> {
        (0..res * res).map(|i| s.tangent_normal(Vec2::new((i % res) as f32 + 0.5, (i / res) as f32 + 0.5) / res as f32)).collect()
    }

    #[test]
    fn normals_are_valid_and_face_out() {
        for (_, s) in PRESETS.iter().chain([("strong", BrushStrokes { strength: 1.0, bristles: 1.0, ..CONCEPT })].iter()) {
            for n in grid(s, 48) {
                assert!((n.length() - 1.0).abs() < 1e-4 && n.z > 0.5, "{n}");
            }
        }
    }

    #[test]
    fn each_stroke_is_one_tilt_with_edges_between_strokes() {
        let s = BrushStrokes { bristles: 0.0, strength: 1.0, ..CONCEPT };
        let res = 160;
        let g = grid(&s, res);
        let (mut same, mut edges, mut pairs) = (0, 0, 0);
        for y in 0..res {
            for x in 0..res - 1 {
                let a = g[y * res + x].angle_between(g[y * res + x + 1]).to_degrees();
                pairs += 1;
                same += usize::from(a < 3.0);
                edges += usize::from(a > 10.0);
            }
        }
        assert!(same * 10 > pairs * 8, "mostly smooth inside strokes: {same}/{pairs}");
        assert!(edges * 100 > pairs, "visible stroke edges: {edges}/{pairs}");
        assert!(grid(&BrushStrokes { strength: 0.0, ..s }, 16).iter().all(|n| (*n - Vec3::Z).length() < 1e-6), "Strength 0, no bristles: untouched");
    }

    #[test]
    fn strokes_repeat_every_uv_and_follow_the_seed() {
        let s = PRESETS[1].1.clone();
        for i in 0..40 {
            let uv = Vec2::new((i as f32 * 0.618).fract(), (i as f32 * 0.377).fract());
            for d in [Vec2::X, Vec2::Y, Vec2::new(-2.0, 1.0)] {
                assert!((s.tangent_normal(uv) - s.tangent_normal(uv + d)).length() < 1e-3, "seamless at {uv}");
            }
        }
        assert_eq!(grid(&s, 20), grid(&s.clone(), 20), "same seed, same strokes");
        assert_ne!(grid(&s, 20), grid(&BrushStrokes { seed: 7, ..s }, 20), "a new seed reshuffles them");
    }

    #[test]
    fn long_strokes_run_along_the_angle() {
        let runs = |angle: f32| {
            let s = BrushStrokes { length: MAX_LENGTH, jitter: 0.0, bristles: 0.0, angle, ..CONCEPT };
            let step = s.size * 0.4;
            let (mut along_u, mut along_v) = (0, 0);
            for i in 0..400 {
                let uv = Vec2::new((i as f32 * 0.618).fract(), (i as f32 * 0.377).fract());
                let n = s.tangent_normal(uv);
                along_u += usize::from(n.angle_between(s.tangent_normal(uv + Vec2::X * step)) < 0.02);
                along_v += usize::from(n.angle_between(s.tangent_normal(uv + Vec2::Y * step)) < 0.02);
            }
            (along_u, along_v)
        };
        let (u, v) = runs(0.0);
        assert!(u > v + 60, "angle 0 runs along U: {u} vs {v}");
        let (u, v) = runs(90.0);
        assert!(v > u + 60, "angle 90 runs along V: {u} vs {v}");
    }
}
