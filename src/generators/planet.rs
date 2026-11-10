//! Planet Studio (Create -> Geometry -> Worlds -> Planet): an equal-angle
//! quad-sphere planet with four concentric toggleable layers, each on its
//! own Material ID so the Planet Studio pack shades them:
//! 1–4 Planet Surface (procedural preset or a custom texture),
//! 5 Weather & Cloud Shell (radius x 1.015), 6 Atmosphere Rim Halo,
//! 7 Planetary Rings.

use super::planet_layers::{Atmosphere, CloudShell, Rings};
use super::planet_surface::{ID_LAND, SurfaceField, SurfacePreset};
use super::rgba;
use super::sphere::equal_angle_sphere;
use crate::mesh::EditablePoly;
use bevy::math::{Mat4, Vec3};

/// Which layer a custom texture goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetPart {
    Surface,
    Clouds,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlanetParams {
    pub radius: f32,
    /// Quads per cube-face edge (4..=128).
    pub subdivisions: u32,
    pub seed: u32,
    pub show_surface: bool,
    pub surface: SurfacePreset,
    /// Fraction of the height range under water (Terran / Desert basins).
    pub sea_level: f32,
    /// Mountain relief as a fraction of the radius.
    pub relief: f32,
    /// `[ Load Custom Surface Texture ]`: the surface becomes one plain
    /// sphere (ID 1, white vertex color) and the Planet Surface
    /// sub-material shows this image through the lat-long UVs.
    pub custom_surface: Option<String>,
    pub clouds: CloudShell,
    pub atmosphere: Atmosphere,
    pub rings: Rings,
    /// Sun direction for the atmosphere's terminator glow (degrees).
    pub sun_azimuth: f32,
    pub sun_elevation: f32,
}

impl Default for PlanetParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            subdivisions: 32,
            seed: 1,
            show_surface: true,
            surface: SurfacePreset::Terran,
            sea_level: 0.52,
            relief: 0.03,
            custom_surface: None,
            clouds: CloudShell::default(),
            atmosphere: Atmosphere::default(),
            rings: Rings::default(),
            sun_azimuth: 30.0,
            sun_elevation: 10.0,
        }
    }
}

impl PlanetParams {
    /// Preset defaults: gas giants get rings and no relief, moons lose
    /// their air and clouds.
    pub fn with_preset(&self, preset: SurfacePreset) -> Self {
        let mut p = Self { surface: preset, ..self.clone() };
        let air = !matches!(preset, SurfacePreset::Moon);
        (p.clouds.enabled, p.atmosphere.enabled) = (air && preset != SurfacePreset::GasGiant, air);
        p.rings.enabled = preset == SurfacePreset::GasGiant;
        p.relief = match preset {
            SurfacePreset::Terran => 0.03,
            SurfacePreset::Moon => 0.04,
            SurfacePreset::Desert => 0.025,
            SurfacePreset::Ice => 0.015,
            SurfacePreset::GasGiant => 0.0,
        };
        p.atmosphere.color = match preset {
            SurfacePreset::Desert => [0.95, 0.6, 0.4],
            SurfacePreset::GasGiant => [0.9, 0.8, 0.6],
            _ => [0.35, 0.6, 1.0],
        };
        p
    }

    pub fn sun_dir(&self) -> Vec3 {
        let (az, el) = (self.sun_azimuth.to_radians(), self.sun_elevation.to_radians());
        Vec3::new(el.cos() * az.cos(), el.cos() * az.sin(), el.sin())
    }

    fn subdiv(&self) -> u32 {
        self.subdivisions.clamp(4, 128)
    }

    /// The Planet Surface layer (unit radius).
    pub fn surface_mesh(&self) -> EditablePoly {
        let mut m = equal_angle_sphere(self.subdiv());
        if self.custom_surface.is_some() {
            m.polygons.values_mut().for_each(|p| p.material_id = 1);
            return m;
        }
        let field = SurfaceField::new(self.surface, self.seed, self.sea_level, self.relief);
        let mut ids = std::collections::HashMap::new();
        for (id, v) in m.vertices.iter_mut() {
            let s = field.sample(v.position);
            v.position *= 1.0 + s.elevation;
            v.color = rgba(s.color, 1.0);
            ids.insert(id, s.id);
        }
        // A polygon takes its corners' most common ID (ties: the highest).
        for p in m.polygons.values_mut() {
            let mut counts = [0u8; 8];
            p.corners.iter().for_each(|c| counts[ids[&c.vertex_id].min(7) as usize] += 1);
            let best = (1..8).rev().max_by_key(|&i| counts[i]).unwrap_or(ID_LAND as usize);
            p.material_id = best as u16;
        }
        m.finalize();
        m
    }

    pub fn build(&self) -> EditablePoly {
        let mut out = EditablePoly::new();
        let scale = Mat4::from_scale(Vec3::splat(self.radius.max(1e-3)));
        if self.show_surface {
            out.append(&self.surface_mesh(), scale);
        }
        if self.clouds.enabled {
            out.append(&self.clouds.build(self.seed, self.subdiv()), scale);
        }
        if self.atmosphere.enabled {
            out.append(&self.atmosphere.build((self.subdiv() / 2).max(8), self.sun_dir()), scale);
        }
        if self.rings.enabled {
            out.append(&self.rings.build(self.seed), scale);
        }
        out.finalize();
        out
    }
}
