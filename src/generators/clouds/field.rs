//! Cloud Field: scatters a mix of cloud types across a layer of sky with
//! blue-noise spacing (dart throwing), so a whole sky is one click.

use super::{CloudParams, CloudType};
use crate::generators::{hash, kit::rng};
use crate::mesh::EditablePoly;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::PI;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CloudFieldParams {
    pub seed: u32,
    /// Layer Base Height (m above the object's origin).
    pub base_height: f32,
    /// Layer Thickness (m): tallest regular cloud; storm towers rise above.
    pub thickness: f32,
    /// Area width (X) and depth (Y), metres.
    pub width: f32,
    pub depth: f32,
    /// Coverage 0..=1: share of the area under cloud.
    pub coverage: f32,
    /// Type Mix: relative weight per `CloudType::ALL` entry.
    pub mix: [f32; 4],
    /// Min / Max Cloud Size (m, footprint of a Cumulus).
    pub min_size: f32,
    pub max_size: f32,
    /// Detail Levels of every cloud (1..=3).
    pub detail: u32,
}

impl Default for CloudFieldParams {
    fn default() -> Self {
        Self {
            seed: 1,
            base_height: 600.0,
            thickness: 400.0,
            width: 4000.0,
            depth: 4000.0,
            coverage: 0.3,
            mix: [1.0, 0.3, 0.0, 0.0],
            min_size: 250.0,
            max_size: 600.0,
            detail: 2,
        }
    }
}

/// One cloud of the field: its parameters and where its base sits.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub params: CloudParams,
    pub offset: Vec3,
}

/// How much bigger than a Cumulus each type is drawn.
fn type_scale(k: CloudType) -> f32 {
    match k {
        CloudType::Cumulus => 1.0,
        CloudType::Congestus => 1.2,
        CloudType::Cumulonimbus => 3.0,
        CloudType::Stratocumulus => 2.5,
    }
}

impl CloudFieldParams {
    fn pick(&self, u: f32) -> Option<CloudType> {
        let total: f32 = self.mix.iter().map(|w| w.max(0.0)).sum();
        if total <= 0.0 {
            return None;
        }
        let mut acc = 0.0;
        for (k, w) in CloudType::ALL.iter().zip(self.mix) {
            acc += w.max(0.0) / total;
            if u < acc {
                return Some(*k);
            }
        }
        Some(CloudType::ALL[3])
    }

    /// The seeded clouds of the field (each with its own seed).
    pub fn placements(&self) -> Vec<Placement> {
        let mut r = rng(self.seed, 0xF1E1D);
        let (w, d) = (self.width.max(1.0), self.depth.max(1.0));
        let target = self.coverage.clamp(0.0, 1.0) * w * d;
        let (lo, hi) = (self.min_size.max(1.0), self.max_size.max(self.min_size.max(1.0)));
        let mut out: Vec<Placement> = Vec::new();
        let mut covered = 0.0;
        let mut tries = 0;
        while covered < target && tries < 4000 && out.len() < 400 {
            tries += 1;
            let Some(kind) = self.pick(r.gen_range(0.0..1.0)) else { break };
            let size = r.gen_range(lo..=hi) * type_scale(kind);
            let xy = Vec2::new(r.gen_range(-0.5..0.5) * w, r.gen_range(-0.5..0.5) * d);
            let rad = size * 0.4;
            if out.iter().any(|o| o.offset.truncate().distance(xy) < (rad + o.params.size * 0.4) * 0.75) {
                continue;
            }
            let (dw, dh) = kind.default_size();
            let tall = size * dh / dw;
            let height = if kind == CloudType::Cumulonimbus { tall } else { tall.min(self.thickness.max(1.0)) };
            let i = out.len() as u32;
            let params = CloudParams { seed: (hash(self.seed, i) * 1e6) as u32, size, height, detail: self.detail.clamp(1, 3), ..CloudParams::of(kind) };
            let z = self.base_height + r.gen_range(-0.05..0.05) * self.thickness;
            out.push(Placement { params, offset: xy.extend(z) });
            covered += PI * rad * rad;
        }
        out
    }

    /// Every cloud of the field merged into one preview mesh.
    pub fn build(&self) -> EditablePoly {
        let mut m = EditablePoly::new();
        for p in self.placements() {
            m.append(&p.params.build(), Mat4::from_translation(p.offset));
        }
        m
    }
}
