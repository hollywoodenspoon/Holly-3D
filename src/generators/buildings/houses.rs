//! Shared house pieces for the residential prefabs: a walled shell with
//! windows and a front door, roofs (gable, hip, flat with parapet) that
//! run along the longer side, and the lot extras (garden, driveway,
//! garage, picket fence) that can sit around a house.

use super::{Ctx, pane_id, parts};
use crate::generators::kit::{Kit, ids, sides};
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec3};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

/// Lot extras around a house (Residential prefabs only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Lot {
    /// Lawn, path to the door and bushes.
    pub garden: bool,
    pub driveway: bool,
    pub garage: bool,
    /// Picket fence round the lot.
    pub fence: bool,
}

impl Default for Lot {
    fn default() -> Self {
        Self { garden: true, driveway: false, garage: false, fence: true }
    }
}

impl Lot {
    pub const NONE: Lot = Lot { garden: false, driveway: false, garage: false, fence: false };
    /// (label, flag) pairs for the checkbox grid.
    pub fn flags(&mut self) -> [(&'static str, &mut bool); 4] {
        [("Garden", &mut self.garden), ("Driveway", &mut self.driveway), ("Garage", &mut self.garage), ("Fence", &mut self.fence)]
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Roof {
    /// Pitch in degrees.
    Gable(f32),
    Hip(f32),
    /// Flat with a parapet of this height.
    Flat(f32),
}

/// A box of storeys with windows on every side and a front door.
pub(super) struct Shell {
    pub hx: f32,
    pub hy: f32,
    pub floors: u32,
    /// Storey height.
    pub fh: f32,
    /// Plinth height (the ground floor sits on it).
    pub base: f32,
    pub wall: u16,
    /// Window spacing, window width x height.
    pub bay: f32,
    pub win: (f32, f32),
    /// Share of lit windows at night.
    pub lit: f64,
    /// Front door position along X (front faces -Y).
    pub door: Option<f32>,
}

impl Shell {
    pub fn new(hx: f32, hy: f32, floors: u32, wall: u16) -> Self {
        Self { hx, hy, floors, fh: 2.9, base: 0.45, wall, bay: 2.4, win: (1.0, 1.35), lit: 0.35, door: Some(0.0) }
    }
    /// Top of the walls.
    pub fn top(&self) -> f32 {
        self.base + self.fh * self.floors as f32
    }
}

/// Builds the shell: plinth, walls, windows (with shutters on the front
/// when that part is on), a band between storeys, and the door.
pub(super) fn walls(b: &mut Ctx, s: &Shell) {
    let (hx, hy) = (s.hx, s.hy);
    if s.base > 0.01 {
        b.kit.mat(ids::CURB).block(Vec3::new(-hx - 0.06, -hy - 0.06, 0.0), Vec3::new(hx + 0.06, hy + 0.06, s.base));
    }
    b.kit.mat(s.wall).block(Vec3::new(-hx, -hy, s.base), Vec3::new(hx, hy, s.top()));
    for f in 1..s.floors {
        let z = s.base + f as f32 * s.fh;
        b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.05, -hy - 0.05, z - 0.08), Vec3::new(hx + 0.05, hy + 0.05, z + 0.08));
    }
    let (r, em, shut) = (&mut b.r, b.p.emissive, b.p.parts.shutters);
    sides(&mut b.kit, hx, hy, |k, side, w| {
        let n = ((w / s.bay).floor() as usize).max(1);
        let cell = w / n as f32;
        for f in 0..s.floors {
            let z = s.base + f as f32 * s.fh + (s.fh - s.win.1) * 0.55;
            for i in 0..n {
                let x = -w * 0.5 + cell * (i as f32 + 0.5);
                if side == 0 && f == 0 && s.door.is_some_and(|d| (d - x).abs() < cell * 0.5) {
                    continue;
                }
                window(k, x, 0.0, z, s.win.0, s.win.1, pane_id(r, em, s.lit));
                if shut && side == 0 && cell > s.win.0 * 1.9 {
                    parts::shutters(k, x, z, s.win.0 * 0.5, s.win.1);
                }
            }
        }
    });
    if let Some(x) = s.door {
        door(&mut b.kit, x, -hy, s.base, 1.0, 2.15);
    }
}

/// A framed door `w` x `h` on a wall facing -Y at `y`, sill at `z`.
pub(super) fn door(k: &mut Kit, x: f32, y: f32, z: f32, w: f32, h: f32) {
    k.mat(ids::TRIM).block(Vec3::new(x - w * 0.5 - 0.1, y - 0.08, z), Vec3::new(x + w * 0.5 + 0.1, y, z + h + 0.12));
    k.mat(ids::IRON).quad(
        Vec3::new(x - w * 0.5, y - 0.09, z),
        Vec3::new(x + w * 0.5, y - 0.09, z),
        Vec3::new(x + w * 0.5, y - 0.09, z + h),
        Vec3::new(x - w * 0.5, y - 0.09, z + h),
    );
}

/// A roof over `2hx` x `2hy` from height `z`, its ridge along the longer side.
pub(super) fn roof(k: &mut Kit, hx: f32, hy: f32, z: f32, roof: Roof, overhang: f32, wall: u16) {
    let along_x = hx >= hy;
    let (a, c) = if along_x { (hx, hy) } else { (hy, hx) };
    let m = Mat4::from_translation(Vec3::Z * z) * if along_x { Mat4::IDENTITY } else { Mat4::from_rotation_z(FRAC_PI_2) };
    k.at(m, |k| match roof {
        Roof::Gable(p) => k.mat(ids::ROOF).gable(a, c, p.to_radians().tan() * c, overhang, wall),
        Roof::Hip(p) => k.mat(ids::ROOF).hipped(a, c, p.to_radians().tan() * c, overhang),
        Roof::Flat(h) => {
            k.mat(ids::ROOF).block(Vec3::new(-a, -c, 0.0), Vec3::new(a, c, 0.12));
            if h > 0.0 {
                k.mat(ids::TRIM);
                k.block(Vec3::new(-a - 0.1, -c - 0.1, 0.0), Vec3::new(a + 0.1, -c + 0.1, h));
                k.block(Vec3::new(-a - 0.1, c - 0.1, 0.0), Vec3::new(a + 0.1, c + 0.1, h));
                k.block(Vec3::new(-a - 0.1, -c + 0.1, 0.0), Vec3::new(-a + 0.1, c - 0.1, h));
                k.block(Vec3::new(a - 0.1, -c + 0.1, 0.0), Vec3::new(a + 0.1, c - 0.1, h));
            }
        }
    });
}

/// Ridge height of a pitched roof over `2hx` x `2hy`.
pub(super) fn rise(hx: f32, hy: f32, pitch: f32) -> f32 {
    pitch.to_radians().tan() * hx.min(hy)
}

/// Lot extras round a `2hx` x `2hy` house whose door is at `door_x`.
pub(super) fn lot(b: &mut Ctx, hx: f32, hy: f32, door_x: f32) {
    let l = b.p.lot;
    let side = if l.driveway || l.garage { 4.4 } else { 1.5 };
    let (x0, x1, y0, y1) = (-hx - 1.5, hx + side, -hy - 6.0, hy + 2.5);
    let k = &mut b.kit;
    if l.garden {
        k.mat(ids::FOLIAGE).block(Vec3::new(x0, y0, 0.0), Vec3::new(x1, y1, 0.04));
        k.mat(ids::SIDEWALK).block(Vec3::new(door_x - 0.6, y0, 0.0), Vec3::new(door_x + 0.6, -hy, 0.07));
        let n = ((hx * 2.0 / 1.6) as usize).clamp(2, 10);
        k.mat(ids::FOLIAGE);
        for i in 0..n {
            let x = -hx + 0.6 + (2.0 * hx - 1.2) * (i as f32 + 0.5) / n as f32 + b.r.gen_range(-0.2..0.2);
            if (x - door_x).abs() < 1.2 {
                continue;
            }
            let s = b.r.gen_range(0.75..1.15);
            k.frustum(Vec3::new(x, -hy - 0.75, 0.0), 0.5 * s, 0.32 * s, 0.85 * s, 7, true);
        }
    }
    if l.driveway || l.garage {
        let end = if l.garage { -hy + 0.5 } else { -hy + 5.0 };
        k.mat(ids::ASPHALT).block(Vec3::new(hx + 0.6, y0, 0.0), Vec3::new(hx + 3.8, end, 0.06));
    }
    if l.garage {
        k.mat(ids::PLASTER).block(Vec3::new(hx + 0.4, -hy + 0.5, 0.0), Vec3::new(hx + 4.0, -hy + 6.6, 2.7));
        k.mat(ids::TRIM).block(Vec3::new(hx + 0.3, -hy + 0.35, 2.7), Vec3::new(hx + 4.1, -hy + 6.75, 2.86));
        let y = -hy + 0.48;
        k.mat(ids::IRON).quad(Vec3::new(hx + 0.8, y, 0.0), Vec3::new(hx + 3.6, y, 0.0), Vec3::new(hx + 3.6, y, 2.2), Vec3::new(hx + 0.8, y, 2.2));
    }
    if l.fence {
        let gaps = [(door_x - 0.7, door_x + 0.7), (hx + 0.5, hx + 3.9)];
        let open = |x: f32| gaps.iter().take(if l.driveway || l.garage { 2 } else { 1 }).any(|(a, b)| x > *a && x < *b);
        k.mat(ids::TRIM);
        let posts = (((x1 - x0) / 0.55) as usize).max(2);
        for i in 0..=posts {
            let x = x0 + (x1 - x0) * i as f32 / posts as f32;
            if !open(x) {
                k.block(Vec3::new(x - 0.04, y0 - 0.04, 0.0), Vec3::new(x + 0.04, y0 + 0.04, 0.95));
            }
        }
        // Rails between the gaps, then the plain side and back fences.
        let mut cuts: Vec<f32> = vec![x0];
        for (a, b) in gaps.iter().take(if l.driveway || l.garage { 2 } else { 1 }) {
            cuts.extend([*a, *b]);
        }
        cuts.push(x1);
        cuts.sort_by(f32::total_cmp);
        for c in cuts.chunks(2) {
            for z in [0.3, 0.75] {
                k.block(Vec3::new(c[0], y0 - 0.06, z), Vec3::new(c[1], y0 - 0.02, z + 0.08));
            }
        }
        for (a, b) in [(Vec3::new(x0, y0, 0.0), Vec3::new(x0, y1, 0.0)), (Vec3::new(x1, y0, 0.0), Vec3::new(x1, y1, 0.0)), (Vec3::new(x0, y1, 0.0), Vec3::new(x1, y1, 0.0))] {
            let len = a.distance(b);
            let n = ((len / 1.5) as usize).max(1);
            for i in 0..=n {
                let p = a.lerp(b, i as f32 / n as f32);
                k.block(p - Vec3::new(0.05, 0.05, 0.0), p + Vec3::new(0.05, 0.05, 1.1));
            }
            k.beam(a + Vec3::Z * 0.85, b + Vec3::Z * 0.85, 0.05, 0.12);
            k.beam(a + Vec3::Z * 0.35, b + Vec3::Z * 0.35, 0.05, 0.12);
        }
    }
}

/// Runs `f` with the kit's local frame multiplied by `m` (for builders
/// that need the whole context, such as `walls` and `lot`).
pub(super) fn framed(b: &mut Ctx, m: Mat4, f: impl FnOnce(&mut Ctx)) {
    let saved = b.kit.xf;
    b.kit.xf = saved * m;
    f(b);
    b.kit.xf = saved;
}
