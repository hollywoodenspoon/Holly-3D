//! Extrude (shape modifier): pushes every spline along local Z, with
//! N-gon caps on closed splines.

use crate::mesh::EditablePoly;
use crate::splines::Shape;
use crate::splines::sweep::{cap, flip, loft};
use bevy::math::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct ExtrudeMod {
    pub amount: f32,
    pub segments: u32,
    pub cap_start: bool,
    pub cap_end: bool,
}

impl Default for ExtrudeMod {
    fn default() -> Self {
        Self { amount: 0.5, segments: 1, cap_start: true, cap_end: true }
    }
}

impl ExtrudeMod {
    pub fn build(&self, shape: &Shape) -> EditablePoly {
        let segs = self.segments.clamp(1, 64) as usize;
        let mut out = EditablePoly::new();
        for pl in shape.polylines() {
            if pl.pts.len() < 2 {
                continue;
            }
            // Counter-clockwise outlines get outward walls without flipping.
            let ccw = !pl.closed || pl.signed_area_xy() >= 0.0;
            let mut pts = pl.pts.clone();
            if !ccw {
                pts.reverse();
            }
            let rows: Vec<Vec<Vec3>> =
                (0..=segs).map(|r| pts.iter().map(|&p| p + Vec3::Z * self.amount * r as f32 / segs as f32).collect()).collect();
            let (mut m, ids) = loft(&rows, pl.closed, false, false);
            if pl.closed && pts.len() >= 3 {
                if self.cap_start {
                    let bottom: Vec<_> = ids[0].iter().rev().copied().collect();
                    cap(&mut m, &bottom);
                }
                if self.cap_end {
                    cap(&mut m, &ids[segs]);
                }
            }
            if self.amount < 0.0 {
                flip(&mut m);
            }
            m.finalize();
            out.append(&m, bevy::math::Mat4::IDENTITY);
        }
        out.finalize();
        out
    }
}
