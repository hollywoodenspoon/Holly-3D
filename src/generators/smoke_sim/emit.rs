//! The source sphere's emission for one substep, worked out once on the
//! CPU as a short list of cells so the CPU and GPU solvers inject exactly
//! the same smoke, heat, fuel and gusts (the 4D noise stays on the CPU).

use super::{SimParams, SimState};
use bevy::math::Vec3;
use noise::{NoiseFn, Perlin};

/// What the source does to one cell: smoke and heat rise to at least
/// `dens` / `temp`, fuel grows by `fuel`, the upward speed eases toward
/// `w` by `s`, and the sideways gusts add `gx` / `gy`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emit {
    pub i: u32,
    pub s: f32,
    pub dens: f32,
    pub temp: f32,
    pub fuel: f32,
    pub w: f32,
    pub gx: f32,
    pub gy: f32,
}

impl Emit {
    /// The 8 words the GPU solver reads (the index bit-cast).
    pub fn words(&self) -> [f32; 8] {
        [f32::from_bits(self.i), self.s, self.dens, self.temp, self.fuel, self.w, self.gx, self.gy]
    }
}

impl SimState {
    /// The source's cells this substep (time `self.time`, length `dt`).
    pub fn emit_list(&self, p: &SimParams, dt: f32) -> Vec<Emit> {
        let (c, r) = self.source(p);
        let noise = Perlin::new(p.seed);
        let (t, turb) = (f64::from(self.time) * 1.7, p.turbulence.clamp(0.0, 2.0));
        let lo = (c - r - 1.0).max(Vec3::ZERO);
        let hi = (c + r + 1.0).min(Vec3::new(self.n[0] as f32 - 1.0, self.n[1] as f32 - 1.0, self.n[2] as f32 - 1.0));
        let mut out = Vec::new();
        for z in lo.z as usize..=hi.z.max(0.0) as usize {
            for y in lo.y as usize..=hi.y as usize {
                for x in lo.x as usize..=hi.x as usize {
                    let q = Vec3::new(x as f32, y as f32, z as f32);
                    let s = (1.0 - (q - c).length_squared() / (r * r)).max(0.0);
                    if s <= 0.0 {
                        continue;
                    }
                    let m = q * (0.35 * self.h.recip().min(4.0) / 4.0).max(0.05);
                    let at = |o: f64| noise.get([f64::from(m.x) + o, f64::from(m.y), f64::from(m.z), t]) as f32;
                    let nz = at(0.0) * turb;
                    let s = s.sqrt();
                    // Sideways gusts break the plume's symmetry so eddies form.
                    let k = turb * (p.source_speed.abs() + 0.5) * 0.6 * s;
                    out.push(Emit {
                        i: self.density.idx(x, y, z) as u32,
                        s,
                        dens: p.smoke.max(0.0) * s * (1.0 + nz).max(0.0),
                        temp: p.heat.max(0.0) * s,
                        fuel: p.fuel.max(0.0) * s * dt,
                        w: p.source_speed * (1.0 + 0.5 * nz),
                        gx: k * at(17.3),
                        gy: k * at(-31.7),
                    });
                }
            }
        }
        out
    }

    pub(super) fn emit(&mut self, p: &SimParams, dt: f32) {
        for e in self.emit_list(p, dt) {
            let i = e.i as usize;
            self.density.d[i] = self.density.d[i].max(e.dens);
            self.temperature.d[i] = self.temperature.d[i].max(e.temp);
            self.fuel.d[i] += e.fuel;
            self.vel[2].d[i] += (e.w - self.vel[2].d[i]) * e.s;
            self.vel[0].d[i] += e.gx;
            self.vel[1].d[i] += e.gy;
        }
    }
}
