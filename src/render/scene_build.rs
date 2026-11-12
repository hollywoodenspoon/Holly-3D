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
    /// Near / far attenuation ramps.
    pub atten: crate::render::lighting::Atten,
    pub shadows: bool,
    pub softness: f32,
    /// Shadow tint factor: (1 - density) + density x shadow color.
    pub shadow_tint: Vec3,
}

pub struct RenderScene {
    pub bvh: Bvh,
    pub materials: Vec<Material>,
    /// Rotation and origin of every scene object (indexed by `Tri::obj`):
    /// the projection space of World / Triplanar materials.
    pub frames: Vec<(bevy::math::Quat, Vec3)>,
    pub lights: Vec<RLight>,
    pub ambient: Vec3,
    /// Skylight hemisphere radiance (sampled with occlusion).
    pub sky: Vec3,
    pub background: Vec3,
    /// HDRI seen by camera misses and reflections (strength applied).
    pub hdri: Option<crate::render::hdri::Hdri>,
    /// Procedural sky evaluated per ray (crisp stars / sun); `hdri` then
    /// holds its bake for ambient and the Cycles world.
    pub procedural: Option<super::sky::SkySettings>,
    pub fog: Option<(Vec3, f32)>,
    /// Volumetric godrays / height mist medium (Environment `8`).
    pub volume: Option<super::volumetrics::Medium>,
    pub camera: ViewCamera,
    /// Scene bounding radius (AO ray length, epsilons).
    pub radius: f32,
    /// Emissive polygons sampled as area lights.
    pub emitters: super::emitters::Emitters,
    /// Any see-through surface (opacity, opacity map or vertex alpha): shadow
    /// rays then pass through it, dimmed by its alpha (cloud shells).
    pub translucent: bool,
}

fn gather_tris(scene: &Scene, materials: &mut Vec<Material>) -> Vec<Tri> {
    let mut tris = vec![];
    for (obj, o) in scene.objects.values().enumerate().filter(|(_, o)| !o.hidden && !o.is_helper()) {
        let mat = o.material.and_then(|id| scene.materials.mats.get(id)).cloned().unwrap_or_else(|| Material {
            base_color: o.wire_color,
            roughness: 0.65,
            ..Material::named("Wire Color")
        });
        // A Multi/Sub-Object material adds one table entry per material ID used.
        let mut by_id: std::collections::HashMap<u16, usize> = Default::default();
        let mut mat_for = |id: u16| {
            *by_id.entry(if mat.is_multi() { id } else { 0 }).or_insert_with(|| {
                let mut sub = mat.for_id(id).clone();
                // Paint layers of a Multi/Sub-Object lie over all its sub-materials.
                if mat.is_multi() {
                    sub.paint = mat.paint.clone();
                }
                materials.push(sub);
                materials.len() - 1
            })
        };
        let w = o.world_matrix();
        let nm = Mat3::from_mat4(w).inverse().transpose();
        let m = o.render_mesh();
        let m = m.as_ref();
        let normals = m.corner_normals();
        for (pid, p) in &m.polygons {
            let pts: Vec<Vec3> = p.vertex_ids().map(|v| m.pos(v)).collect();
            let ns = &normals[&pid];
            let mat = mat_for(p.material_id);
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
        let frames = scene
            .objects
            .values()
            .map(|o| {
                let (_, rot, origin) = o.world_matrix().to_scale_rotation_translation();
                (rot, origin)
            })
            .collect();
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
                    atten: Default::default(),
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
                    atten: l.atten(),
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
        ambient += env.sky.night_ambient();
        let procedural = (env.sky.mode != super::sky::SkyMode::Off).then(|| env.sky.clone());
        let hdri = super::sky::procedural_hdri(&env.sky, 1024).or_else(|| env.hdri.clone()).filter(|h| h.image.is_some());
        if let Some(h) = hdri.as_ref().filter(|h| h.lighting) {
            ambient += h.image.as_ref().map_or(Vec3::ZERO, |i| i.average()) * h.strength / K_AMBIENT;
        }
        let bvh = Bvh::build(tris);
        let (lo, hi) = bvh.tris.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), t| {
            (lo.min(t.p[0]).min(t.p[1]).min(t.p[2]), hi.max(t.p[0]).max(t.p[1]).max(t.p[2]))
        });
        let radius = if bvh.tris.is_empty() { 1.0 } else { ((hi - lo).length() * 0.5).max(0.1) };
        let emitters = super::emitters::Emitters::build(&bvh, &materials);
        let translucent = bvh.tris.iter().any(|t| {
            let m = &materials[t.mat];
            m.opacity < 0.999 || m.active_map(crate::materials::MapChannel::Opacity).is_some() || t.col.iter().any(|c| c.w < 0.999)
        });
        Self {
            bvh,
            materials,
            frames,
            lights,
            ambient: ambient * K_AMBIENT,
            sky,
            background: lin3(env.background),
            hdri,
            procedural,
            fog: env.fog.then(|| (lin3(env.fog_color), env.fog_density)),
            volume: super::volumetrics::Medium::new(&env.volume),
            camera,
            radius,
            emitters,
            translucent,
        }
    }
}
