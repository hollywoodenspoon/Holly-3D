//! 2D sketch constraints for Editable Spline, after SolidCraft's sketcher
//! (MIT / Apache-2.0). They are one-shot actions: the first selected item (in
//! spline / vertex order) is
//! the reference and every other selected item is moved to satisfy the
//! constraint once (nothing is solved again later).
//!
//! - Parallel / Perpendicular / Equal Length: Segment level (`2`). Each
//!   other selected segment turns (or stretches) to match the first. A
//!   segment that shares a vertex with the reference pivots on that vertex,
//!   so the reference never moves; others pivot on their midpoint.
//! - Tangent: Vertex level (`1`). The curve leaves each selected vertex in
//!   the direction it arrived, so the joint is smooth (G1) but keeps a
//!   corner-type handle pair.
//! - Concentric: Spline level (`3`). Every other selected spline moves so
//!   its centre sits on the first one's centre.

use super::edit::SplineLevel;
use super::{KnotType, Shape};
use bevy::math::{Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplineConstraint {
    Parallel,
    Perpendicular,
    Tangent,
    Concentric,
    EqualLength,
}

impl SplineConstraint {
    pub const ALL: [SplineConstraint; 5] = [
        SplineConstraint::Parallel,
        SplineConstraint::Perpendicular,
        SplineConstraint::EqualLength,
        SplineConstraint::Tangent,
        SplineConstraint::Concentric,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SplineConstraint::Parallel => "Parallel",
            SplineConstraint::Perpendicular => "Perpendicular",
            SplineConstraint::Tangent => "Tangent",
            SplineConstraint::Concentric => "Concentric",
            SplineConstraint::EqualLength => "Equal Length",
        }
    }

    pub fn tip(self) -> &'static str {
        match self {
            SplineConstraint::Parallel => "Segment level: turn the other selected segments parallel to the first",
            SplineConstraint::Perpendicular => "Segment level: turn the other selected segments perpendicular to the first",
            SplineConstraint::Tangent => "Vertex level: make the curve leave each selected vertex in the direction it arrived",
            SplineConstraint::Concentric => "Spline level: move the other selected splines onto the first one's centre",
            SplineConstraint::EqualLength => "Segment level: give the other selected segments the first one's length",
        }
    }

    /// The sub-object level the constraint works on.
    pub fn level(self) -> SplineLevel {
        match self {
            SplineConstraint::Tangent => SplineLevel::Vertex,
            SplineConstraint::Concentric => SplineLevel::Spline,
            _ => SplineLevel::Segment,
        }
    }
}

impl Shape {
    /// Selected segments as `(spline, segment)`, in order.
    pub fn selected_segments(&self) -> Vec<(usize, usize)> {
        let mut v = vec![];
        for (si, s) in self.splines.iter().enumerate() {
            v.extend((0..s.segment_count()).filter(|&i| s.knots[i].seg_selected).map(|i| (si, i)));
        }
        v
    }

    fn seg_ends(&self, (si, i): (usize, usize)) -> ((usize, usize), (usize, usize)) {
        ((si, i), (si, (i + 1) % self.splines[si].knots.len()))
    }

    fn pos(&self, (si, k): (usize, usize)) -> Vec3 {
        self.splines[si].knots[k].pos
    }

    /// Applies `c` once. Returns how many items it changed.
    pub fn constrain(&mut self, c: SplineConstraint) -> usize {
        match c {
            SplineConstraint::Tangent => self.make_tangent(),
            SplineConstraint::Concentric => self.make_concentric(),
            _ => self.constrain_segments(c),
        }
    }

