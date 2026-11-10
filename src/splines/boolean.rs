//! 2D spline Boolean (Editable Spline > Spline > Boolean): Union, Subtraction
//! and Intersection of two closed splines in the shape's XY plane. Both
//! splines are refined at their crossings (keeping the curves), every Bezier
//! segment is kept or dropped by whether it lies inside the other spline,
//! and the kept segments are chained back into closed splines.

use super::{Knot, KnotType, Shape, Spline, bezier};
use bevy::math::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SplineBoolOp {
    #[default]
    Union,
    Subtraction,
    Intersection,
}

impl SplineBoolOp {
    pub const ALL: [SplineBoolOp; 3] = [Self::Union, Self::Subtraction, Self::Intersection];
    pub fn label(self) -> &'static str {
        match self {
            Self::Union => "Union",
            Self::Subtraction => "Subtraction",
            Self::Intersection => "Intersection",
        }
    }
    /// Compact button label (3ds Max shows icons here).
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Union => "Un",
            Self::Subtraction => "Sub",
            Self::Intersection => "Int",
        }
    }
}

const SAMPLES: usize = 24;

fn xy(p: Vec3) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// Sampled outline of a spline: (point, segment, t) along each segment.
fn samples(s: &Spline) -> Vec<(Vec2, usize, f32)> {
    let mut out = vec![];
    for i in 0..s.segment_count() {
        let seg = s.segment(i);
        out.extend((0..SAMPLES).map(|k| {
            let t = k as f32 / SAMPLES as f32;
            (xy(bezier(seg, t)), i, t)
        }));
    }
    out
}

fn outline(s: &Spline) -> Vec<Vec2> {
    samples(s).into_iter().map(|x| x.0).collect()
}

fn area(pts: &[Vec2]) -> f32 {
    (0..pts.len()).map(|i| pts[i].perp_dot(pts[(i + 1) % pts.len()])).sum::<f32>() * 0.5
}

fn inside(p: Vec2, poly: &[Vec2]) -> bool {
    let mut c = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

/// Distance from `p` to the outline and the outline direction there.
fn nearest(p: Vec2, poly: &[Vec2]) -> (f32, Vec2) {
    let mut best = (f32::MAX, Vec2::X);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let ab = b - a;
        let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
        let d = (a + ab * t).distance(p);
        if d < best.0 {
            best = (d, ab.normalize_or_zero());
        }
    }
    best
}

/// A crossing: the point and (segment, t) on each spline.
type Crossing = (Vec2, (usize, f32), (usize, f32));

/// Crossings of two sampled outlines.
fn crossings(a: &Spline, b: &Spline) -> Vec<Crossing> {
    let (sa, sb) = (samples(a), samples(b));
    let next = |s: &[(Vec2, usize, f32)], i: usize| {
        let (seg, t) = (s[i].1, s[i].2 + 1.0 / SAMPLES as f32);
        (s[(i + 1) % s.len()].0, seg, t)
    };
    let mut out: Vec<Crossing> = vec![];
    for i in 0..sa.len() {
        let (p0, (p1, _, ta1)) = (sa[i].0, next(&sa, i));
        for j in 0..sb.len() {
            let (q0, (q1, _, tb1)) = (sb[j].0, next(&sb, j));
            let (r, s) = (p1 - p0, q1 - q0);
            let den = r.perp_dot(s);
            if den.abs() < 1e-12 {
                continue;
            }
            let (u, v) = ((q0 - p0).perp_dot(s) / den, (q0 - p0).perp_dot(r) / den);
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let x = p0 + r * u;
            if out.iter().any(|c| c.0.distance(x) < 1e-4) {
                continue;
            }
            let ta = sa[i].2 + (ta1 - sa[i].2) * u;
            let tb = sb[j].2 + (tb1 - sb[j].2) * v;
            out.push((x, (sa[i].1, ta), (sb[j].1, tb)));
        }
    }
    out
}

/// Refines `s` at the given (segment, t, point) cuts and snaps the knots there.
fn cut(s: &mut Spline, mut cuts: Vec<(usize, f32, Vec2)>) {
    // last segment / largest t first, so earlier indices and parameters stay valid
    cuts.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)));
    let mut prev: Option<(usize, f32)> = None;
    for (seg, t0, p) in cuts {
        // a later cut on the same segment shortened it to [0, t_prev]
        let t = prev.filter(|q| q.0 == seg).map_or(t0, |q| t0 / q.1.max(1e-6));
        let k = if t < 1e-3 {
            seg
        } else if t > 1.0 - 1e-3 {
            (seg + 1) % s.knots.len()
        } else {
            s.refine(seg, t)
        };
        if (1e-3..=1.0 - 1e-3).contains(&t) {
            prev = Some((seg, t0));
        }
        let kn = &mut s.knots[k];
        kn.translate(Vec3::new(p.x - kn.pos.x, p.y - kn.pos.y, 0.0));
    }
}

