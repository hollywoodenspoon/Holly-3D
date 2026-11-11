//! Loft: skins two or more cross-section shapes along a path spline (the
//! object the modifier sits on), with twist, a scale curve and caps. Each
//! section sits at a percentage of the path; between sections the profiles
//! blend point by point, after being resampled to the same count and
//! lined up so the skin does not corkscrew.

use crate::mesh::{EditablePoly, VertexId};
use crate::splines::sweep::{cap, flip, loft};
use crate::splines::{Polyline, Shape};
use bevy::math::{Quat, Vec2, Vec3};

/// One cross-section, in the picked shape's own XY plane (about its pivot).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LoftSection {
    pub name: String,
    /// Where on the path, 0..=100 %.
    pub at: f32,
    pub points: Vec<Vec2>,
    pub closed: bool,
}

impl LoftSection {
    /// The first spline of `pl`, flattened onto its XY plane.
    pub fn from_polyline(name: impl Into<String>, at: f32, pl: &Polyline) -> Self {
        Self { name: name.into(), at, points: pl.pts.iter().map(|p| p.truncate()).collect(), closed: pl.closed }
    }

    /// A circle (the stand-in until a shape is picked).
    pub fn circle(r: f32, n: usize) -> Self {
        let points = (0..n).map(|i| Vec2::from_angle(i as f32 / n as f32 * std::f32::consts::TAU) * r).collect();
        Self { name: "Circle".into(), at: 0.0, points, closed: true }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LoftMod {
    pub sections: Vec<LoftSection>,
    /// Rows along the path.
    pub path_steps: u32,
    /// Points around every section.
    pub shape_steps: u32,
    /// Total twist from start to end, degrees.
    pub twist: f32,
    /// Section scale at the start and end of the path, %.
    pub scale_start: f32,
    pub scale_end: f32,
    /// Scale curve shape: 1 = linear, below eases out, above eases in.
    pub scale_curve: f32,
    pub cap_start: bool,
    pub cap_end: bool,
    pub flip: bool,
    /// Highlighted section row.
    pub selected: Option<usize>,
    /// Path % given to the next picked shape.
    pub next_at: f32,
}

impl Default for LoftMod {
    fn default() -> Self {
        Self {
            sections: vec![],
            path_steps: 48,
            shape_steps: 32,
            twist: 0.0,
            scale_start: 100.0,
            scale_end: 100.0,
            scale_curve: 1.0,
            cap_start: true,
            cap_end: true,
            flip: false,
            selected: None,
            next_at: 100.0,
        }
    }
}

/// Resamples `pts` to `n` points evenly by length.
pub fn resample(pts: &[Vec3], closed: bool, n: usize) -> Vec<Vec3> {
    let p = pts;
    if p.len() < 2 || n < 2 {
        return pts.to_vec();
    }
    let mut ring = p.to_vec();
    if closed {
        ring.push(p[0]);
    }
    let mut acc = vec![0.0];
    for w in ring.windows(2) {
        acc.push(acc.last().unwrap() + w[0].distance(w[1]));
    }
    let total = *acc.last().unwrap();
    if total < 1e-9 {
        return vec![pts[0]; n];
    }
    let count = if closed { n } else { n - 1 };
    let mut seg = 0;
    (0..n)
        .map(|i| {
            let d = total * i as f32 / count as f32;
            while seg + 2 < acc.len() && acc[seg + 1] < d {
                seg += 1;
            }
            let t = ((d - acc[seg]) / (acc[seg + 1] - acc[seg]).max(1e-9)).clamp(0.0, 1.0);
            ring[seg].lerp(ring[seg + 1], t)
        })
        .collect()
}

fn signed_area(p: &[Vec2]) -> f32 {
    (0..p.len()).map(|i| p[i].perp_dot(p[(i + 1) % p.len()])).sum::<f32>() * 0.5
}

/// Lines `b` up with `a`: same winding and the start rotated to the best match.
fn align(a: &[Vec2], b: &mut [Vec2], closed: bool) {
    if !closed || a.len() != b.len() {
        return;
    }
    if signed_area(a).signum() != signed_area(b).signum() {
        b.reverse();
    }
    let n = b.len();
    let cost = |k: usize| (0..n).map(|i| a[i].distance_squared(b[(i + k) % n])).sum::<f32>();
    let best = (0..n).min_by(|&x, &y| cost(x).total_cmp(&cost(y))).unwrap_or(0);
    b.rotate_left(best);
}

/// Path frames (point, tangent, normal) by parallel transport.
fn frames(path: &[Vec3], closed: bool) -> Vec<(Vec3, Vec3, Vec3)> {
    let n = path.len();
    let tan = |i: usize| {
        let (a, b) = match (i, closed) {
            (0, false) => (path[0], path[1]),
            (i, false) if i + 1 == n => (path[n - 2], path[n - 1]),
            _ => (path[(i + n - 1) % n], path[(i + 1) % n]),
        };
        (b - a).normalize_or(Vec3::Z)
    };
    let t0 = tan(0);
    let up = if t0.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let mut normal = (up - t0 * up.dot(t0)).normalize();
    let mut prev = t0;
    (0..n)
        .map(|i| {
            let t = tan(i);
            normal = Quat::from_rotation_arc(prev, t) * normal;
            normal = (normal - t * normal.dot(t)).normalize_or(normal);
            prev = t;
            (path[i], t, normal)
        })
        .collect()
}

impl LoftMod {
    /// Adds a section, keeping them ordered along the path.
    pub fn add_section(&mut self, s: LoftSection) {
        self.sections.push(s);
        self.sections.sort_by(|a, b| a.at.total_cmp(&b.at));
    }

    /// Section scale factor at path fraction `t`.
    pub fn scale_at(&self, t: f32) -> f32 {
        let k = t.clamp(0.0, 1.0).powf(self.scale_curve.clamp(0.1, 10.0));
        (self.scale_start + (self.scale_end - self.scale_start) * k) / 100.0
    }

    /// The blended section at path fraction `t` (resampled to `m` points).
    fn profile_at(&self, t: f32, sections: &[(f32, Vec<Vec2>)]) -> Vec<Vec2> {
        let pct = t * 100.0;
        let i = sections.iter().position(|(at, _)| *at >= pct).unwrap_or(sections.len());
        match i {
            0 => sections[0].1.clone(),
            i if i == sections.len() => sections[i - 1].1.clone(),
            i => {
                let ((a0, p0), (a1, p1)) = (&sections[i - 1], &sections[i]);
                let k = ((pct - a0) / (a1 - a0).max(1e-6)).clamp(0.0, 1.0);
                let k = k * k * (3.0 - 2.0 * k);
                p0.iter().zip(p1).map(|(a, b)| a.lerp(*b, k)).collect()
            }
        }
    }

    pub fn build(&self, shape: &Shape) -> EditablePoly {
        let Some(path) = shape.polylines().into_iter().find(|p| p.pts.len() >= 2) else { return EditablePoly::new() };
        let defaults = [LoftSection::circle(0.2, 16)];
        let src: &[LoftSection] = if self.sections.is_empty() { &defaults } else { &self.sections };
        let m = self.shape_steps.clamp(3, 512) as usize;
        let closed = src.iter().all(|s| s.closed);
        let mut sections: Vec<(f32, Vec<Vec2>)> = vec![];
        for s in src.iter().filter(|s| s.points.len() >= 2) {
            let pts: Vec<Vec3> = s.points.iter().map(|p| p.extend(0.0)).collect();
            let mut r: Vec<Vec2> = resample(&pts, closed, m).into_iter().map(|p| p.truncate()).collect();
            if let Some((_, prev)) = sections.last() {
                align(prev, &mut r, closed);
            }
            sections.push((s.at.clamp(0.0, 100.0), r));
        }
        if sections.is_empty() {
            return EditablePoly::new();
        }
        let rows_n = self.path_steps.clamp(2, 1024) as usize;
        let samples = resample(&path.pts, path.closed, if path.closed { rows_n } else { rows_n + 1 });
        let fr = frames(&samples, path.closed);
        let count = (fr.len() - usize::from(!path.closed)).max(1) as f32;
        let rows: Vec<Vec<Vec3>> = fr
            .iter()
            .enumerate()
            .map(|(i, &(p, t, n))| {
                let f = i as f32 / count;
                let rot = Vec2::from_angle(self.twist.to_radians() * f);
                let s = self.scale_at(f);
                let b = t.cross(n);
                self.profile_at(f, &sections).into_iter().map(|q| {
                    let q = rot.rotate(q) * s;
                    p + n * q.x + b * q.y
                }).collect()
            })
            .collect();
        let (mut mesh, ids) = loft(&rows, closed, path.closed, false);
        if closed && !path.closed {
            if self.cap_start {
                let first: Vec<VertexId> = ids[0].iter().rev().copied().collect();
                cap(&mut mesh, &first);
            }
            if self.cap_end {
                cap(&mut mesh, ids.last().unwrap());
            }
            mesh.finalize();
        }
        let solid = closed && (path.closed || (self.cap_start && self.cap_end));
        if (solid && mesh.signed_volume() < 0.0) != self.flip {
            flip(&mut mesh);
            mesh.finalize();
        }
        mesh
    }
}
