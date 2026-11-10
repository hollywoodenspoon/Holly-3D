//! Planet Surface layer presets: Terran continents & oceans, cratered moon,
//! arid desert world, ice world and gas giant storm bands. Each direction
//! on the unit sphere gets an elevation (fraction of the radius), an sRGB
//! vertex color and a Material ID (1 ocean / lowland, 2 land, 3 highland /
//! rock, 4 ice) matching the Planet Studio pack.

use super::{hash, smooth};
use bevy::math::Vec3;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, RidgedMulti};

pub const ID_OCEAN: u16 = 1;
pub const ID_LAND: u16 = 2;
pub const ID_HIGH: u16 = 3;
pub const ID_ICE: u16 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SurfacePreset {
    #[default]
    Terran,
    Moon,
    Desert,
    Ice,
    GasGiant,
}

impl SurfacePreset {
    pub const ALL: [SurfacePreset; 5] = [Self::Terran, Self::Moon, Self::Desert, Self::Ice, Self::GasGiant];
    pub fn label(self) -> &'static str {
        match self {
            Self::Terran => "Terran Continents & Oceans",
            Self::Moon => "Cratered Moon",
            Self::Desert => "Arid Desert World",
            Self::Ice => "Ice World",
            Self::GasGiant => "Gas Giant Storm Bands",
        }
    }
}

/// One surface sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePoint {
    /// Radial offset as a fraction of the radius.
    pub elevation: f32,
    /// sRGB vertex color.
    pub color: Vec3,
    pub id: u16,
}

pub(crate) struct SurfaceField {
    preset: SurfacePreset,
    seed: u32,
    sea: f32,
    relief: f32,
    fbm: Fbm<Perlin>,
    ridged: RidgedMulti<Perlin>,
    detail: Fbm<Perlin>,
    craters: Vec<(Vec3, f32)>,
}

fn get3(n: &impl NoiseFn<f64, 3>, p: Vec3) -> f32 {
    n.get(p.as_dvec3().to_array()) as f32
}

fn lerp3(stops: &[(f32, Vec3)], t: f32) -> Vec3 {
    let mut c = stops[0].1;
    for w in stops.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if t >= a {
            c = ca.lerp(cb, ((t - a) / (b - a).max(1e-6)).clamp(0.0, 1.0));
        }
    }
    c
}

impl SurfaceField {
    pub fn new(preset: SurfacePreset, seed: u32, sea_level: f32, relief: f32) -> Self {
        let craters = (0..90)
            .map(|i| {
                let (z, a) = (hash(seed, i * 3) * 2.0 - 1.0, hash(seed, i * 3 + 1) * std::f32::consts::TAU);
                let s = (1.0 - z * z).max(0.0).sqrt();
                // Many small craters, a few large basins.
                (Vec3::new(s * a.cos(), s * a.sin(), z), 0.03 + 0.25 * hash(seed, i * 3 + 2).powi(5))
            })
            .collect();
        Self {
            preset,
            seed,
            sea: sea_level.clamp(0.0, 1.0),
            relief: relief.max(0.0),
            fbm: Fbm::<Perlin>::new(seed).set_octaves(7).set_persistence(0.5),
            ridged: RidgedMulti::<Perlin>::new(seed.wrapping_add(5)).set_octaves(6),
            detail: Fbm::<Perlin>::new(seed.wrapping_add(9)).set_octaves(4),
            craters,
        }
    }

    pub fn sample(&self, d: Vec3) -> SurfacePoint {
        match self.preset {
            SurfacePreset::Terran => self.terran(d),
            SurfacePreset::Moon => self.moon(d),
            SurfacePreset::Desert => self.desert(d),
            SurfacePreset::Ice => self.ice(d),
            SurfacePreset::GasGiant => self.gas_giant(d),
        }
    }

    fn terran(&self, d: Vec3) -> SurfacePoint {
        let h = (get3(&self.fbm, d * 1.4) * 0.5 + 0.5 + 0.25 * get3(&self.ridged, d * 2.2).max(0.0) * 0.4).clamp(0.0, 1.0);
        let lat = d.z.abs();
        let polar = lat > 0.86 - 0.08 * get3(&self.detail, d * 4.0);
        if h < self.sea {
            let depth = (self.sea - h) / self.sea.max(1e-3);
            let col = Vec3::new(0.12, 0.45, 0.6).lerp(Vec3::new(0.02, 0.1, 0.3), smooth(0.0, 0.5, depth));
            let (col, id) = if polar { (Vec3::new(0.85, 0.92, 0.97), ID_ICE) } else { (col, ID_OCEAN) };
            return SurfacePoint { elevation: 0.0, color: col, id };
        }
        let e = (h - self.sea) / (1.0 - self.sea).max(1e-3);
        let dry = smooth(0.15, 0.4, 1.0 - lat) * smooth(0.2, 0.6, get3(&self.detail, d * 2.0) * 0.5 + 0.5);
        let low = Vec3::new(0.2, 0.42, 0.14).lerp(Vec3::new(0.62, 0.53, 0.33), dry);
        let col = lerp3(&[(0.0, Vec3::new(0.76, 0.7, 0.5)), (0.04, low), (0.45, Vec3::new(0.4, 0.33, 0.22)), (0.8, Vec3::new(0.5, 0.48, 0.46))], e);
        let (col, id) = if polar || e > 0.82 {
            (Vec3::new(0.93, 0.95, 0.98), ID_ICE)
        } else if e > 0.45 {
            (col, ID_HIGH)
        } else {
            (col, ID_LAND)
        };
        SurfacePoint { elevation: e.powf(1.5) * self.relief, color: col, id }
    }

