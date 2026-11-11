//! Cycles through Blender: writes the render scene as JSON, then runs
//! `blender --background --factory-startup -E CYCLES --python loader.py`.
//! The loader (`blender_loader.py`) rebuilds meshes, Principled materials,
//! lights, camera and world, picks the compute device (HIP + HIP-RT by
//! default, falling back to CPU), turns on OpenImageDenoise and renders a PNG.

use super::cycles::{wait, work_dir};
use super::framebuffer::FrameBuffer;
use super::scene_build::{RShape, RenderScene};
use super::settings::{CyclesDevice, RenderSettings};
use crate::materials::ShadingModel;
use crate::materials::shading::lin3;
use bevy::math::Vec3;
use serde_json::{Value, json};
use std::f32::consts::PI;
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub const LOADER: &str = include_str!("blender_loader.py");

fn v(a: Vec3) -> Value {
    json!([a.x, a.y, a.z])
}

fn camera(rs: &RenderScene, cfg: &RenderSettings) -> Value {
    let c = &rs.camera;
    let (r, u, f, e) = (c.right(), c.up(), c.forward(), c.eye());
    // Blender cameras look down local -Z with +Y up (row-major 4x4).
    let m = [[r.x, u.x, -f.x, e.x], [r.y, u.y, -f.y, e.y], [r.z, u.z, -f.z, e.z], [0.0, 0.0, 0.0, 1.0]];
    let size = c.ortho_size(cfg.aspect());
    json!({
        "matrix": m,
        "ortho": c.is_ortho(),
        "ortho_scale": size.x.max(size.y),
        "fov_y": c.fov_y(cfg.aspect()),
        "clip": [c.near.max(1e-3), if c.far.is_finite() { c.far } else { 1e5 }],
        "dof": (cfg.post.dof && !c.is_ortho()).then(|| json!({ "focus": cfg.post.focus_distance, "fstop": cfg.post.f_stop })),
    })
}

fn materials(rs: &RenderScene) -> Value {
    rs.materials
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "unlit": m.model == ShadingModel::Unlit,
                "base": v(lin3(m.base_color)),
                "metallic": m.metallic,
                "roughness": m.roughness,
                "ior": m.ior,
                "transmission": if m.model == ShadingModel::Crystal { m.transmission } else { 0.0 },
                "alpha": m.opacity,
                "emission": v(lin3(m.emissive) * m.emissive_strength),
                // Preserve Texture Appearance: camera rays see the base at this brightness.
                "camera_brightness": m.decoupled().then_some(m.direct_camera_brightness.clamp(0.0, 2.0)),
            })
        })
        .collect()
}

/// One mesh per (object, material): unwelded triangles + corner normals
/// and UVs, plus projection-space positions for World / Triplanar materials.
fn meshes(rs: &RenderScene) -> Value {
    let mut groups: std::collections::BTreeMap<(u32, usize), [Vec<f32>; 4]> = Default::default();
    for t in &rs.bvh.tris {
        let [p, n, uv, tpos] = groups.entry((t.obj, t.mat)).or_default();
        let frame = rs.materials[t.mat].projection.on().then(|| rs.frames.get(t.obj as usize).copied().unwrap_or_default());
        for k in 0..3 {
            p.extend(t.p[k].to_array());
            n.extend(t.n[k].to_array());
            uv.extend(t.uv[k].to_array());
            if let Some((rot, origin)) = frame {
                tpos.extend((rot.inverse() * (t.p[k] - origin)).to_array());
            }
        }
    }
    groups
        .into_iter()
        .map(|((obj, mat), [p, n, uv, tpos])| {
            let mut mesh = json!({ "obj": obj, "mat": mat, "verts": p, "normals": n, "uvs": uv });
            if !tpos.is_empty() {
                mesh["tpos"] = json!(tpos);
            }
            mesh
        })
        .collect()
}

