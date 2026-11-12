//! Settlement planning: oriented lots with separating-axis overlap tests,
//! the outline / bounds containment test, ground height (the village hill),
//! roads that also block building lots, and four-sided facade helpers.

use super::SettlementParams;
use crate::generators::kit::{Kit, inside};
pub use crate::generators::kit::sides;
use bevy::math::{Mat4, Quat, Vec2};
use rand_chacha::ChaCha8Rng;

/// An oriented rectangle in the ground plane; a building's front faces -Y
/// of its local frame (towards the street).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lot {
    pub c: Vec2,
    pub half: Vec2,
    pub yaw: f32,
}

impl Lot {
    pub fn new(c: Vec2, half: Vec2, yaw: f32) -> Self {
        Self { c, half, yaw }
    }
    fn axes(&self) -> [Vec2; 2] {
        let (s, c) = self.yaw.sin_cos();
        [Vec2::new(c, s), Vec2::new(-s, c)]
    }
    pub fn corners(&self) -> [Vec2; 4] {
        let [u, v] = self.axes();
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(a, b)| self.c + u * self.half.x * a + v * self.half.y * b)
    }
    /// Separating-axis overlap test of two oriented rectangles.
    pub fn overlaps(&self, o: &Lot) -> bool {
        let (a, b) = (self.corners(), o.corners());
        self.axes().into_iter().chain(o.axes()).all(|ax| {
            let span = |pts: &[Vec2; 4]| pts.iter().map(|p| p.dot(ax)).fold((f32::MAX, f32::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)));
            let ((a0, a1), (b0, b1)) = (span(&a), span(&b));
            a0 < b1 - 1e-3 && b0 < a1 - 1e-3
        })
    }
    pub fn contains(&self, q: Vec2) -> bool {
        let [u, v] = self.axes();
        let d = q - self.c;
        d.dot(u).abs() <= self.half.x && d.dot(v).abs() <= self.half.y
    }
    /// Local frame of the lot at height `z`.
    pub fn frame(&self, z: f32) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_z(self.yaw), self.c.extend(z))
    }
}

/// Everything an era builder needs: parameters, the kit, the area, the RNG
/// and the lots / roads placed so far.
pub struct Plan {
    pub p: SettlementParams,
    pub kit: Kit,
    pub min: Vec2,
    pub max: Vec2,
    pub outline: Option<Vec<Vec2>>,
    pub rng: ChaCha8Rng,
    pub used: Vec<Lot>,
    /// Village hill: centre, radius, height.
    pub hill: Option<(Vec2, f32, f32)>,
    /// Water (river, sea, moat): no ground, roads or lots.
    pub wet: Vec<Lot>,
    /// Road centrelines and widths (for the street dressing pass).
    pub roads: Vec<(Vec2, Vec2, f32)>,
}

impl Plan {
    pub fn new(p: &SettlementParams, min: Vec2, max: Vec2, outline: Option<Vec<Vec2>>) -> Self {
        let outline = outline.filter(|o| o.len() >= 3);
        Self { p: p.clone(), kit: Kit::new(), min, max, outline, rng: crate::generators::kit::rng(p.seed, p.era as u64), used: vec![], hill: None, wet: vec![], roads: vec![] }
    }

    pub fn contains(&self, q: Vec2) -> bool {
        self.dry(q) && !self.wet.iter().any(|w| w.contains(q))
    }

    /// Inside the area and outline (water or not).
    pub fn dry(&self, q: Vec2) -> bool {
        q.cmpge(self.min).all() && q.cmple(self.max).all() && self.outline.as_ref().is_none_or(|o| inside(o, q))
    }

    /// Ground height (0 except on the village hill).
    pub fn h(&self, q: Vec2) -> f32 {
        hill_height(self.hill, q)
    }

    pub fn fits(&self, l: &Lot) -> bool {
        l.corners().iter().all(|q| self.contains(*q)) && !self.used.iter().any(|u| u.overlaps(l))
    }

    /// Reserves a free lot; false when it is outside or overlaps.
    pub fn claim(&mut self, l: Lot) -> bool {
        let ok = self.fits(&l);
        if ok {
            self.used.push(l);
        }
        ok
    }

    /// Ground grid (Material ID 2) over the area, following the hill.
    pub fn ground(&mut self, cell: f32) {
        let (min, max, hill, outline, wet) = (self.min, self.max, self.hill, self.outline.clone(), self.wet.clone());
        let h = move |q: Vec2| hill_height(hill, q);
        self.kit.mat(crate::generators::kit::ids::LOT);
        self.kit.ground(min, max, cell, h, |q| outline.as_ref().is_none_or(|o| inside(o, q)) && !wet.iter().any(|w| w.contains(q)));
    }

    /// A road segment `w` wide (Material `id`) raised `lift` above the
    /// ground; pieces outside the area are skipped. Blocks building lots.
    pub fn road(&mut self, a: Vec2, b: Vec2, w: f32, lift: f32, id: u16) {
        let d = b - a;
        let len = d.length();
        if len < 1e-3 {
            return;
        }
        let n = (len / 4.0).ceil().max(1.0) as usize;
        let hill = self.hill;
        self.roads.push((a, b, w));
        self.kit.mat(id);
        for i in 0..n {
            let (p, q) = (a + d * (i as f32 / n as f32), a + d * ((i + 1) as f32 / n as f32));
            if !self.contains((p + q) * 0.5) {
                continue;
            }
            self.kit.strip(p, q, w, 4.0, move |x| hill_height(hill, x) + lift);
        }
        self.used.push(Lot::new((a + b) * 0.5, Vec2::new(len * 0.5 + w * 0.5, w * 0.5), d.y.atan2(d.x)));
    }

    /// A road along a polyline.
    pub fn road_path(&mut self, pts: &[Vec2], w: f32, lift: f32, id: u16) {
        for s in pts.windows(2) {
            self.road(s[0], s[1], w, lift, id);
        }
    }
}

/// Smoothstep hill: centre, radius, height.
pub fn hill_height(hill: Option<(Vec2, f32, f32)>, q: Vec2) -> f32 {
    let Some((c, r, h)) = hill else { return 0.0 };
    let t = (1.0 - q.distance(c) / r.max(1e-3)).clamp(0.0, 1.0);
    h * t * t * (3.0 - 2.0 * t)
}
