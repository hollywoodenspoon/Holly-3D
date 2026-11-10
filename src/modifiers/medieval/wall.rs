//! Castle Curtain Wall & Gatehouse on a Box (along its long axis) or along
//! a spline path: thick battered stone walls with a recessed wall-walk,
//! front / rear crenellations, periodic stone buttresses, corner towers on
//! paths, and a toggleable centre gatehouse arch with an iron portcullis.

use super::parts::{self, Masonry};
use super::{ArchLayout, ArchitectMod, Footprint};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use std::f32::consts::FRAC_PI_2;

pub fn build(kit: &mut Kit, a: &ArchitectMod, s: Vec3, lay: &ArchLayout, m: &Masonry) {
    // Run along the long axis; the box's short side is the thickness.
    let along_x = s.x >= s.y;
    let (len, t, h) = (s.x.max(s.y), s.x.min(s.y).max(0.6), s.z);
    let f = if along_x { Mat4::IDENTITY } else { Mat4::from_rotation_z(FRAC_PI_2) };
    kit.at(f, |k| run(k, a, len, t, h, lay, m, a.door));
}

/// One straight wall run centred on the origin along local X.
#[allow(clippy::too_many_arguments)]
fn run(k: &mut Kit, a: &ArchitectMod, len: f32, t: f32, h: f32, lay: &ArchLayout, m: &Masonry, gate: bool) {
    let gw = if gate { (h * 0.9).clamp(4.0, len * 0.4) } else { 0.0 };
    // Battered (sloping) base course.
    let batter = (h * 0.15).clamp(0.5, 3.0);
    k.mat(ids::FACADE).frustum_box(len, t, batter, 0.45);
    let pieces: Vec<(f32, f32)> = if gate { vec![(-len * 0.5, -gw * 0.5), (gw * 0.5, len * 0.5)] } else { vec![(-len * 0.5, len * 0.5)] };
    for (x0, x1) in pieces {
        if x1 - x0 > 0.2 {
            parts::wall_segment(k, Vec2::new(x0, 0.0), Vec2::new(x1, 0.0), 0.0, h, t, m, true);
        }
    }
    if gate {
        parts::gatehouse(k, Vec2::new(-gw * 0.5, 0.0), Vec2::new(gw * 0.5, 0.0), h, t, m);
        for s in [-1.0f32, 1.0] {
            parts::round_tower(k, Vec3::new(s * (gw * 0.5 + t * 0.4), -t * 0.4, 0.0), t * 0.9, h + 3.0, m, a.roof);
        }
    }
    if a.buttresses {
        let n = lay.buttresses.max(1);
        for i in 0..n {
            let x = -len * 0.5 + len * (i as f32 + 0.5) / n as f32;
            if gate && x.abs() < gw * 0.5 + t * 1.5 {
                continue;
            }
            buttress(k, x, -t * 0.5, h * 0.8, (t * 0.5).clamp(0.6, 2.0));
        }
    }
    if a.windows {
        let n = lay.windows_per_floor.max(1);
        for i in 0..n {
            let x = -len * 0.5 + len * (i as f32 + 0.5) / n as f32;
            if !(gate && x.abs() < gw * 0.5 + t) {
                k.at(Mat4::from_translation(Vec3::new(0.0, -t * 0.5 - m.depth - 0.02, 0.0)), |k| parts::arrow_slit(k, x, h * 0.45, (h * 0.25).min(1.8)));
            }
        }
    }
}

/// Stepped stone buttress against the face at y = `face` (outward -Y).
fn buttress(k: &mut Kit, x: f32, face: f32, h: f32, d: f32) {
    k.mat(ids::FACADE);
    let w = (d * 0.7).max(0.5);
    k.block(Vec3::new(x - w * 0.5, face - d, 0.0), Vec3::new(x + w * 0.5, face, h * 0.55));
    k.block(Vec3::new(x - w * 0.42, face - d * 0.6, h * 0.55), Vec3::new(x + w * 0.42, face, h));
    k.quad(Vec3::new(x - w * 0.42, face - d * 0.6, h), Vec3::new(x + w * 0.42, face - d * 0.6, h), Vec3::new(x + w * 0.42, face, h + d * 0.6), Vec3::new(x - w * 0.42, face, h + d * 0.6));
}

/// Walls following a spline path with a round tower on every knot.
pub fn path(kit: &mut Kit, a: &ArchitectMod, fp: &Footprint, m: &Masonry) {
    let Some((pts, closed)) = &fp.path else { return };
    let (h, t) = (a.spline_height.max(1.0), a.wall_thickness.max(0.5));
    let n = pts.len();
    let segs = if *closed { n } else { n.saturating_sub(1) };
    // Gatehouse in the middle of the longest run.
    let gate_seg = (0..segs).max_by(|&i, &j| pts[i].distance(pts[(i + 1) % n]).total_cmp(&pts[j].distance(pts[(j + 1) % n])));
    let lay = a.layout(fp);
    for i in 0..segs {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let d = q - p;
        let len = d.length();
        if len < 0.2 {
            continue;
        }
        let f = Mat4::from_translation(((p + q) * 0.5).extend(0.0)) * Mat4::from_rotation_z(d.y.atan2(d.x));
        let per = ArchLayout { buttresses: ((len / (a.window_spacing.max(0.5) * 2.5)).floor() as u32).max(1), windows_per_floor: ((len / a.window_spacing.max(0.5)) as u32).max(1), ..lay };
        kit.at(f, |k| run(k, a, len, t, h, &per, m, a.door && Some(i) == gate_seg && len > t * 6.0));
    }
    for (i, p) in pts.iter().enumerate() {
        if *closed || (i > 0 && i + 1 < n) || n == 2 {
            parts::round_tower(kit, p.extend(0.0), t * 1.1, h + 3.0, m, a.roof);
        }
    }
}

impl Kit {
    /// A battered plinth under a `len` x `t` wall: `h` tall, `grow` wider
    /// at the bottom on each side.
    pub fn frustum_box(&mut self, len: f32, t: f32, h: f32, grow: f32) {
        let (a, b) = (Vec2::new(len * 0.5 + grow, t * 0.5 + grow), Vec2::new(len * 0.5, t * 0.5));
        let lo = [(-a.x, -a.y), (a.x, -a.y), (a.x, a.y), (-a.x, a.y)].map(|(x, y)| self.vtx(Vec3::new(x, y, 0.0)));
        let hi = [(-b.x, -b.y), (b.x, -b.y), (b.x, b.y), (-b.x, b.y)].map(|(x, y)| self.vtx(Vec3::new(x, y, h)));
        for i in 0..4 {
            let j = (i + 1) % 4;
            self.face(&[lo[i], lo[j], hi[j], hi[i]]);
        }
        self.face(&hi);
        self.face(&[lo[3], lo[2], lo[1], lo[0]]);
    }
}
