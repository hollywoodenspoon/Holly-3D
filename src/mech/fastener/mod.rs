//! Parametric fasteners (Create -> Geometry -> Mech), after SolidCraft's
//! fastener generators (MIT / Apache-2.0): hex bolts, hex nuts and blocks
//! with a tapped (threaded) hole, in ISO metric or UNC inch sizes, with real
//! 60° helical threads.
//!
//! Each part is one closed lathe-like grid: every column is one angle
//! around the axis and runs down the part's profile; the thread is the
//! profile radius changing with height and angle along the helix.

pub mod table;

use crate::mesh::EditablePoly;
use crate::splines::sweep::{flip, loft};
use bevy::math::Vec3;
use std::f32::consts::{PI, TAU};
pub use table::{IMPERIAL, METRIC, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FastenerKind {
    Bolt,
    Nut,
    /// A square block with a threaded hole straight through.
    TappedHole,
}

impl FastenerKind {
    pub fn label(self) -> &'static str {
        match self {
            FastenerKind::Bolt => "Hex Bolt",
            FastenerKind::Nut => "Hex Nut",
            FastenerKind::TappedHole => "Threaded Hole Block",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThreadStd {
    #[default]
    Metric,
    Imperial,
}

impl ThreadStd {
    pub fn sizes(self) -> &'static [Size] {
        match self {
            ThreadStd::Metric => &METRIC,
            ThreadStd::Imperial => &IMPERIAL,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ThreadStd::Metric => "ISO Metric",
            ThreadStd::Imperial => "Imperial (UNC)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Fastener {
    pub kind: FastenerKind,
    pub standard: ThreadStd,
    /// Index into the standard's size table.
    pub size: usize,
    /// Bolt: shank length under the head. Hole block: block height (mm).
    pub length: f32,
    /// Bolt: threaded length from the tip (mm).
    pub thread_length: f32,
    /// Hole block: width of the square block (mm).
    pub block: f32,
    /// Off: plain cylinders (light geometry).
    pub threads: bool,
    /// Columns around the axis (a multiple of 12 keeps hex and square corners).
    pub sides: u32,
    /// Rows per thread turn.
    pub detail: u32,
    /// Scene units per real millimetre x 1000 (1 = true size in metres).
    pub scale: f32,
}

impl Default for Fastener {
    fn default() -> Self {
        Self::new(FastenerKind::Bolt)
    }
}

impl Fastener {
    pub fn new(kind: FastenerKind) -> Self {
        Self { kind, standard: ThreadStd::Metric, size: 5, length: 40.0, thread_length: 26.0, block: 30.0, threads: true, sides: 48, detail: 8, scale: 10.0 }
    }

    pub fn spec(&self) -> Size {
        let t = self.standard.sizes();
        t[self.size.min(t.len() - 1)]
    }

    /// Thread depth (external) for the pitch: 5/8 of the ISO basic H.
    pub fn depth(&self) -> f32 {
        if self.threads { 0.866 * self.spec().pitch * 0.625 } else { 0.0 }
    }

    pub fn build(&self) -> EditablePoly {
        let s = self.spec();
        let sides = (self.sides.clamp(12, 192) / 12 * 12) as usize;
        let angles: Vec<f32> = (0..sides).map(|j| j as f32 / sides as f32 * TAU).collect();
        let cols: Vec<Vec<Vec3>> = angles.iter().map(|&a| self.column(&s, a)).collect();
        // Transpose: loft rows are rings around the axis.
        let rows: Vec<Vec<Vec3>> = (0..cols[0].len()).map(|r| cols.iter().map(|c| c[r]).collect()).collect();
        let closed = self.kind != FastenerKind::Bolt;
        let (mut m, _) = loft(&rows, true, closed, !closed);
        if m.signed_volume() < 0.0 {
            flip(&mut m);
        }
        let k = 0.001 * self.scale.max(1e-4);
        for v in m.vertices.values_mut() {
            v.position *= k;
        }
        m.auto_smooth(&[], 40.0);
        m.finalize();
        m
    }

    /// Height steps through a threaded zone `z0..z1`.
    fn steps(&self, s: &Size, z0: f32, z1: f32) -> Vec<f32> {
        let per = if self.threads { s.pitch / self.detail.clamp(3, 24) as f32 } else { (z1 - z0).max(1e-3) };
        let n = (((z1 - z0) / per).ceil() as usize).clamp(1, 800);
        (0..=n).map(|i| z0 + (z1 - z0) * i as f32 / n as f32).collect()
    }

    /// Thread radius offset (0 at the crest, 1 at the root) at height `z`, angle `a`.
    fn groove(s: &Size, z: f32, a: f32) -> f32 {
        let phase = (z / s.pitch - a / TAU).rem_euclid(1.0);
        let v = (2.0 * phase - 1.0).abs();
        // 60° flanks with small flats at crest and root.
        ((1.0 - v) * 1.25 - 0.1).clamp(0.0, 1.0)
    }

    /// One profile column at angle `a` (mm, axis = +Z, pivot at the base).
    fn column(&self, s: &Size, a: f32) -> Vec<Vec3> {
        let dir = Vec3::new(a.cos(), a.sin(), 0.0);
        let at = |r: f32, z: f32| dir * r + Vec3::Z * z;
        let hex = (s.flats * 0.5) / ((a.rem_euclid(PI / 3.0)) - PI / 6.0).cos();
        let depth = self.depth();
        let r_major = s.d * 0.5;
        match self.kind {
            FastenerKind::Bolt => {
                let (k, len) = (s.head, self.length.max(s.pitch * 2.0));
                let tl = self.thread_length.clamp(s.pitch, len);
                let top = k + len;
                let mut c = vec![at(0.0, 0.0), at(hex * 0.94, 0.0), at(hex, k * 0.12), at(hex, k), at(r_major, k)];
                let z_thread = top - tl;
                if z_thread > k + 1e-3 {
                    c.push(at(r_major, z_thread));
                }
                let chamfer = s.pitch.min(tl * 0.5);
                for z in self.steps(s, z_thread.max(k), top).into_iter().skip(1) {
                    // Run-out into the shank and a chamfered tip.
                    let run = ((z - z_thread) / s.pitch).clamp(0.0, 1.0);
                    let tip = ((top - z) / chamfer).clamp(0.0, 1.0);
                    let r = r_major - depth * Self::groove(s, z, a) * run;
                    let r = r.min(r_major - depth + depth * tip);
                    c.push(at(r, z));
                }
                c.push(at(0.0, top));
                c
            }
            FastenerKind::Nut | FastenerKind::TappedHole => {
                let (h, outer) = if self.kind == FastenerKind::Nut {
                    (s.nut, hex)
                } else {
                    let half = self.block.max(s.d * 1.5) * 0.5;
                    (self.length.max(s.pitch * 2.0), half / a.cos().abs().max(a.sin().abs()))
                };
                // Internal thread: the hole's crest is the bolt's root.
                let r_minor = r_major - depth.max(s.pitch * 0.05);
                let mut c = vec![at(outer, 0.0), at(outer, h)];
                for z in self.steps(s, 0.0, h).into_iter().rev() {
                    c.push(at(r_minor + depth * Self::groove(s, z, a), z));
                }
                c
            }
        }
    }
}
