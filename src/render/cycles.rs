//! Cycles Standalone Bridge: writes the render scene as a Cycles XML file
//! (camera, Principled BSDF shaders, meshes, lights, background) and runs
//! the headless `cycles` CLI, loading its PNG back into the frame buffer.
//! The default route goes through Blender instead (`blender.rs`).

use super::framebuffer::FrameBuffer;
use super::scene_build::{RShape, RenderScene};
use super::settings::{CyclesVia, RenderSettings};
use crate::materials::ShadingModel;
use crate::materials::shading::lin3;
use bevy::math::Vec3;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

fn v3(v: Vec3) -> String {
    format!("{} {} {}", v.x, v.y, v.z)
}

fn camera(out: &mut String, rs: &RenderScene, cfg: &RenderSettings) {
    let c = &rs.camera;
    let (r, u, f, e) = (c.right(), c.up(), c.forward(), c.eye());
    // Cycles camera space: +X right, +Y up, +Z forward (column-major matrix).
    let m = [r.x, r.y, r.z, 0.0, u.x, u.y, u.z, 0.0, f.x, f.y, f.z, 0.0, e.x, e.y, e.z, 1.0];
    let m: Vec<String> = m.iter().map(|v| v.to_string()).collect();
    let _ = writeln!(out, "<camera width=\"{}\" height=\"{}\" />", cfg.width, cfg.height);
    let _ = writeln!(out, "<transform matrix=\"{}\">", m.join(" "));
    if c.is_ortho() {
        let _ = writeln!(out, "  <camera type=\"orthographic\" />");
    } else {
        let _ = writeln!(out, "  <camera type=\"perspective\" fov=\"{}\" />", c.fov_y(cfg.width as f32 / cfg.height.max(1) as f32));
    }
    let _ = writeln!(out, "</transform>");
}

fn shaders(out: &mut String, rs: &RenderScene) {
    for (i, m) in rs.materials.iter().enumerate() {
        let base = lin3(m.base_color);
        let _ = writeln!(out, "<shader name=\"mat{i}\">");
        if m.model == ShadingModel::Unlit {
            let _ = writeln!(out, "  <emission name=\"e\" color=\"{}\" strength=\"1\" />", v3(base));
            let _ = writeln!(out, "  <connect from=\"e emission\" to=\"output surface\" />");
        } else {
            let trans = if m.model == ShadingModel::Crystal { m.transmission } else { 0.0 };
            let emit = lin3(m.emissive) * m.emissive_strength;
            let _ = writeln!(
                out,
                "  <principled_bsdf name=\"p\" base_color=\"{}\" metallic=\"{}\" roughness=\"{}\" ior=\"{}\" transmission_weight=\"{}\" alpha=\"{}\" emission_color=\"{}\" emission_strength=\"1\" />",
                v3(base),
                m.metallic,
                m.roughness,
                m.ior,
                trans,
                m.opacity,
                v3(emit)
            );
            let _ = writeln!(out, "  <connect from=\"p bsdf\" to=\"output surface\" />");
        }
        let _ = writeln!(out, "</shader>");
    }
}

fn meshes(out: &mut String, rs: &RenderScene) {
    let mut by_mat: Vec<Vec<usize>> = vec![vec![]; rs.materials.len()];
    for (i, t) in rs.bvh.tris.iter().enumerate() {
        by_mat[t.mat].push(i);
    }
    for (mat, tris) in by_mat.iter().enumerate().filter(|(_, t)| !t.is_empty()) {
        let mut p = String::new();
        let mut verts = String::new();
        let nverts = vec!["3"; tris.len()].join(" ");
        for (k, &ti) in tris.iter().enumerate() {
            for (c, v) in rs.bvh.tris[ti].p.iter().enumerate() {
                let _ = write!(p, "{} ", v3(*v));
                let _ = write!(verts, "{} ", k * 3 + c);
            }
        }
        let _ = writeln!(out, "<state shader=\"mat{mat}\">");
        let _ = writeln!(out, "  <mesh P=\"{}\" nverts=\"{nverts}\" verts=\"{}\" />", p.trim_end(), verts.trim_end());
        let _ = writeln!(out, "</state>");
    }
}

