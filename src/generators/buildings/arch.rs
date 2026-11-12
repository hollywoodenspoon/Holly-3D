//! Parametric architectural families: walls along a path with mitred
//! corners, self-framing door / window openings that cut their own hole in
//! the wall (built as wall pieces around the opening, so no Boolean pass is
//! needed), straight / L-shaped / spiral staircases and a cross-gable roof
//! for L / T plans.

use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StairKind {
    #[default]
    Straight,
    LShaped,
    Spiral,
}

impl StairKind {
    pub const ALL: [StairKind; 3] = [Self::Straight, Self::LShaped, Self::Spiral];
    pub fn label(self) -> &'static str {
        match self {
            Self::Straight => "Straight",
            Self::LShaped => "L-Shaped",
            Self::Spiral => "Spiral",
        }
    }
}

/// Offset lines of a wall centreline: the left (outer for a
/// counter-clockwise loop's right side) and right corner points, mitred
/// where segments meet so neighbouring wall boxes share their corners.
pub fn miter_offsets(pts: &[Vec2], closed: bool, half_t: f32) -> Vec<(Vec2, Vec2)> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let prev = if i > 0 { Some(pts[i - 1]) } else if closed { Some(pts[n - 1]) } else { None };
            let next = if i + 1 < n { Some(pts[i + 1]) } else if closed { Some(pts[0]) } else { None };
            let d0 = prev.map(|p| (pts[i] - p).normalize_or_zero());
            let d1 = next.map(|q| (q - pts[i]).normalize_or_zero());
            let (na, nb) = (d0.map(|d| d.perp()), d1.map(|d| d.perp()));
            let off = match (na, nb) {
                (Some(a), Some(b)) => {
                    let m = (a + b).normalize_or_zero();
                    let c = m.dot(a).max(0.25);
                    m * half_t / c
                }
                (Some(a), None) | (None, Some(a)) => a * half_t,
                _ => Vec2::ZERO,
            };
            (pts[i] + off, pts[i] - off)
        })
        .collect()
}

/// Walls `t` thick and `h` tall along a polyline (closed for a building
/// outline), with mitred corners.
pub fn wall_path(kit: &mut Kit, pts: &[Vec2], closed: bool, t: f32, z0: f32, h: f32) {
    let o = miter_offsets(pts, closed, t * 0.5);
    let segs = if closed { pts.len() } else { pts.len().saturating_sub(1) };
    for i in 0..segs {
        let j = (i + 1) % pts.len();
        let (l0, r0, l1, r1) = (o[i].0, o[i].1, o[j].0, o[j].1);
        // A box from four floor-plan corners, outward-facing.
        let base = [r0, r1, l1, l0];
        let area: f32 = (0..4).map(|k| base[k].perp_dot(base[(k + 1) % 4])).sum();
        let ring: Vec<Vec2> = if area < 0.0 { base.iter().rev().copied().collect() } else { base.to_vec() };
        kit.prism(&ring, z0, h, true, false);
    }
}

/// A door or window in a wall: centre distance along the wall, width,
/// sill height (0 for a door) and opening height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opening {
    pub at: f32,
    pub width: f32,
    pub sill: f32,
    pub height: f32,
    /// Pane / leaf Material ID.
    pub fill: u16,
}

/// A straight wall in the local XZ plane (from x = 0 to `len`, front face
/// at y = 0 facing -Y, `t` deep) with framed openings cut into it.
pub fn wall_with_openings(kit: &mut Kit, len: f32, t: f32, h: f32, openings: &[Opening]) {
    let wall = kit.mat;
    let mut ops: Vec<Opening> = openings
        .iter()
        .copied()
        .filter(|o| o.width > 0.05 && o.at - o.width * 0.5 >= 0.0 && o.at + o.width * 0.5 <= len && o.sill + o.height < h)
        .collect();
    ops.sort_by(|a, b| a.at.total_cmp(&b.at));
    let mut x = 0.0;
    for o in &ops {
        let (a, b) = (o.at - o.width * 0.5, o.at + o.width * 0.5);
        if a < x {
            continue; // overlapping openings: keep the first
        }
        kit.mat(wall).block(Vec3::new(x, 0.0, 0.0), Vec3::new(a, t, h));
        if o.sill > 0.0 {
            kit.block(Vec3::new(a, 0.0, 0.0), Vec3::new(b, t, o.sill));
        }
        kit.block(Vec3::new(a, 0.0, o.sill + o.height), Vec3::new(b, t, h));
        // Self-framing: jambs, head and sill, then the pane / door leaf.
        let f = 0.08;
        kit.mat(ids::TRIM);
        kit.block(Vec3::new(a, -0.04, o.sill), Vec3::new(a + f, t * 0.5, o.sill + o.height));
        kit.block(Vec3::new(b - f, -0.04, o.sill), Vec3::new(b, t * 0.5, o.sill + o.height));
        kit.block(Vec3::new(a, -0.04, o.sill + o.height - f), Vec3::new(b, t * 0.5, o.sill + o.height));
        if o.sill > 0.0 {
            kit.block(Vec3::new(a - 0.05, -0.1, o.sill - f), Vec3::new(b + 0.05, t * 0.5, o.sill));
        }
        let y = t * 0.45;
        kit.mat(o.fill).quad(
            Vec3::new(a + f, y, o.sill),
            Vec3::new(b - f, y, o.sill),
            Vec3::new(b - f, y, o.sill + o.height - f),
            Vec3::new(a + f, y, o.sill + o.height - f),
        );
        x = b;
    }
    kit.mat(wall).block(Vec3::new(x, 0.0, 0.0), Vec3::new(len, t, h));
}