    fn moon(&self, d: Vec3) -> SurfacePoint {
        let mut h = 0.5 + 0.15 * get3(&self.fbm, d * 2.0);
        let mut rim_light: f32 = 0.0;
        for &(c, r) in &self.craters {
            let x = d.dot(c).clamp(-1.0, 1.0).acos() / r;
            if x < 1.6 {
                // Bowl, raised rim, then fade.
                h += if x < 1.0 { -0.35 * (1.0 - x * x) + 0.12 } else { 0.12 * (1.0 - smooth(1.0, 1.6, x)) };
                rim_light = rim_light.max(1.0 - (x - 1.0).abs() * 3.0);
            }
        }
        let maria = smooth(0.1, -0.15, get3(&self.detail, d * 1.1));
        let grey = 0.55 - 0.25 * maria + 0.12 * rim_light.max(0.0) + 0.06 * get3(&self.ridged, d * 8.0);
        let id = if maria > 0.5 { ID_LAND } else { ID_HIGH };
        SurfacePoint { elevation: (h - 0.5) * self.relief, color: Vec3::splat(grey.clamp(0.1, 0.9)), id }
    }

    fn desert(&self, d: Vec3) -> SurfacePoint {
        let h = (get3(&self.fbm, d * 1.6) * 0.5 + 0.5).clamp(0.0, 1.0);
        let ridges = get3(&self.ridged, d * 3.0).max(0.0);
        let e = (h * 0.7 + ridges * 0.3).clamp(0.0, 1.0);
        let col = lerp3(&[(0.0, Vec3::new(0.55, 0.3, 0.16)), (0.4, Vec3::new(0.82, 0.55, 0.3)), (0.75, Vec3::new(0.66, 0.4, 0.25))], e);
        let polar = d.z.abs() > 0.94;
        let (col, id) = if polar {
            (Vec3::new(0.9, 0.88, 0.85), ID_ICE)
        } else if e < self.sea * 0.6 {
            (col * 0.85, ID_OCEAN)
        } else if e > 0.62 {
            (col, ID_HIGH)
        } else {
            (col, ID_LAND)
        };
        SurfacePoint { elevation: e * self.relief, color: col, id }
    }

    fn ice(&self, d: Vec3) -> SurfacePoint {
        // Thin dark-blue fracture lines where a noise field crosses zero.
        let crack = (1.0 - smooth(0.0, 0.05, get3(&self.detail, d * 6.0).abs())) * 0.7;
        let base = Vec3::new(0.86, 0.93, 0.98).lerp(Vec3::new(0.62, 0.78, 0.9), smooth(-0.3, 0.4, get3(&self.fbm, d * 2.0)));
        let col = base.lerp(Vec3::new(0.3, 0.5, 0.68), crack);
        let id = if crack > 0.3 { ID_HIGH } else { ID_ICE };
        SurfacePoint { elevation: (get3(&self.fbm, d * 2.5) * 0.5 - crack * 0.4) * self.relief * 0.5, color: col, id }
    }

    fn gas_giant(&self, d: Vec3) -> SurfacePoint {
        let warp = get3(&self.fbm, d * 2.5) * 0.12;
        let mut lat = d.z + warp;
        // Great storm ovals: the bands swirl around a few seeded eyes.
        for i in 0..3 {
            let (z, a) = (hash(self.seed, 200 + i) * 1.2 - 0.6, hash(self.seed, 210 + i) * std::f32::consts::TAU);
            let s = (1.0 - z * z).max(0.0).sqrt();
            let eye = Vec3::new(s * a.cos(), s * a.sin(), z);
            let dist = (d - eye).length() / (0.12 + 0.08 * hash(self.seed, 220 + i));
            lat += (-dist * dist).exp() * 0.08 * (dist * 6.0).sin();
        }
        let t = (lat * 7.0 + 0.5).rem_euclid(1.0);
        let col = lerp3(
            &[(0.0, Vec3::new(0.85, 0.75, 0.6)), (0.3, Vec3::new(0.7, 0.45, 0.3)), (0.55, Vec3::new(0.93, 0.88, 0.78)), (0.8, Vec3::new(0.6, 0.38, 0.26)), (1.0, Vec3::new(0.85, 0.75, 0.6))],
            t,
        );
        SurfacePoint { elevation: 0.0, color: col, id: ID_LAND }
    }
}