fn lights(out: &mut String, rs: &RenderScene) {
    for (i, l) in rs.lights.iter().enumerate() {
        let _ = writeln!(out, "<shader name=\"light{i}\">");
        let _ = writeln!(out, "  <emission name=\"e\" color=\"{}\" strength=\"1\" />", v3(l.radiance));
        let _ = writeln!(out, "  <connect from=\"e emission\" to=\"output surface\" />");
        let _ = writeln!(out, "</shader>");
        let _ = writeln!(out, "<state shader=\"light{i}\">");
        let size = l.softness.max(0.01);
        match l.shape {
            RShape::Point => {
                let _ = writeln!(out, "  <light light_type=\"point\" co=\"{}\" size=\"{size}\" />", v3(l.pos));
            }
            RShape::Spot { dir, cos_out, cos_in } => {
                let angle = cos_out.acos() * 2.0;
                let blend = (1.0 - cos_in.acos() / cos_out.acos().max(1e-4)).clamp(0.0, 1.0);
                let _ = writeln!(
                    out,
                    "  <light light_type=\"spot\" co=\"{}\" dir=\"{}\" size=\"{size}\" spot_angle=\"{angle}\" spot_smooth=\"{blend}\" />",
                    v3(l.pos),
                    v3(dir)
                );
            }
            RShape::Directional { dir } => {
                let _ = writeln!(out, "  <light light_type=\"distant\" dir=\"{}\" angle=\"{}\" />", v3(dir), size * 0.05);
            }
        }
        let _ = writeln!(out, "</state>");
    }
}

/// Complete Cycles XML scene description.
pub fn scene_xml(rs: &RenderScene, cfg: &RenderSettings) -> String {
    let mut out = String::from("<cycles>\n");
    let _ = writeln!(out, "<integrator max_bounce=\"{}\" />", cfg.max_depth.max(1) * 2);
    let _ = writeln!(out, "<film transparent=\"{}\" />", cfg.transparent);
    camera(&mut out, rs, cfg);
    let world = rs.background + rs.ambient + rs.sky;
    let _ = writeln!(out, "<background>");
    if let Some(h) = &rs.hdri {
        // Cycles' world is Z-up equirect too; the rotation turns its Z axis.
        let rot = format!("0 0 {}", (-h.rotation).to_radians());
        let _ = writeln!(out, "  <environment_texture name=\"env\" filename=\"{}\" tex_mapping.rotation=\"{rot}\" />", h.path.display());
        let _ = writeln!(out, "  <background name=\"bg\" strength=\"{}\" />", h.strength);
        let _ = writeln!(out, "  <connect from=\"env color\" to=\"bg color\" />");
    } else {
        let _ = writeln!(out, "  <background name=\"bg\" color=\"{}\" strength=\"1\" />", v3(world));
    }
    let _ = writeln!(out, "  <connect from=\"bg background\" to=\"output surface\" />");
    let _ = writeln!(out, "</background>");
    shaders(&mut out, rs);
    meshes(&mut out, rs);
    lights(&mut out, rs);
    out.push_str("</cycles>\n");
    out
}

/// Writes the scene, runs the CLI and loads the result.
pub fn run(rs: &RenderScene, cfg: &RenderSettings, cancel: &AtomicBool) -> Result<FrameBuffer, String> {
    if cfg.cycles_via == CyclesVia::Blender {
        return super::blender::run(rs, cfg, cancel);
    }
    let dir = work_dir()?;
    let (xml, png) = (dir.join("scene.xml"), dir.join("render.png"));
    std::fs::write(&xml, scene_xml(rs, cfg)).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&png);
    let mut cmd = std::process::Command::new(&cfg.cycles_exe);
    cmd.args(["--background", "--samples", &cfg.cycles_samples.to_string()])
        .args(["--device", cfg.cycles_device.id()])
        .args(["--width", &cfg.width.to_string(), "--height", &cfg.height.to_string()])
        .arg("--output")
        .arg(&png)
        .arg(&xml);
    wait(cmd, &cfg.cycles_exe, cancel)?;
    let img = image::open(&png).map_err(|e| format!("Cycles produced no image: {e}"))?;
    Ok(FrameBuffer::from_image(img))
}

/// Per-process scratch folder for exported scenes and results.
pub(super) fn work_dir() -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("minimax_cycles_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Runs `cmd` to completion (killing it on cancel); errors carry the last
/// line of its stderr.
pub(super) fn wait(mut cmd: std::process::Command, exe: &str, cancel: &AtomicBool) -> Result<(), String> {
    let mut child = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start '{exe}': {e}. Set the executable path in Render Setup (F10)."))?;
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            return Err("Render cancelled".into());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if status.success() {
                return Ok(());
            }
            let mut err = String::new();
            if let Some(mut s) = child.stderr.take() {
                let _ = std::io::Read::read_to_string(&mut s, &mut err);
            }
            let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
            return Err(format!("{exe} exited with {status}: {last}"));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
