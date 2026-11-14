//! Delta tracking (real collisions) and ratio tracking (transmittance)
//! through one volume's majorant grid. Rays go to local space with an
//! unnormalised direction, so the parameter `t` stays in world metres.

use super::VolumeObj;
use crate::render::trace::Rng;
use bevy::math::Vec3;

/// Below this the ratio-tracked transmittance ends (a fully dark shadow).
const T_MIN: f32 = 1e-3;

fn step(rng: &mut Rng, m: f32) -> f32 {
    -(1.0 - rng.f()).max(1e-7).ln() / m
}

impl VolumeObj {
    fn local(&self, o: Vec3, d: Vec3) -> (Vec3, Vec3) {
        (self.inv.transform_point3(o), self.inv.transform_vector3(d))
    }

    /// Nearest real collision before `tmax`: its `t` and shape density.
    pub fn collide(&self, o: Vec3, d: Vec3, tmax: f32, rng: &mut Rng) -> Option<(f32, f32)> {
        let (ol, dl) = self.local(o, d);
        let (t0, t1) = self.grid.clip(ol, dl, tmax)?;
        let sigma = self.sigma();
        let mut hit = None;
        self.grid.walk(ol, dl, t0, t1, |cell, ta, tb| {
            let m = self.grid.max[cell] * sigma;
            if m <= 0.0 {
                return true;
            }
            let mut t = ta;
            loop {
                t += step(rng, m);
                if t >= tb {
                    return true;
                }
                let dn = self.density(ol + dl * t);
                if rng.f() * m < dn * sigma {
                    hit = Some((t, dn));
                    return false;
                }
            }
        });
        hit
    }

    /// Ratio-tracking estimate of the transmittance along `0..tmax`.
    pub fn transmittance(&self, o: Vec3, d: Vec3, tmax: f32, rng: &mut Rng) -> f32 {
        let (ol, dl) = self.local(o, d);
        let Some((t0, t1)) = self.grid.clip(ol, dl, tmax) else { return 1.0 };
        let sigma = self.sigma();
        let mut tr = 1.0f32;
        self.grid.walk(ol, dl, t0, t1, |cell, ta, tb| {
            let m = self.grid.max[cell] * sigma;
            if m <= 0.0 {
                return true;
            }
            // A uniform cell: exact Beer-Lambert, no noise.
            if self.grid.max[cell] >= 1.0 && matches!(self.src, super::density::Source::Uniform) {
                tr *= (-m * (tb - ta)).exp();
                return tr > T_MIN;
            }
            let mut t = ta;
            loop {
                t += step(rng, m);
                if t >= tb {
                    return true;
                }
                tr *= 1.0 - (self.density(ol + dl * t) * sigma / m).min(1.0);
                if tr < T_MIN {
                    tr = 0.0;
                    return false;
                }
            }
        });
        tr
    }
}