/// Evenly spaced windows along a wall of length `len` (one every `bay`).
pub fn window_row(len: f32, bay: f32, w: f32, sill: f32, h: f32, mut fill: impl FnMut(usize) -> u16) -> Vec<Opening> {
    let n = ((len / bay.max(0.5)).floor() as usize).max(1);
    (0..n).map(|i| Opening { at: len * (i as f32 + 0.5) / n as f32, width: w, sill, height: h, fill: fill(i) }).collect()
}

/// A staircase climbing `rise` metres, starting at the local origin and
/// running along +Y (`width` wide). Returns the number of steps.
pub fn stairs(kit: &mut Kit, kind: StairKind, width: f32, rise: f32) -> usize {
    let n = ((rise / 0.18).round() as usize).max(2);
    let (sh, tread) = (rise / n as f32, 0.28);
    kit.mat(ids::FACADE);
    match kind {
        StairKind::Straight => {
            for i in 0..n {
                let y = i as f32 * tread;
                kit.block(Vec3::new(-width * 0.5, y, 0.0), Vec3::new(width * 0.5, y + tread, sh * (i + 1) as f32));
            }
        }
        StairKind::LShaped => {
            let first = n / 2;
            for i in 0..first {
                let y = i as f32 * tread;
                kit.block(Vec3::new(-width * 0.5, y, 0.0), Vec3::new(width * 0.5, y + tread, sh * (i + 1) as f32));
            }
            // Landing, then the second flight turns along +X.
            let ly = first as f32 * tread;
            let lz = sh * first as f32;
            kit.block(Vec3::new(-width * 0.5, ly, 0.0), Vec3::new(width * 0.5, ly + width, lz));
            for i in first..n {
                let x = width * 0.5 + (i - first) as f32 * tread;
                kit.block(Vec3::new(x, ly, 0.0), Vec3::new(x + tread, ly + width, sh * (i + 1) as f32));
            }
        }
        StairKind::Spiral => {
            let r = width.max(0.6);
            kit.mat(ids::IRON).frustum(Vec3::ZERO, 0.12, 0.12, rise + 1.0, 8, true);
            kit.mat(ids::FACADE);
            for i in 0..n {
                let (a0, a1) = (i as f32 * TAU / 14.0, (i + 1) as f32 * TAU / 14.0);
                let z = sh * i as f32;
                let p = |a: f32, rr: f32, zz: f32| Vec3::new(a.cos() * rr, a.sin() * rr, zz);
                let (c0, c1) = (Vec3::new(0.0, 0.0, z), Vec3::new(0.0, 0.0, z + 0.06));
                let b = [c0, p(a0, r, z), p(a1, r, z)];
                let t = [c1, p(a0, r, z + 0.06), p(a1, r, z + 0.06)];
                kit.poly(&[t[0], t[1], t[2]]);
                kit.poly(&[b[0], b[2], b[1]]);
                kit.quad(b[1], b[2], t[2], t[1]);
                kit.quad(b[0], b[1], t[1], t[0]);
                kit.quad(b[2], b[0], t[0], t[2]);
            }
        }
    }
    n
}

/// Cross-gable roof over an L / T plan: a main gable along X over
/// `2hx` x `2hy` and a wing gable along Y over `2wx` wide reaching `wy`
/// out from the main ridge (positive = towards +Y).
pub fn cross_gable(kit: &mut Kit, hx: f32, hy: f32, wx: f32, wy: f32, pitch_deg: f32, wall: u16) {
    let tan = pitch_deg.clamp(5.0, 75.0).to_radians().tan();
    kit.gable(hx, hy, tan * hy, 0.4, wall);
    let half = wy.abs() * 0.5;
    let at = Mat4::from_translation(Vec3::new(0.0, wy.signum() * half, 0.0)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    kit.at(at, |k| k.gable(half, wx, tan * wx, 0.3, wall));
}
