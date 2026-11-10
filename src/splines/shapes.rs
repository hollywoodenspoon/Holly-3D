//! Parametric shapes (Create -> Shapes): Circle, Arc, NGon, Rectangle,
//! Ellipse, Star and Helix, all built in the local XY plane around the pivot.

use super::{Knot, KnotType, Shape, Spline};
use bevy::math::Vec3;
use std::f32::consts::{FRAC_PI_2, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShapeKind {
    Line,
    Circle,
    Arc,
    NGon,
    Rectangle,
    Ellipse,
    Star,
    Helix,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 8] =
        [Self::Line, Self::Circle, Self::Arc, Self::NGon, Self::Rectangle, Self::Ellipse, Self::Star, Self::Helix];
    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Circle => "Circle",
            Self::Arc => "Arc",
            Self::NGon => "NGon",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Star => "Star",
            Self::Helix => "Helix",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ShapeParams {
    Circle {
        radius: f32,
    },
    /// Angles in degrees; `pie` closes the arc through the center.
    Arc {
        radius: f32,
        from: f32,
        to: f32,
        pie: bool,
    },
    NGon {
        radius: f32,
        sides: u32,
        circular: bool,
    },
    Rectangle {
        length: f32,
        width: f32,
        corner_radius: f32,
    },
    Ellipse {
        length: f32,
        width: f32,
    },
    Star {
        radius1: f32,
        radius2: f32,
        points: u32,
        distortion: f32,
    },
    Helix {
        radius1: f32,
        radius2: f32,
        height: f32,
        turns: f32,
    },
}

impl ShapeParams {
    /// Defaults for a click-placed shape (`Line` has no parameters).
    pub fn default_of(kind: ShapeKind) -> Option<Self> {
        Some(match kind {
            ShapeKind::Line => return None,
            ShapeKind::Circle => Self::Circle { radius: 0.6 },
            ShapeKind::Arc => Self::Arc { radius: 0.6, from: 0.0, to: 180.0, pie: false },
            ShapeKind::NGon => Self::NGon { radius: 0.6, sides: 6, circular: false },
            ShapeKind::Rectangle => Self::Rectangle { length: 1.0, width: 1.4, corner_radius: 0.0 },
            ShapeKind::Ellipse => Self::Ellipse { length: 0.8, width: 1.4 },
            ShapeKind::Star => Self::Star { radius1: 0.7, radius2: 0.35, points: 6, distortion: 0.0 },
            ShapeKind::Helix => Self::Helix { radius1: 0.5, radius2: 0.5, height: 1.2, turns: 3.0 },
        })
    }

    pub fn kind(&self) -> ShapeKind {
        match self {
            Self::Circle { .. } => ShapeKind::Circle,
            Self::Arc { .. } => ShapeKind::Arc,
            Self::NGon { .. } => ShapeKind::NGon,
            Self::Rectangle { .. } => ShapeKind::Rectangle,
            Self::Ellipse { .. } => ShapeKind::Ellipse,
            Self::Star { .. } => ShapeKind::Star,
            Self::Helix { .. } => ShapeKind::Helix,
        }
    }

    pub fn to_shape(&self) -> Shape {
        let spline = match *self {
            Self::Circle { radius } => ellipse(radius, radius, 0.0, TAU, true),
            Self::Ellipse { length, width } => ellipse(width * 0.5, length * 0.5, 0.0, TAU, true),
            Self::Arc { radius, from, to, pie } => {
                let (a, b) = (from.to_radians(), to.to_radians());
                let span = (b - a).rem_euclid(TAU);
                let mut s = ellipse(radius, radius, a, if span < 1e-4 { TAU } else { span }, false);
                if pie {
                    s.knots.push(Knot::corner(Vec3::ZERO));
                    s.closed = true;
                    let n = s.knots.len();
                    // straight edges into / out of the center
                    s.knots[0].kind = KnotType::BezierCorner;
                    s.knots[0].in_h = s.knots[0].pos;
                    s.knots[n - 2].kind = KnotType::BezierCorner;
                    s.knots[n - 2].out_h = s.knots[n - 2].pos;
                }
                s
            }
            Self::NGon { radius, sides, circular } => {
                let n = sides.max(3);
                if circular {
                    ellipse(radius, radius, 0.0, TAU, true)
                } else {
                    let knots = (0..n).map(|i| Knot::corner(polar(radius, i as f32 / n as f32 * TAU + FRAC_PI_2))).collect();
                    Spline { knots, closed: true, selected: false }
                }
            }
            Self::Rectangle { length, width, corner_radius } => rectangle(width * 0.5, length * 0.5, corner_radius),
            Self::Star { radius1, radius2, points, distortion } => {
                let n = points.max(3);
                let knots = (0..n * 2)
                    .map(|i| {
                        let mut a = i as f32 / (n * 2) as f32 * TAU + FRAC_PI_2;
                        if i % 2 == 1 {
                            a += distortion.to_radians();
                        }
                        Knot::corner(polar(if i % 2 == 0 { radius1 } else { radius2 }, a))
                    })
                    .collect();
                Spline { knots, closed: true, selected: false }
            }
            Self::Helix { radius1, radius2, height, turns } => {
                let n = ((turns.abs() * 8.0).ceil() as usize).max(2);
                let knots = (0..=n)
                    .map(|i| {
                        let t = i as f32 / n as f32;
                        let r = radius1 + (radius2 - radius1) * t;
                        polar(r, t * turns * TAU) + Vec3::Z * height * t
                    })
                    .map(Knot::smooth)
                    .collect();
                Spline { knots, closed: false, selected: false }
            }
        };
        Shape { splines: vec![spline], steps: 6 }
    }
}

fn polar(r: f32, a: f32) -> Vec3 {
    Vec3::new(r * a.cos(), r * a.sin(), 0.0)
}

/// Elliptical arc from angle `a0` spanning `span` radians, as Bezier knots
/// (one segment per quarter turn at most).
fn ellipse(rx: f32, ry: f32, a0: f32, span: f32, closed: bool) -> Spline {
    let pieces = ((span / FRAC_PI_2 - 1e-4).ceil() as usize).max(1);
    let step = span / pieces as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let count = if closed { pieces } else { pieces + 1 };
    let knots = (0..count)
        .map(|i| {
            let a = a0 + step * i as f32;
            let p = Vec3::new(rx * a.cos(), ry * a.sin(), 0.0);
            let t = Vec3::new(-rx * a.sin(), ry * a.cos(), 0.0) * k;
            Knot::bezier(p, p - t, p + t)
        })
        .collect();
    Spline { knots, closed, selected: false }
}

fn rectangle(hx: f32, hy: f32, r: f32) -> Spline {
    let r = r.clamp(0.0, hx.min(hy));
    if r <= 1e-5 {
        let knots = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)].map(|(x, y)| Knot::corner(Vec3::new(x, y, 0.0))).to_vec();
        return Spline { knots, closed: true, selected: false };
    }
    // Two Bezier Corner knots per rounded corner (quarter arc between them).
    let k = r * 0.552_284_8;
    let mut knots = vec![];
    for (cx, cy, a) in
        [(hx - r, -hy + r, -FRAC_PI_2), (hx - r, hy - r, 0.0), (-hx + r, hy - r, FRAC_PI_2), (-hx + r, -hy + r, std::f32::consts::PI)]
    {
        let c = Vec3::new(cx, cy, 0.0);
        let (p0, p1) = (c + polar(r, a), c + polar(r, a + FRAC_PI_2));
        let (t0, t1) = (polar(1.0, a + FRAC_PI_2), polar(1.0, a + std::f32::consts::PI));
        let mut k0 = Knot::bezier(p0, p0, p0 + t0 * k);
        k0.kind = KnotType::BezierCorner;
        let mut k1 = Knot::bezier(p1, p1 - t1 * k, p1);
        k1.kind = KnotType::BezierCorner;
        knots.push(k0);
        knots.push(k1);
    }
    Spline { knots, closed: true, selected: false }
}
