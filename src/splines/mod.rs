//! Splines & shapes (3ds Max Create -> Shapes, Editable Spline): Bezier
//! knots with Corner / Smooth / Bezier / Bezier Corner tangents, polyline
//! interpolation, the Rendering rollout sweep and spline edit operations.

pub mod edit;
pub mod shapes;
mod spline_ops;
pub mod sweep;
#[cfg(test)]
mod tests;

pub use shapes::{ShapeKind, ShapeParams};
pub use sweep::SplineRender;

use bevy::math::Vec3;

/// 3ds Max vertex (knot) tangent types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum KnotType {
    #[default]
    Corner,
    Smooth,
    Bezier,
    BezierCorner,
}

impl KnotType {
    pub const ALL: [KnotType; 4] = [Self::BezierCorner, Self::Bezier, Self::Corner, Self::Smooth];
    pub fn label(self) -> &'static str {
        match self {
            Self::Corner => "Corner",
            Self::Smooth => "Smooth",
            Self::Bezier => "Bezier",
            Self::BezierCorner => "Bezier Corner",
        }
    }
}

/// A spline vertex. Handles are absolute positions (used by Bezier types).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Knot {
    pub pos: Vec3,
    pub in_h: Vec3,
    pub out_h: Vec3,
    pub kind: KnotType,
    pub selected: bool,
    /// Selection flag of the segment that starts at this knot.
    pub seg_selected: bool,
}

impl Knot {
    pub fn corner(pos: Vec3) -> Self {
        Self { pos, in_h: pos, out_h: pos, kind: KnotType::Corner, selected: false, seg_selected: false }
    }
    pub fn smooth(pos: Vec3) -> Self {
        Self { kind: KnotType::Smooth, ..Self::corner(pos) }
    }
    pub fn bezier(pos: Vec3, in_h: Vec3, out_h: Vec3) -> Self {
        Self { pos, in_h, out_h, kind: KnotType::Bezier, selected: false, seg_selected: false }
    }
    /// Moves the knot together with its handles.
    pub fn translate(&mut self, d: Vec3) {
        self.pos += d;
        self.in_h += d;
        self.out_h += d;
    }
    pub fn transform(&mut self, m: bevy::math::Mat4) {
        self.pos = m.transform_point3(self.pos);
        self.in_h = m.transform_point3(self.in_h);
        self.out_h = m.transform_point3(self.out_h);
    }
}

#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Spline {
    pub knots: Vec<Knot>,
    pub closed: bool,
    pub selected: bool,
}

/// A shape: one or more splines (3ds Max shapes can hold several).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shape {
    pub splines: Vec<Spline>,
    /// Interpolation steps per curved segment.
    pub steps: u32,
}

impl Default for Shape {
    fn default() -> Self {
        Self { splines: vec![], steps: 6 }
    }
}

/// An interpolated spline: points (closed ones do not repeat the first).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Polyline {
    pub pts: Vec<Vec3>,
    pub closed: bool,
}

impl Spline {
    pub fn segment_count(&self) -> usize {
        match self.knots.len() {
            0 | 1 => 0,
            n if self.closed => n,
            n => n - 1,
        }
    }

    /// Effective (in, out) handles of knot `i` after resolving its type.
    pub fn handles(&self, i: usize) -> (Vec3, Vec3) {
        let n = self.knots.len();
        let k = &self.knots[i];
        match k.kind {
            KnotType::Bezier | KnotType::BezierCorner => (k.in_h, k.out_h),
            KnotType::Corner => (k.pos, k.pos),
            KnotType::Smooth => {
                let prev = if i > 0 { Some(self.knots[i - 1].pos) } else { self.closed.then(|| self.knots[n - 1].pos) };
                let next = if i + 1 < n { Some(self.knots[i + 1].pos) } else { self.closed.then(|| self.knots[0].pos) };
                let t = match (prev, next) {
                    (Some(a), Some(b)) => (b - a) / 6.0,
                    (None, Some(b)) => (b - k.pos) / 3.0,
                    (Some(a), None) => (k.pos - a) / 3.0,
                    (None, None) => Vec3::ZERO,
                };
                (k.pos - t, k.pos + t)
            }
        }
    }

    /// Bezier control points of segment `i` (knot i -> knot i+1).
    pub fn segment(&self, i: usize) -> [Vec3; 4] {
        let j = (i + 1) % self.knots.len();
        [self.knots[i].pos, self.handles(i).1, self.handles(j).0, self.knots[j].pos]
    }