/// Lights in Blender units: our radiance is irradiance x d² for points
/// (Blender point watts = 4π² x that) and irradiance for suns (x π).
fn lights(rs: &RenderScene) -> Value {
    rs.lights
        .iter()
        .map(|l| {
            let (kind, dir, energy, spot) = match l.shape {
                RShape::Point => ("POINT", Vec3::NEG_Z, 4.0 * PI * PI, None),
                RShape::Spot { dir, cos_in, cos_out } => {
                    let size = cos_out.acos() * 2.0;
                    let blend = (1.0 - cos_in.acos() / cos_out.acos().max(1e-4)).clamp(0.0, 1.0);
                    ("SPOT", dir, 4.0 * PI * PI, Some((size, blend)))
                }
                RShape::Directional { dir } => ("SUN", dir, PI, None),
            };
            let strength = l.radiance.max_element().max(1e-6);
            json!({
                "type": kind,
                "pos": v(l.pos),
                "dir": v(dir),
                "color": v(l.radiance / strength),
                "energy": strength * energy,
                "radius": l.softness.max(0.01),
                "spot": spot.map(|(s, b)| json!([s, b])),
                "shadows": l.shadows,
                "range": if l.range.is_finite() { l.range } else { 0.0 },
            })
        })
        .collect()
}

/// Color management: Blender view transform for the tonemapper, Exposure
/// (EV) and a contrast look.
fn view(cfg: &RenderSettings) -> Value {
    use super::post::Tonemap;
    let t = &cfg.post.tone;
    let transform = match t.op {
        Tonemap::AgX => "AgX",
        Tonemap::AcesFilmic => "Filmic",
        Tonemap::Reinhard => "Khronos PBR Neutral",
        Tonemap::StandardSrgb => "Standard",
    };
    let look = if t.contrast > 1.15 {
        "High Contrast"
    } else if t.contrast < 0.85 {
        "Low Contrast"
    } else {
        "None"
    };
    json!({ "transform": transform, "exposure": t.exposure, "look": look })
}

/// The whole scene description the loader reads.
pub fn scene_json(rs: &RenderScene, cfg: &RenderSettings, out_png: &Path) -> Value {
    let hdri = rs.hdri.as_ref().map(|h| json!({ "path": h.path, "strength": h.strength, "rotation": h.rotation }));
    json!({
        "format": "MiniMax-Cycles",
        "width": cfg.width,
        "height": cfg.height,
        "samples": cfg.cycles_samples,
        "max_bounces": cfg.max_depth.max(1) * 2,
        "transparent": cfg.transparent,
        "device": cfg.cycles_device.id(),
        "hiprt": cfg.hiprt && cfg.cycles_device == CyclesDevice::Hip,
        "denoise": cfg.denoise,
        "output": out_png,
        "camera": camera(rs, cfg),
        "view": view(cfg),
        "world": { "color": v(rs.background + rs.ambient + rs.sky), "hdri": hdri },
        "materials": materials(rs),
        "meshes": meshes(rs),
        "lights": lights(rs),
    })
}

/// Exports, runs Blender and loads the rendered PNG.
pub fn run(rs: &RenderScene, cfg: &RenderSettings, cancel: &AtomicBool) -> Result<FrameBuffer, String> {
    if let Some(h) = &rs.hdri {
        h.ensure_file()?;
    }
    let dir = work_dir()?;
    let (scene, loader, png) = (dir.join("scene.json"), dir.join("loader.py"), dir.join("render.png"));
    let mut json = scene_json(rs, cfg, &png);
    super::blender_tex::attach(&mut json, &rs.materials, &dir)?;
    let text = serde_json::to_string(&json).map_err(|e| e.to_string())?;
    std::fs::write(&scene, text).map_err(|e| e.to_string())?;
    std::fs::write(&loader, LOADER).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&png);
    let mut cmd = std::process::Command::new(&cfg.blender_exe);
    cmd.args(["--background", "--factory-startup", "-noaudio", "-E", "CYCLES", "--python-exit-code", "1", "--python"])
        .arg(&loader)
        .arg("--")
        .arg(&scene);
    wait(cmd, &cfg.blender_exe, cancel)?;
    let img = image::open(&png).map_err(|e| format!("Blender produced no image: {e}"))?;
    Ok(FrameBuffer::from_image(img))
}
