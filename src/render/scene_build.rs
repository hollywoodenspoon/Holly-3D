//! Snapshot of the scene for offline rendering: world-space triangles in a
//! BVH, a material table, render lights, ambient / sky, fog and camera.
//! Owns all its data so a render can run on worker threads.

use super::bvh::{Bvh, Tri};
use super::lighting::{LightKind, emit_dir, headlight_rig};
use crate::materials::Material;
use crate::materials::shading::lin3;
use crate::mesh::triangulate::triangulate_polygon;
use crate::scene::Scene;
use crate::viewport::camera::ViewCamera;
use bevy::math::{Mat3, Vec3};

/// Radiance of a multiplier-1 directional light (matches the slot previews).
pub const K_DIRECT: f32 = 3.0;
/// Omni / spot intensity: equals a directional key at 4 units.
pub const K_POINT: f32 = K_DIRECT * 16.0;
/// Ambient level 1 -> diffuse ambient term.
pub const K_AMBIENT: f32 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RShape {
    Point,
    /// Emission axis + cos(half hotspot), cos(half falloff).
    Spot {
        dir: Vec3,
        cos_in: f32,
        cos_out: f32,
    },
    /// Direction the light travels.
    Directional {
        dir: Vec3,
    },
}

#[derive(Clone, Debug)]
pub struct RLight {
    pub shape: RShape,
    pub pos: Vec3,
    /// Linear radiance (color x multiplier x engine constant).
    pub radiance: Vec3,
    pub range: f32,
    pub shadows: bool,
    pub softness: f32,
    /// Shadow tint factor: (1 - density) + density x shadow color.
    pub shadow_tint: Vec3,
}

pub struct RenderScene {
    pub bvh: Bvh,
    pub materials: Vec<Material>,
    pub lights: Vec<RLight>,
    pub ambient: Vec3,
    /// Skylight hemisphere radiance (sampled with occlusion).
    pub sky: Vec3,
    pub background: Vec3,
    pub fog: Option<(Vec3, f32)>,
    pub camera: ViewCamera,
    /// Scene bounding radius (AO ray length, epsilons).
    pub radius: f32,
}

fn gather_tris(scene: &Scene, materials: &mut Vec<Material>) -> Vec<Tri> {
    let mut tris = vec![];
    for (obj, o) in scene.objects.values().enumerate().filter(|(_, o)| !o.hidden && !o.is_helper()) {
        let mat = o.material.and_then(|id| scene.materials.mats.get(id)).cloned().unwrap_or_else(|| Material {
            base_color: o.wire_color,
            roughness: 0.65,
            ..Material::named("Wire Color")
        });
        materials.push(mat);
        let mat = materials.len() - 1;
        let w = o.world_matrix();
        let nm = Mat3::from_mat4(w).inverse().transpose();
        let m = &o.evaluated_mesh;
        let normals = m.corner_normals();
        for (pid, p) in &m.polygons {
            let pts: Vec<Vec3> = p.vertex_ids().map(|v| m.pos(v)).collect();
            let ns = &normals[&pid];
            for [a, b, c] in triangulate_polygon(&pts) {
                let ix = [a, b, c];
                tris.push(Tri {
                    p: ix.map(|i| w.transform_point3(pts[i])),
                    n: ix.map(|i| (nm * ns[i]).normalize_or(Vec3::Z)),
                    uv: ix.map(|i| p.corners[i].uv),
                    col: ix.map(|i| m.vertices[p.corners[i].vertex_id].color),
                    mat,
                    obj: obj as u32,
                });
            }
        }
    }
    tris
}

impl RenderScene {
    pub fn build(scene: &Scene, camera: ViewCamera, use_rig: bool) -> Self {
        let mut materials = vec![];
        let tris = gather_tris(scene, &mut materials);
        let env = &scene.environment;
        let mut lights = vec![];
        let mut sky = Vec3::ZERO;
        if use_rig {
            for rl in headlight_rig(camera.forward(), camera.right(), camera.up()) {
                lights.push(RLight {
                    shape: RShape::Directional { dir: rl.dir.normalize() },
                    pos: Vec3::ZERO,
                    radiance: rl.color * rl.strength * K_DIRECT,
                    range: f32::MAX,
                    shadows: false,
                    softness: 0.0,
                    shadow_tint: Vec3::ZERO,
                });
            }
        } else {
            for (_, o, l) in scene.active_lights() {
                if l.kind == LightKind::Skylight {
                    sky += l.radiance();
                    continue;
                }
                let dir = emit_dir(o.transform.rotation);
                let (shape, k) = match l.kind {
                    LightKind::Omni => (RShape::Point, K_POINT),
                    LightKind::TargetSpot | LightKind::FreeSpot => (
                        RShape::Spot { dir, cos_in: (l.hotspot * 0.5).to_radians().cos(), cos_out: (l.falloff * 0.5).to_radians().cos() },
                        K_POINT,
                    ),
                    _ => (RShape::Directional { dir }, K_DIRECT),
                };
                let d = l.shadow_density.clamp(0.0, 1.0);
                lights.push(RLight {
                    shape,
                    pos: o.transform.translation,
                    radiance: l.radiance() * k,
                    range: l.range(),
                    shadows: l.cast_shadows,
                    softness: l.softness,
                    shadow_tint: Vec3::splat(1.0 - d) + lin3(l.shadow_color) * d,
                });
            }
        }
        let mut ambient = lin3(env.ambient) * env.ambient_level;
        if use_rig {
            ambient = ambient.max(Vec3::splat(0.25));
        }
        let bvh = Bvh::build(tris);
        let (lo, hi) = bvh.tris.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), t| {
            (lo.min(t.p[0]).min(t.p[1]).min(t.p[2]), hi.max(t.p[0]).max(t.p[1]).max(t.p[2]))
        });
        let radius = if bvh.tris.is_empty() { 1.0 } else { ((hi - lo).length() * 0.5).max(0.1) };
        Self {
            bvh,
            materials,
            lights,
            ambient: ambient * K_AMBIENT,
            sky,
            background: lin3(env.background),
            fog: env.fog.then(|| (lin3(env.fog_color), env.fog_density)),
            camera,
            radius,
        }
    }
}
