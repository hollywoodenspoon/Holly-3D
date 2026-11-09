//! Lathe: revolves a shape's splines around an axis into a mesh.

use crate::mesh::EditablePoly;
use crate::splines::Shape;
use crate::splines::sweep::{cap, flip, loft};
use bevy::math::{Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LatheAlign {
    #[default]
    Min,
    Center,
    Max,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatheMod {
    pub degrees: f32,
    pub segments: u32,
    pub weld_core: bool,
    pub flip_normals: bool,
    pub cap_start: bool,
    pub cap_end: bool,
    /// 0 = X, 1 = Y, 2 = Z (local).
    pub axis: usize,
    pub align: LatheAlign,
    /// Axis sub-object offset (moved with W).
    pub axis_offset: Vec3,
}

impl Default for LatheMod {
    fn default() -> Self {
        Self {
            degrees: 360.0,
            segments: 16,
            weld_core: true,
            flip_normals: false,
            cap_start: true,
            cap_end: true,
            axis: 1,
            align: LatheAlign::Min,
            axis_offset: Vec3::ZERO,
        }
    }
}

impl LatheMod {
    pub fn axis_dir(&self) -> Vec3 {
        [Vec3::X, Vec3::Y, Vec3::Z][self.axis.min(2)]
    }

    /// A point on the revolution axis (local space) for `shape`.
    pub fn axis_point(&self, shape: &Shape) -> Vec3 {
        let Some(b) = shape.bounds() else { return self.axis_offset };
        // radial axis: the in-plane axis perpendicular to the lathe axis
        let r = if self.axis == 0 { 1 } else { 0 };
        let mut c = b.center();
        c[r] = match self.align {
            LatheAlign::Min => b.min[r],
            LatheAlign::Center => c[r],
            LatheAlign::Max => b.max[r],
        };
        c + self.axis_offset
    }

    pub fn build(&self, shape: &Shape) -> EditablePoly {
        let (dir, origin) = (self.axis_dir(), self.axis_point(shape));
        let full = self.degrees.abs() >= 359.99;
        let segs = self.segments.clamp(3, 128) as usize;
        let cols = if full { segs } else { segs + 1 };
        let mut out = EditablePoly::new();
        for pl in shape.polylines() {
            if pl.pts.len() < 2 {
                continue;
            }
            let rows: Vec<Vec<Vec3>> = pl
                .pts
                .iter()
                .map(|&p| {
                    (0..cols)
                        .map(|c| {
                            let a = (self.degrees * c as f32 / segs as f32).to_radians();
                            origin + Quat::from_axis_angle(dir, a) * (p - origin)
                        })
                        .collect()
                })
                .collect();
            let (mut m, ids) = loft(&rows, full, pl.closed, self.weld_core);
            if !full && pl.pts.len() >= 3 {
                let first: Vec<_> = ids.iter().map(|r| r[0]).collect();
                let last: Vec<_> = ids.iter().rev().map(|r| r[cols - 1]).collect();
                if self.cap_start {
                    cap(&mut m, &first);
                }
                if self.cap_end {
                    cap(&mut m, &last);
                }
            }
            // Face away from the axis by default (3ds Max users then use Flip Normals).
            let outward: f32 = m
                .polygons
                .keys()
                .map(|p| {
                    let c = m.poly_centroid(p) - origin;
                    let radial = c - dir * c.dot(dir);
                    m.poly_normal_raw(p).dot(radial)
                })
                .sum();
            if (outward < 0.0) != self.flip_normals {
                flip(&mut m);
            }
            m.finalize();
            out.append(&m, bevy::math::Mat4::IDENTITY);
        }
        out.finalize();
        out
    }
}
