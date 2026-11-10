//! Hose / Mecha Cable: a tube along a Bezier or Catmull-Rom path between
//! two ends, with Tension, Slack and Gravity Sag, plus optional corrugated
//! rings (round or square). Ends are free points in the hose's own space,
//! or bound to two objects (written each frame by `Scene::update_hoses`).

use crate::mesh::{EditablePoly, VertexId};
use crate::scene::ObjId;
use bevy::math::{Quat, Vec2, Vec3};
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HoseCurve {
    #[default]
    Bezier,
    CatmullRom,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hose {
    /// Ends in the hose's local space, each with the direction the hose
    /// travels there (out of the start, into the end).
    pub start: Vec3,
    pub start_dir: Vec3,
    pub end: Vec3,
    pub end_dir: Vec3,
    /// Bound ends: the hose leaves the top object down its -Z axis and
    /// enters the bottom object along its -Z axis (up its +Z).
    pub top: Option<ObjId>,
    pub bottom: Option<ObjId>,
    pub curve: HoseCurve,
    pub tension: f32,
    pub slack: f32,
    pub sag: f32,
    pub diameter: f32,
    pub sides: u32,
    pub segments: u32,
    pub rings: u32,
    pub ring_depth: f32,
    pub square_rings: bool,
}

impl Default for Hose {
    fn default() -> Self {
        Self {
            start: Vec3::new(0.0, 0.0, 1.5),
            start_dir: Vec3::X,
            end: Vec3::new(2.0, 0.0, 1.5),
            end_dir: Vec3::X,
            top: None,
            bottom: None,
            curve: HoseCurve::Bezier,
            tension: 0.5,
            slack: 0.0,
            sag: 0.4,
            diameter: 0.12,
            sides: 12,
            segments: 32,
            rings: 0,
            ring_depth: 0.015,
            square_rings: false,
        }
    }
}

fn bezier(p: [Vec3; 4], t: f32) -> Vec3 {
    let u = 1.0 - t;
    p[0] * u * u * u + p[1] * 3.0 * u * u * t + p[2] * 3.0 * u * t * t + p[3] * t * t * t
}

fn catmull(p: [Vec3; 4], t: f32) -> Vec3 {
    let (t2, t3) = (t * t, t * t * t);
    ((p[1] * 2.0) + (p[2] - p[0]) * t + (p[0] * 2.0 - p[1] * 5.0 + p[2] * 4.0 - p[3]) * t2 + (p[1] * 3.0 - p[0] - p[2] * 3.0 + p[3]) * t3)
        * 0.5
}

impl Hose {
    /// The hose's centerline at `t` in 0..=1.
    pub fn point(&self, t: f32) -> Vec3 {
        let (a, b) = (self.start, self.end);
        let len = a.distance(b).max(1e-4);
        let (da, db) = (self.start_dir.normalize_or(Vec3::X), self.end_dir.normalize_or(Vec3::X));
        let hand = len * self.tension.clamp(0.0, 2.0) * 0.5;
        // Gravity Sag plus Slack (extra length hangs lower), deepest mid-way.
        let droop = self.sag.max(0.0) + self.slack.max(0.0) * len * 0.5;
        match self.curve {
            HoseCurve::Bezier => {
                let bend = bezier([a, a + da * hand, b - db * hand, b], t);
                bend - Vec3::Z * droop * 4.0 * t * (1.0 - t)
            }
            HoseCurve::CatmullRom => {
                // Through start, the sagging middle and end; the phantom
                // points follow the end directions.
                let mid = (a + b) * 0.5 - Vec3::Z * droop + (da - db) * hand * 0.25;
                let pts = [a - da * hand.max(1e-3), a, mid, b, b + db * hand.max(1e-3)];
                let (seg, s) = if t < 0.5 { (0, t * 2.0) } else { (1, t * 2.0 - 1.0) };
                catmull([pts[seg], pts[seg + 1], pts[seg + 2], pts[seg + 3]], s)
            }
        }
    }

    /// Tube radius at `u` along the hose (corrugation applied).
    fn radius(&self, u: f32, ring_low: Option<bool>) -> f32 {
        let r = self.diameter.max(1e-4) * 0.5;
        let d = self.ring_depth.clamp(0.0, r * 0.9);
        match (self.rings, ring_low) {
            (0, _) => r,
            (_, Some(low)) => r - if low { d } else { 0.0 },
            (n, None) => r - d * (0.5 - 0.5 * (TAU * n as f32 * u).cos()),
        }
    }

    /// Ring stations (u, radius). Square rings get two stations at each
    /// step so the walls are crisp.
    fn stations(&self) -> Vec<(f32, f32)> {
        let segs = self.segments.clamp(2, 512) as usize;
        let n = if self.ring_depth > 1e-5 { self.rings.min(200) as usize } else { 0 };
        if n == 0 || !self.square_rings {
            let count = if n == 0 { segs } else { segs.max(n * 8) };
            return (0..=count).map(|i| i as f32 / count as f32).map(|u| (u, self.radius(u, None))).collect();
        }
        let steps = 2 * n;
        let sub = (segs / steps).max(1);
        let mut out = vec![];
        for k in 0..steps {
            let r = self.radius(0.0, Some(k % 2 == 1));
            for s in 0..=sub {
                out.push(((k * sub + s) as f32 / (steps * sub) as f32, r));
            }
        }
        out
    }

    pub fn build(&self) -> EditablePoly {
        let st = self.stations();
        let sides = self.sides.clamp(3, 64) as usize;
        let pts: Vec<Vec3> = st.iter().map(|&(u, _)| self.point(u)).collect();
        // Parallel-transported frames along the path (no twisting).
        let tangent = |i: usize| {
            let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(pts.len() - 1)]);
            (b - a).normalize_or(self.start_dir.normalize_or(Vec3::X))
        };
        let mut t0 = tangent(0);
        let mut side = t0.any_orthonormal_vector();
        let mut m = EditablePoly::new();
        let mut rings: Vec<Vec<VertexId>> = vec![];
        for (i, &(_, r)) in st.iter().enumerate() {
            let t = tangent(i);
            side = (Quat::from_rotation_arc(t0, t) * side).normalize();
            t0 = t;
            let up = t.cross(side);
            rings.push(
                (0..sides)
                    .map(|k| {
                        let a = TAU * k as f32 / sides as f32;
                        m.add_vertex(pts[i] + (side * a.cos() + up * a.sin()) * r)
                    })
                    .collect(),
            );
        }
        let n = rings.len();
        for i in 0..n - 1 {
            for k in 0..sides {
                let k1 = (k + 1) % sides;
                let vs = [rings[i][k], rings[i][k1], rings[i + 1][k1], rings[i + 1][k]];
                let (u0, u1) = (k as f32 / sides as f32, (k + 1) as f32 / sides as f32);
                let (v0, v1) = (st[i].0, st[i + 1].0);
                let p = m.add_polygon_uv(&vs, &[Vec2::new(u0, v0), Vec2::new(u1, v0), Vec2::new(u1, v1), Vec2::new(u0, v1)]);
                m.polygons[p].smoothing_group = 1;
            }
        }
        // End caps.
        let disc = |k: usize| {
            let a = TAU * k as f32 / sides as f32;
            Vec2::new(a.cos(), a.sin()) * 0.5 + Vec2::splat(0.5)
        };
        let mut cap: Vec<usize> = (0..sides).rev().collect();
        for ring in [0, n - 1] {
            let vs: Vec<VertexId> = cap.iter().map(|&k| rings[ring][k]).collect();
            let uv: Vec<Vec2> = cap.iter().map(|&k| disc(k)).collect();
            m.add_polygon_uv(&vs, &uv);
            cap.reverse();
        }
        m.rebuild_edges();
        m
    }
}