    fn constrain_segments(&mut self, c: SplineConstraint) -> usize {
        let segs = self.selected_segments();
        let Some((&first, rest)) = segs.split_first() else { return 0 };
        let (r0, r1) = self.seg_ends(first);
        let ref_v = self.pos(r1) - self.pos(r0);
        let ref_dir = ref_v.normalize_or_zero();
        if ref_dir == Vec3::ZERO {
            return 0;
        }
        let mut n = 0;
        for &seg in rest {
            let (a, b) = self.seg_ends(seg);
            let (pa, pb) = (self.pos(a), self.pos(b));
            let dir = (pb - pa).normalize_or_zero();
            if dir == Vec3::ZERO {
                continue;
            }
            // Pivot on a vertex shared with the reference, else the midpoint.
            let shared = [a, b].into_iter().find(|k| *k == r0 || *k == r1);
            let pivot = shared.map_or((pa + pb) * 0.5, |k| self.pos(k));
            let movers: Vec<(usize, usize)> = [a, b].into_iter().filter(|k| Some(*k) != shared).collect();
            let m = match c {
                SplineConstraint::EqualLength => {
                    let k = ref_v.length() / (pb - pa).length();
                    Mat4::from_translation(pivot) * Mat4::from_scale(Vec3::splat(k)) * Mat4::from_translation(-pivot)
                }
                _ => {
                    let target = match c {
                        SplineConstraint::Parallel => ref_dir,
                        _ => {
                            let t = dir - ref_dir * dir.dot(ref_dir);
                            if t.length_squared() > 1e-10 { t.normalize() } else { Vec3::Z.cross(ref_dir).normalize_or(Vec3::Y) }
                        }
                    };
                    // Turn the short way round (an antiparallel line is already parallel).
                    let target = if target.dot(dir) < 0.0 { -target } else { target };
                    let q = Quat::from_rotation_arc(dir, target);
                    Mat4::from_translation(pivot) * Mat4::from_quat(q) * Mat4::from_translation(-pivot)
                }
            };
            if m.abs_diff_eq(Mat4::IDENTITY, 1e-6) {
                continue;
            }
            for (si, k) in movers {
                self.splines[si].knots[k].transform(m);
            }
            n += 1;
        }
        n
    }

    fn make_tangent(&mut self) -> usize {
        let mut n = 0;
        for (si, k) in self.selected_knots(SplineLevel::Vertex) {
            let s = &self.splines[si];
            let len = s.knots.len();
            if len < 2 || (!s.closed && (k == 0 || k + 1 == len)) {
                continue;
            }
            let prev = (k + len - 1) % len;
            let [a, b, c, d] = s.segment(prev);
            // Arrival direction: the last non-degenerate Bezier leg.
            let Some(dir) = [d - c, d - b, d - a].into_iter().map(|v| v.normalize_or_zero()).find(|v| *v != Vec3::ZERO) else { continue };
            let (in_h, out_h) = s.handles(k);
            let next = s.knots[(k + 1) % len].pos;
            let out_len = match (out_h - d).length() {
                l if l > 1e-6 => l,
                _ => (next - d).length() / 3.0,
            };
            let in_len = match (d - in_h).length() {
                l if l > 1e-6 => l,
                _ => (d - a).length() / 3.0,
            };
            let kn = &mut self.splines[si].knots[k];
            kn.kind = KnotType::BezierCorner;
            kn.in_h = d - dir * in_len;
            kn.out_h = d + dir * out_len;
            n += 1;
        }
        n
    }

    fn make_concentric(&mut self) -> usize {
        let centre = |s: &super::Spline| s.knots.iter().map(|k| k.pos).sum::<Vec3>() / s.knots.len().max(1) as f32;
        let sel: Vec<usize> = (0..self.splines.len()).filter(|&i| self.splines[i].selected && !self.splines[i].knots.is_empty()).collect();
        let Some((&first, rest)) = sel.split_first() else { return 0 };
        let target = centre(&self.splines[first]);
        let mut n = 0;
        for &i in rest {
            let d = target - centre(&self.splines[i]);
            if d.length_squared() > 1e-12 {
                self.splines[i].knots.iter_mut().for_each(|k| k.translate(d));
                n += 1;
            }
        }
        n
    }
}
