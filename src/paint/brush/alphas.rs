//! The procedural shapes behind every brush alpha and damage stamp. Each
//! is a function of the point `q` in the brush disc (-1..1, x along the
//! stroke) and a seed that changes from stamp to stamp.

use super::{Alpha, Dab};
use crate::materials::proc_engine::lattice::{cellular, rand2, value};
use crate::paint::falloff_weight;
use crate::paint::layers::Flat;
use bevy::math::Vec2;

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth noise 0..1 around a circle (`spokes` lobes).
fn ring_noise(angle: f32, spokes: f32, seed: u32) -> f32 {
    let t = (angle / std::f32::consts::TAU + 0.5) * spokes;
    value(t, 0.5, spokes as i32, seed)
}

fn look(color: [f32; 3], rough: f32, metal: f32, height: f32, emission: f32, channels: [bool; 5]) -> Option<Flat> {
    Some(Flat { color, rough, metal, height, emission, channels })
}

const COLOR_ROUGH_HEIGHT: [bool; 5] = [true, true, false, true, false];
const COAT: [bool; 5] = [true, true, true, true, false];

impl Alpha {
    /// The alpha at `q`; `falloff` shapes the round brushes.
    pub fn eval(self, q: Vec2, falloff: f32, seed: u32) -> Dab {
        let r = q.length();
        if r >= 1.0 {
            return Dab { cov: 0.0, look: None };
        }
        let angle = q.y.atan2(q.x);
        match self {
            Alpha::SoftRound => Dab { cov: falloff_weight(r, 1.0, falloff), look: None },
            Alpha::HardRound => Dab { cov: 1.0 - smooth(0.94, 1.0, r), look: None },
            Alpha::BulletHole => {
                // A pit, a raised chipped lip, then soot fading outwards.
                let lip = 0.3 + 0.05 * ring_noise(angle, 9.0, seed);
                if r < lip * 0.8 {
                    let depth = 0.08 + 0.3 * (r / (lip * 0.8)).powi(2);
                    Dab { cov: 1.0, look: look([0.04, 0.04, 0.04], 0.95, 0.0, depth, 0.0, COAT) }
                } else if r < lip * 1.25 {
                    let t = (r - lip * 0.8) / (lip * 0.45);
                    let h = 0.5 + 0.2 * (t * std::f32::consts::PI).sin();
                    Dab { cov: 1.0 - smooth(0.7, 1.0, t) * 0.3, look: look([0.62, 0.6, 0.57], 0.7, 0.0, h, 0.0, COLOR_ROUGH_HEIGHT) }
                } else {
                    let streak = ring_noise(angle, 14.0, seed ^ 0x51);
                    let cov = (1.0 - smooth(lip * 1.25, 0.55 + 0.45 * streak, r)) * 0.75;
                    Dab { cov, look: look([0.06, 0.055, 0.05], 0.95, 0.0, 0.5, 0.0, [true, true, false, false, false]) }
                }
            }
            Alpha::BlastStar => {
                // Soot thrown out along uneven rays from a scorched core.
                let ray = ring_noise(angle, 11.0, seed).powf(1.6);
                let reach = 0.3 + 0.7 * ray;
                let cov = (1.0 - smooth(reach * 0.35, reach, r)) * (0.75 + 0.25 * value(q.x * 6.0 + 9.0, q.y * 6.0 + 9.0, 64, seed));
                let core = 1.0 - smooth(0.0, 0.3, r);
                let c = 0.03 + 0.09 * (1.0 - core);
                Dab { cov, look: look([c * 1.15, c, c * 0.85], 0.96, 0.0, 0.5 - 0.06 * core, 0.0, COLOR_ROUGH_HEIGHT) }
            }
            Alpha::BloodSplatter => {
                // A warped pool with droplets flung around it.
                let pool = 0.34 + 0.16 * ring_noise(angle, 7.0, seed);
                let mut cov = 1.0 - smooth(pool * 0.8, pool, r);
                let cell = cellular(q.x * 5.0 + 20.0, q.y * 5.0 + 20.0, 64, 0.9, seed);
                let drop_r = 0.08 + 0.3 * cell.id * (1.0 - r);
                if cell.id > 0.45 {
                    cov = cov.max(1.0 - smooth(drop_r * 0.6, drop_r, cell.f1));
                }
                let dark = 0.8 + 0.2 * value(q.x * 4.0 + 5.0, q.y * 4.0 + 5.0, 64, seed);
                Dab { cov, look: look([0.36 * dark, 0.015, 0.02], 0.22, 0.0, 0.52, 0.0, COLOR_ROUGH_HEIGHT) }
            }
            Alpha::BurnPatch => {
                // Charcoal with an ashen crackle, feathered by noise.
                let edge = 0.55 + 0.4 * value(q.x * 3.0 + 7.0, q.y * 3.0 + 7.0, 64, seed);
                let cov = 1.0 - smooth(edge * 0.55, edge, r);
                let cell = cellular(q.x * 7.0 + 30.0, q.y * 7.0 + 30.0, 64, 1.0, seed);
                let crack = 1.0 - smooth(0.0, 0.09, cell.f2 - cell.f1);
                let c = 0.035 + 0.2 * crack;
                Dab { cov, look: look([c, c * 0.95, c * 0.9], 0.97, 0.0, 0.5 - 0.1 * crack, 0.0, COAT) }
            }
            Alpha::EmberCracks => {
                // Glowing veins between cooled plates.
                let cell = cellular(q.x * 4.5 + 40.0, q.y * 4.5 + 40.0, 64, 1.0, seed);
                let vein = 1.0 - smooth(0.0, 0.14, cell.f2 - cell.f1);
                let fade = 1.0 - smooth(0.55, 1.0, r);
                if vein > 0.25 {
                    let heat = vein * fade;
                    Dab { cov: fade, look: look([1.0, 0.25 + 0.45 * heat, 0.04 + 0.2 * heat * heat], 0.8, 0.0, 0.5 - 0.12 * vein, heat, [true, true, false, true, true]) }
                } else {
                    Dab { cov: fade * 0.85, look: look([0.045, 0.04, 0.04], 0.95, 0.0, 0.5, 0.0, [true, true, false, true, true]) }
                }
            }
            Alpha::Scratch => {
                // A thin gouge along the stroke, tapering to both ends.
                let wobble = (value(q.x * 3.0 + 11.0, 0.5, 64, seed) - 0.5) * 0.12;
                let half = 0.07 * (1.0 - q.x * q.x).max(0.0).sqrt();
                let d = (q.y - wobble).abs();
                let cov = 1.0 - smooth(half * 0.5, half.max(1e-4), d);
                Dab { cov, look: look([0.8, 0.8, 0.82], 0.32, 1.0, 0.3, 0.0, COAT) }
            }
            Alpha::Rivet => {
                // A dome; colour is left alone so it takes the surface's.
                let dome = (1.0 - (r / 0.6).powi(2)).max(0.0).sqrt();
                let cov = 1.0 - smooth(0.6, 0.72, r);
                Dab { cov, look: look([0.5; 3], 0.45, 0.0, 0.5 + 0.4 * dome, 0.0, [false, false, false, true, false]) }
            }
        }
    }
}

/// Random turn (radians) for stamp number `k` of a stroke.
pub fn spin(k: u32, seed: u32) -> f32 {
    rand2(k as i32, 17, seed) * std::f32::consts::TAU
}
