//! A trail's path: a polyline from the source (index 0) back along where
//! the source has been, measured by arc length.

use bevy::math::Vec3;

pub struct Path {
    pts: Vec<Vec3>,
    /// Arc length at each point.
    cum: Vec<f32>,
}

impl Path {
    /// A path through `pts` (duplicate points dropped); `None` when it has
    /// no length.
    pub fn new(pts: &[Vec3]) -> Option<Self> {
        let mut out: Vec<Vec3> = Vec::with_capacity(pts.len());
        for &p in pts {
            if out.last().is_none_or(|q: &Vec3| q.distance(p) > 1e-4) {
                out.push(p);
            }
        }
        let mut cum = vec![0.0];
        for w in out.windows(2) {
            cum.push(cum.last().unwrap() + w[0].distance(w[1]));
        }
        (out.len() >= 2).then_some(Self { pts: out, cum })
    }

    /// The built-in path: from the origin back along -X over `length`,
    /// turning `turn` degrees (left, about +Z) and rising `climb` metres.
    pub fn arc(length: f32, turn: f32, climb: f32) -> Vec<Vec3> {
        let n = 48;
        let (len, k) = (length.max(0.1), turn.to_radians());
        let mut p = Vec3::ZERO;
        let mut out = vec![p];
        for i in 0..n {
            let a = k * (i as f32 + 0.5) / n as f32;
            let step = len / n as f32;
            p += Vec3::new(-a.cos() * step, -a.sin() * step, climb / n as f32);
            out.push(p);
        }
        out
    }

    pub fn len(&self) -> f32 {
        *self.cum.last().unwrap()
    }

    fn seg(&self, s: f32) -> (usize, f32) {
        let s = s.clamp(0.0, self.len());
        let i = self.cum.partition_point(|&c| c <= s).clamp(1, self.pts.len() - 1) - 1;
        let l = (self.cum[i + 1] - self.cum[i]).max(1e-6);
        (i, (s - self.cum[i]) / l)
    }

    /// Point at arc length `s`.
    pub fn at(&self, s: f32) -> Vec3 {
        let (i, f) = self.seg(s);
        self.pts[i].lerp(self.pts[i + 1], f)
    }

    /// Unit direction of travel away from the source at `s`.
    pub fn dir(&self, s: f32) -> Vec3 {
        let (i, _) = self.seg(s);
        (self.pts[i + 1] - self.pts[i]).normalize_or(Vec3::NEG_X)
    }

    /// Side and up vectors at `s` (up stays near +Z).
    pub fn frame(&self, s: f32) -> (Vec3, Vec3) {
        let t = self.dir(s);
        let up0 = if t.z.abs() > 0.95 { Vec3::X } else { Vec3::Z };
        let side = up0.cross(t).normalize_or(Vec3::Y);
        (side, t.cross(side).normalize_or(Vec3::Z))
    }

    /// Turn rate (radians per metre) around `s`, over a window of `win` m.
    pub fn curvature(&self, s: f32, win: f32) -> f32 {
        let (a, b) = ((s - win).max(0.0), (s + win).min(self.len()));
        if b - a < 1e-3 {
            return 0.0;
        }
        self.dir(a).angle_between(self.dir(b)) / (b - a)
    }
}
