//! Selection region shapes cycled with `Q` (3ds Max): Rectangular, Circular,
//! Fence (click a polygon) and Lasso (drag a free-hand outline).

use bevy::math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RegionShape {
    #[default]
    Rectangle,
    Circle,
    Fence,
    Lasso,
}

impl RegionShape {
    pub const ALL: [RegionShape; 4] = [Self::Rectangle, Self::Circle, Self::Fence, Self::Lasso];

    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangular Selection Region",
            Self::Circle => "Circular Selection Region",
            Self::Fence => "Fence Selection Region",
            Self::Lasso => "Lasso Selection Region",
        }
    }

    /// The next shape (what `Q` switches to when already in Select mode).
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

/// A screen-space selection region.
#[derive(Clone, Debug, PartialEq)]
pub enum Region {
    Rect { min: Vec2, max: Vec2 },
    Circle { center: Vec2, radius: f32 },
    Polygon(Vec<Vec2>),
}

impl Region {
    /// Rectangle spanned by two corners.
    pub fn rect(a: Vec2, b: Vec2) -> Self {
        Self::Rect { min: a.min(b), max: a.max(b) }
    }

    /// Region of `shape` from the drag / clicked points and the cursor:
    /// Rectangle and Circle use the first point and the cursor; Fence and
    /// Lasso close the outline through every point. `None` if degenerate.
    pub fn build(shape: RegionShape, pts: &[Vec2], cursor: Vec2) -> Option<Self> {
        let first = *pts.first()?;
        let r = match shape {
            RegionShape::Rectangle => Self::rect(first, cursor),
            RegionShape::Circle => Self::Circle { center: first, radius: first.distance(cursor) },
            RegionShape::Fence | RegionShape::Lasso => {
                let mut poly = pts.to_vec();
                if poly.last().is_none_or(|p| p.distance(cursor) > 0.5) {
                    poly.push(cursor);
                }
                Self::Polygon(poly)
            }
        };
        (!r.is_degenerate()).then_some(r)
    }

    fn is_degenerate(&self) -> bool {
        match self {
            Self::Rect { min, max } => (*max - *min).min_element() < 1.0,
            Self::Circle { radius, .. } => *radius < 1.0,
            Self::Polygon(p) => p.len() < 3 || area(p).abs() < 1.0,
        }
    }

    pub fn contains(&self, p: Vec2) -> bool {
        match self {
            Self::Rect { min, max } => p.x >= min.x && p.y >= min.y && p.x <= max.x && p.y <= max.y,
            Self::Circle { center, radius } => p.distance(*center) <= *radius,
            Self::Polygon(poly) => {
                let mut c = false;
                for i in 0..poly.len() {
                    let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                    if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                        c = !c;
                    }
                }
                c
            }
        }
    }
}

fn area(p: &[Vec2]) -> f32 {
    (0..p.len()).map(|i| p[i].perp_dot(p[(i + 1) % p.len()])).sum::<f32>() * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_cycle_and_contain() {
        let seen: Vec<_> = std::iter::successors(Some(RegionShape::Rectangle), |s| Some(s.next())).take(5).collect();
        assert_eq!(seen, [RegionShape::Rectangle, RegionShape::Circle, RegionShape::Fence, RegionShape::Lasso, RegionShape::Rectangle]);
        let o = Vec2::ZERO;
        let r = Region::build(RegionShape::Rectangle, &[Vec2::new(10.0, 10.0)], o).unwrap();
        assert!(r.contains(Vec2::new(5.0, 5.0)) && !r.contains(Vec2::new(11.0, 5.0)));
        let c = Region::build(RegionShape::Circle, &[o], Vec2::new(10.0, 0.0)).unwrap();
        assert!(c.contains(Vec2::new(6.0, 6.0)) && !c.contains(Vec2::new(8.0, 8.0)), "corner of the box is outside the circle");
        // An L-shaped lasso excludes its notch.
        let l = [(0., 0.), (20., 0.), (20., 10.), (10., 10.), (10., 20.)].map(|(x, y)| Vec2::new(x, y));
        let lasso = Region::build(RegionShape::Lasso, &l, Vec2::new(0.0, 20.0)).unwrap();
        assert!(lasso.contains(Vec2::new(5.0, 15.0)) && lasso.contains(Vec2::new(15.0, 5.0)));
        assert!(!lasso.contains(Vec2::new(15.0, 15.0)));
        // Fence: the closing point is the cursor; a line is degenerate.
        let fence = Region::build(RegionShape::Fence, &[o, Vec2::new(10.0, 0.0)], Vec2::new(0.0, 10.0)).unwrap();
        assert!(fence.contains(Vec2::new(2.0, 2.0)) && !fence.contains(Vec2::new(8.0, 8.0)));
        assert!(Region::build(RegionShape::Fence, &[o, Vec2::new(10.0, 0.0)], Vec2::new(20.0, 0.0)).is_none());
        assert!(Region::build(RegionShape::Circle, &[o], o).is_none());
    }
}