    pub fn segment_is_line(&self, i: usize) -> bool {
        let [a, b, c, d] = self.segment(i);
        let dir = (d - a).normalize_or_zero();
        let off = |p: Vec3| (p - a - dir * (p - a).dot(dir)).length();
        off(b) < 1e-6 && off(c) < 1e-6
    }

    pub fn interpolate(&self, steps: u32) -> Polyline {
        let mut pts = vec![];
        if let Some(k) = self.knots.first() {
            pts.push(k.pos);
        }
        for i in 0..self.segment_count() {
            let seg = self.segment(i);
            let n = if self.segment_is_line(i) { 1 } else { steps + 1 };
            for s in 1..=n {
                pts.push(bezier(seg, s as f32 / n as f32));
            }
        }
        if self.closed && pts.len() > 1 {
            pts.pop();
        }
        Polyline { pts, closed: self.closed }
    }
}

/// Cubic Bezier point.
pub fn bezier([a, b, c, d]: [Vec3; 4], t: f32) -> Vec3 {
    let u = 1.0 - t;
    a * (u * u * u) + b * (3.0 * u * u * t) + c * (3.0 * u * t * t) + d * (t * t * t)
}

impl Polyline {
    /// Consecutive point pairs (including the closing one).
    pub fn segments(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        let n = self.pts.len();
        let m = if self.closed && n > 2 { n } else { n.saturating_sub(1) };
        (0..m).map(move |i| (self.pts[i], self.pts[(i + 1) % n]))
    }
    pub fn length(&self) -> f32 {
        self.segments().map(|(a, b)| a.distance(b)).sum()
    }
    /// Signed area in the XY plane (positive = counter-clockwise).
    pub fn signed_area_xy(&self) -> f32 {
        let n = self.pts.len();
        (0..n).map(|i| self.pts[i].truncate().perp_dot(self.pts[(i + 1) % n].truncate())).sum::<f32>() * 0.5
    }
}

impl Shape {
    pub fn polylines(&self) -> Vec<Polyline> {
        self.splines.iter().filter(|s| !s.knots.is_empty()).map(|s| s.interpolate(self.steps)).collect()
    }
    pub fn knot_count(&self) -> usize {
        self.splines.iter().map(|s| s.knots.len()).sum()
    }
    pub fn segment_count(&self) -> usize {
        self.splines.iter().map(|s| s.segment_count()).sum()
    }
    pub fn bounds(&self) -> Option<crate::mesh::Aabb> {
        crate::mesh::Aabb::from_points(self.polylines().into_iter().flat_map(|p| p.pts))
    }
    pub fn transform(&mut self, m: bevy::math::Mat4) {
        self.splines.iter_mut().flat_map(|s| s.knots.iter_mut()).for_each(|k| k.transform(m));
    }
}

/// The base object of a shape: parametric or Editable Spline, plus the
/// Interpolation and Rendering rollouts.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShapeObject {
    pub source: ShapeSource,
    pub steps: u32,
    pub render: SplineRender,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShapeSource {
    Param(ShapeParams),
    Editable(Shape),
}

impl ShapeObject {
    pub fn param(p: ShapeParams) -> Self {
        Self { source: ShapeSource::Param(p), steps: 6, render: SplineRender::default() }
    }
    pub fn editable(s: Shape) -> Self {
        Self { steps: s.steps, source: ShapeSource::Editable(s), render: SplineRender::default() }
    }
    pub fn to_shape(&self) -> Shape {
        let mut s = match &self.source {
            ShapeSource::Param(p) => p.to_shape(),
            ShapeSource::Editable(s) => s.clone(),
        };
        s.steps = self.steps;
        s
    }
    pub fn label(&self) -> &'static str {
        match &self.source {
            ShapeSource::Param(p) => p.kind().label(),
            ShapeSource::Editable(_) => "Editable Spline",
        }
    }
    /// Convert to Editable Spline.
    pub fn make_editable(&mut self) {
        self.source = ShapeSource::Editable(self.to_shape());
    }
    pub fn editable_mut(&mut self) -> Option<&mut Shape> {
        match &mut self.source {
            ShapeSource::Editable(s) => Some(s),
            ShapeSource::Param(_) => None,
        }
    }
    /// Mesh shown in the viewport (Enable In Viewport) or empty.
    pub fn viewport_mesh(&self) -> crate::mesh::EditablePoly {
        if self.render.viewport { self.render.sweep(&self.to_shape()) } else { crate::mesh::EditablePoly::new() }
    }
}