/// Segments of `s` kept by `keep(midpoint, direction)`, as Bezier controls.
fn kept(s: &Spline, keep: impl Fn(Vec2, Vec2) -> bool, flip: bool) -> Vec<[Vec3; 4]> {
    (0..s.segment_count())
        .map(|i| s.segment(i))
        .filter(|seg| {
            let m = xy(bezier(*seg, 0.5));
            let d = xy(bezier(*seg, 0.55) - bezier(*seg, 0.45)).normalize_or_zero();
            keep(m, d)
        })
        .map(|[a, b, c, d]| if flip { [d, c, b, a] } else { [a, b, c, d] })
        .collect()
}

/// Joins pieces head to tail into closed splines.
fn chain(mut pieces: Vec<[Vec3; 4]>, eps: f32) -> Vec<Spline> {
    let mut out = vec![];
    while let Some(first) = pieces.pop() {
        let mut loop_ = vec![first];
        while let Some(j) = pieces.iter().position(|p| p[0].distance(loop_.last().unwrap()[3]) < eps) {
            loop_.push(pieces.swap_remove(j));
        }
        let closed = loop_[0][0].distance(loop_.last().unwrap()[3]) < eps;
        let n = loop_.len();
        let knot = |i: usize| {
            let p = loop_[i];
            let in_h = if i > 0 || closed { loop_[(i + n - 1) % n][2] } else { p[0] };
            let mut k = Knot::bezier(p[0], in_h, p[1]);
            if in_h.distance(p[0]) < 1e-6 && p[1].distance(p[0]) < 1e-6 {
                k.kind = KnotType::Corner;
            } else {
                k.kind = KnotType::BezierCorner;
            }
            k
        };
        let mut knots: Vec<Knot> = (0..n).map(knot).collect();
        if !closed {
            let l = loop_[n - 1];
            knots.push(Knot { kind: KnotType::BezierCorner, ..Knot::bezier(l[3], l[2], l[3]) });
        }
        if knots.len() >= 2 {
            out.push(Spline { knots, closed: closed && n >= 2, selected: true });
        }
    }
    out
}

impl Shape {
    /// Boolean of closed splines `a` and `b`: both are replaced by the result.
    /// Returns false (and changes nothing) when either is open or degenerate.
    pub fn spline_boolean(&mut self, a: usize, b: usize, op: SplineBoolOp) -> bool {
        let ok = |i: usize| self.splines.get(i).is_some_and(|s| s.closed && s.knots.len() >= 2);
        if a == b || !ok(a) || !ok(b) {
            return false;
        }
        let (mut sa, mut sb) = (self.splines[a].clone(), self.splines[b].clone());
        for s in [&mut sa, &mut sb] {
            if area(&outline(s)) < 0.0 {
                s.reverse();
            }
        }
        let size = outline(&sa).iter().chain(outline(&sb).iter()).fold(0.0f32, |m, p| m.max(p.abs().max_element()));
        let eps = (size * 1e-4).max(1e-5);
        let xs = crossings(&sa, &sb);
        cut(&mut sa, xs.iter().map(|x| (x.1.0, x.1.1, x.0)).collect());
        cut(&mut sb, xs.iter().map(|x| (x.2.0, x.2.1, x.0)).collect());
        let (pa, pb) = (outline(&sa), outline(&sb));
        // segment of one outline lying on the other: keep A's once, never B's
        let on = |m: Vec2, d: Vec2, other: &[Vec2]| {
            let (dist, dir) = nearest(m, other);
            (dist < eps * 10.0).then_some(d.dot(dir) > 0.0)
        };
        let side = |m: Vec2, other: &[Vec2]| inside(m, other);
        let mut pieces = match op {
            SplineBoolOp::Union => kept(&sa, |m, d| on(m, d, &pb).unwrap_or(!side(m, &pb)), false),
            SplineBoolOp::Intersection => kept(&sa, |m, d| on(m, d, &pb).unwrap_or(side(m, &pb)), false),
            SplineBoolOp::Subtraction => kept(&sa, |m, d| on(m, d, &pb).map_or(!side(m, &pb), |same| !same), false),
        };
        pieces.extend(match op {
            SplineBoolOp::Union => kept(&sb, |m, d| on(m, d, &pa).is_none() && !side(m, &pa), false),
            SplineBoolOp::Intersection => kept(&sb, |m, d| on(m, d, &pa).is_none() && side(m, &pa), false),
            SplineBoolOp::Subtraction => kept(&sb, |m, d| on(m, d, &pa).is_none() && side(m, &pa), true),
        });
        let result = chain(pieces, eps * 10.0);
        let (hi, lo) = (a.max(b), a.min(b));
        self.splines.remove(hi);
        self.splines.remove(lo);
        self.splines.extend(result);
        true
    }
}
