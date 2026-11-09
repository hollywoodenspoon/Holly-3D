//! Per-spline Editable Spline operations: Refine, knot type, Fillet /
//! Chamfer, Make First, Reverse and Outline offset.

use super::{Knot, KnotType, Spline, bezier};
use bevy::math::Vec3;

impl Spline {
    /// Splits segment `i` at parameter `t` (de Casteljau), inserting a Bezier
    /// knot that keeps the curve shape. Returns the new knot's index.
    pub fn refine(&mut self, i: usize, t: f32) -> usize {
        let [a, b, c, d] = self.segment(i);
        let line = self.segment_is_line(i);
        let j = (i + 1) % self.knots.len();
        let lerp = |p: Vec3, q: Vec3| p + (q - p) * t;
        let (ab, bc, cd) = (lerp(a, b), lerp(b, c), lerp(c, d));
        let (abc, bcd) = (lerp(ab, bc), lerp(bc, cd));
        let p = bezier([a, b, c, d], t);
        // Freeze the neighbours' tangents so their curves do not change.
        for (k, h_out, h_in) in [(i, Some(ab), None), (j, None, Some(cd))] {
            let (hi, ho) = self.handles(k);
            let kn = &mut self.knots[k];
            if kn.kind == KnotType::Smooth || (kn.kind == KnotType::Bezier && !line) {
                (kn.in_h, kn.out_h) = (hi, ho);
                kn.kind = KnotType::BezierCorner;
            }
            if !line {
                if let Some(h) = h_out {
                    kn.out_h = h;
                }
                if let Some(h) = h_in {
                    kn.in_h = h;
                }
            }
        }
        let knot = if line { Knot::corner(p) } else { Knot::bezier(p, abc, bcd) };
        self.knots.insert(i + 1, knot);
        i + 1
    }

    /// Changes the knot type, seeding Bezier handles from the current curve.
    pub fn set_type(&mut self, i: usize, kind: KnotType) {
        let (hi, ho) = self.handles(i);
        let k = &mut self.knots[i];
        if matches!(kind, KnotType::Bezier | KnotType::BezierCorner) && matches!(k.kind, KnotType::Corner | KnotType::Smooth) {
            let fallback = self.segment_len_hint(i);
            let (hi, ho) = if (ho - hi).length() < 1e-6 { fallback } else { (hi, ho) };
            let k = &mut self.knots[i];
            (k.in_h, k.out_h) = (hi, ho);
        }
        self.knots[i].kind = kind;
    }

    /// Smooth handles a Corner knot would get (used when converting to Bezier).
    fn segment_len_hint(&self, i: usize) -> (Vec3, Vec3) {
        let mut s = self.clone();
        s.knots[i].kind = KnotType::Smooth;
        s.handles(i)
    }

    /// Replaces knot `i` with two knots `dist` along its segments; the new
    /// pair is joined by a straight line (Chamfer) or a quarter-ish arc (Fillet).
    pub fn chamfer_knot(&mut self, i: usize, dist: f32, fillet: bool) -> bool {
        let n = self.knots.len();
        if dist <= 0.0 || n < 2 || (!self.closed && (i == 0 || i == n - 1)) {
            return false;
        }
        let p = self.knots[i].pos;
        let prev = self.knots[(i + n - 1) % n].pos;
        let next = self.knots[(i + 1) % n].pos;
        let (da, db) = ((prev - p).normalize_or_zero(), (next - p).normalize_or_zero());
        let lim = |q: Vec3| dist.min(q.distance(p) * 0.5);
        let (a, b) = (p + da * lim(prev), p + db * lim(next));
        let (mut ka, mut kb) = (Knot::corner(a), Knot::corner(b));
        if fillet {
            let h = 0.552_284_8;
            ka = Knot { kind: KnotType::BezierCorner, out_h: a + (p - a) * h, ..ka };
            kb = Knot { kind: KnotType::BezierCorner, in_h: b + (p - b) * h, ..kb };
        }
        ka.selected = true;
        kb.selected = true;
        self.knots.splice(i..=i, [ka, kb]);
        true
    }

    /// Make First: rotates a closed spline (or reverses an open one when its
    /// last knot is chosen) so knot `i` comes first.
    pub fn make_first(&mut self, i: usize) {
        if self.closed {
            self.knots.rotate_left(i);
        } else if i + 1 == self.knots.len() {
            self.reverse();
        }
    }

    pub fn reverse(&mut self) {
        self.knots.reverse();
        for k in &mut self.knots {
            std::mem::swap(&mut k.in_h, &mut k.out_h);
        }
        if self.closed {
            // keep the first knot first
            self.knots.rotate_right(1);
        }
        // segment flags belonged to the old forward segments
        let n = self.knots.len();
        let flags: Vec<bool> = (0..n).map(|i| self.knots[(i + 1) % n].seg_selected).collect();
        for (k, f) in self.knots.iter_mut().zip(flags) {
            k.seg_selected = f;
        }
    }

    /// Offset copy of the spline in the XY plane (positive = outward for CCW).
    pub fn offset(&self, amount: f32) -> Spline {
        let n = self.knots.len();
        let mut out = self.clone();
        for i in 0..n {
            let p = self.knots[i].pos;
            let prev = if i > 0 { Some(self.knots[i - 1].pos) } else { self.closed.then(|| self.knots[n - 1].pos) };
            let next = if i + 1 < n { Some(self.knots[i + 1].pos) } else { self.closed.then(|| self.knots[0].pos) };
            let side = |a: Vec3, b: Vec3| {
                let d = (b - a).normalize_or_zero();
                Vec3::new(d.y, -d.x, 0.0)
            };
            let off = match (prev, next) {
                (Some(a), Some(b)) => {
                    let (n0, n1) = (side(a, p), side(p, b));
                    let m = (n0 + n1).normalize_or(n0);
                    m / m.dot(n0).max(0.25)
                }
                (Some(a), None) => side(a, p),
                (None, Some(b)) => side(p, b),
                (None, None) => Vec3::ZERO,
            };
            out.knots[i].translate(off * amount);
        }
        out
    }
}
