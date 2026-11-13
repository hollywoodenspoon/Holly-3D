//! 1-click Turntable & isometric Sprite-Sheet renderer: N frames orbiting
//! 360° around the selection (numbered PNG sequence), or a 4 / 8-direction
//! tactics sprite set (N, NE, E, SE, S, SW, W, NW) at an isometric pitch,
//! packed into one atlas PNG + `.json` frame metadata.

use super::framebuffer::FrameBuffer;
use super::scene_build::RenderScene;
use super::settings::RenderSettings;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SheetMode {
    #[default]
    Turntable,
    Sprites,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SheetSettings {
    pub mode: SheetMode,
    /// Turntable frames (8..=72).
    pub frames: u32,
    /// Sprite directions: 4 or 8.
    pub dirs: u32,
    /// Isometric camera pitch below the horizon, degrees.
    pub pitch: f32,
    /// Sprite cell size in pixels (square).
    pub cell: u32,
    pub transparent: bool,
    pub outline: bool,
    /// Frame the selection (else the whole scene).
    pub selection_only: bool,
    pub folder: String,
    pub name: String,
}

impl Default for SheetSettings {
    fn default() -> Self {
        Self {
            mode: SheetMode::Turntable,
            frames: 24,
            dirs: 8,
            pitch: 30.0,
            cell: 256,
            transparent: true,
            outline: true,
            selection_only: true,
            folder: "renders".into(),
            name: "turntable".into(),
        }
    }
}

/// Isometric pitch presets: game 2:1 (30°), true isometric, top-down 45°.
pub const PITCHES: [(f32, &str); 3] = [(30.0, "30° (2:1 Game)"), (35.264, "35.264° (True Isometric)"), (45.0, "45° (Top-Down)")];
pub const DIRS8: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

/// Compass names of the sprite directions, clockwise from N.
pub fn dir_names(dirs: u32) -> Vec<&'static str> {
    if dirs == 4 { vec!["N", "E", "S", "W"] } else { DIRS8.to_vec() }
}

/// Camera yaw that shows a model (front facing -Y, the Front view) facing
/// compass direction `deg` on screen (N = up, E = right, clockwise).
pub fn facing_yaw(deg: f32) -> f32 {
    (deg - 90.0).to_radians()
}

/// One named camera per frame / direction around the bounds `min..max`.
pub fn views(s: &SheetSettings, min: Vec3, max: Vec3, base: &ViewCamera) -> Vec<(String, ViewCamera)> {
    let mut cam = *base;
    cam.frame(min, max);
    match s.mode {
        SheetMode::Turntable => {
            let n = s.frames.clamp(8, 72);
            (0..n)
                .map(|i| {
                    let mut c = cam;
                    c.yaw = base.yaw + std::f32::consts::TAU * i as f32 / n as f32;
                    (format!("{}_{:04}", s.name, i), c)
                })
                .collect()
        }
        SheetMode::Sprites => {
            let names = dir_names(s.dirs);
            names
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    let mut c = cam;
                    (c.kind, c.pitch) = (ViewKind::User, -s.pitch.to_radians());
                    c.yaw = facing_yaw(360.0 * i as f32 / names.len() as f32);
                    (n.to_string(), c)
                })
                .collect()
        }
    }
}

/// Atlas grid (columns, rows) for `n` cells: one row for up to 8 cells.
pub fn atlas_grid(n: u32) -> (u32, u32) {
    let cols = n.clamp(1, 8);
    (cols, n.div_ceil(cols))
}

/// Atlas `.json` frame metadata (TexturePacker-style hash).
pub fn atlas_json(names: &[String], cell: u32, image: &str) -> String {
    let (cols, rows) = atlas_grid(names.len() as u32);
    let frames: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let (x, y) = ((i as u32 % cols) * cell, (i as u32 / cols) * cell);
            format!("    \"{n}\": {{ \"frame\": {{ \"x\": {x}, \"y\": {y}, \"w\": {cell}, \"h\": {cell} }} }}")
        })
        .collect();
    format!(
        "{{\n  \"frames\": {{\n{}\n  }},\n  \"meta\": {{ \"image\": \"{image}\", \"size\": {{ \"w\": {}, \"h\": {} }}, \"format\": \"RGBA8888\" }}\n}}\n",
        frames.join(",\n"),
        cols * cell,
        rows * cell
    )
}

/// Renders one camera to an RGBA8 frame (the F10 renderer, synchronously).
fn frame(rs: &mut RenderScene, cfg: &RenderSettings, cam: ViewCamera, cancel: &AtomicBool) -> Vec<u8> {
    rs.camera = cam;
    let region = [0, 0, cfg.width, cfg.height];
    let px = match cfg.path.integrator {
        super::pathtrace::settings::Integrator::Classic => super::job::raytrace_region(rs, cfg, region, cancel, |_, _| {}),
        super::pathtrace::settings::Integrator::PathTracer => super::pathtrace::run_mix(rs, cfg, region, cancel, |_, _| {}).0,
    };
    let mut fb = FrameBuffer::new(cfg.width, cfg.height);
    (fb.plain_bg, fb.transparent) = (rs.hdri.is_none(), cfg.transparent);
    let (display, hdr): (Vec<_>, Vec<_>) = px.iter().map(|p| (p.display, p.hdr)).unzip();
    fb.put_block(0, 0, cfg.width, &display, &hdr);
    fb.regrade(&cfg.post.tone, &cfg.post.grade);
    fb.rgba8()
}

/// Renders every view and writes the PNG sequence (turntable) or the atlas
/// PNG + JSON (sprites). `done` counts finished frames.
pub fn render_sheet(
    mut rs: RenderScene,
    cfg: &RenderSettings,
    s: &SheetSettings,
    views: Vec<(String, ViewCamera)>,
    cancel: &AtomicBool,
    done: &AtomicU32,
) -> Result<String, String> {
    let mut cfg = cfg.clone();
    (cfg.transparent, cfg.outlines) = (s.transparent || cfg.transparent, s.outline || cfg.outlines);
    if s.mode == SheetMode::Sprites {
        (cfg.width, cfg.height) = (s.cell.max(16), s.cell.max(16));
    }
    std::fs::create_dir_all(&s.folder).map_err(|e| format!("{}: {e}", s.folder))?;
    let dir = std::path::Path::new(&s.folder);
    let (w, h) = (cfg.width, cfg.height);
    let (cols, rows) = atlas_grid(views.len() as u32);
    let mut atlas = image::RgbaImage::new(cols * w, rows * h);
    let names: Vec<String> = views.iter().map(|(n, _)| n.clone()).collect();
    for (i, (name, cam)) in views.into_iter().enumerate() {
        let rgba = frame(&mut rs, &cfg, cam, cancel);
        if cancel.load(Ordering::Relaxed) {
            return Err("Turntable cancelled".into());
        }
        let img = image::RgbaImage::from_raw(w, h, rgba).ok_or("frame size mismatch")?;
        match s.mode {
            SheetMode::Turntable => img.save(dir.join(format!("{name}.png"))).map_err(|e| e.to_string())?,
            SheetMode::Sprites => image::imageops::replace(&mut atlas, &img, i64::from(i as u32 % cols * w), i64::from(i as u32 / cols * h)),
        }
        done.fetch_add(1, Ordering::Relaxed);
    }
    if s.mode == SheetMode::Turntable {
        return Ok(format!("Saved {} turntable frames to {}", names.len(), dir.display()));
    }
    let png = format!("{}.png", s.name);
    atlas.save(dir.join(&png)).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.json", s.name)), atlas_json(&names, w, &png)).map_err(|e| e.to_string())?;
    Ok(format!("Saved {} sprite atlas ({}x{}) + JSON to {}", names.len(), cols * w, rows * h, dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_direction_sheet_layout() {
        let s = SheetSettings { mode: SheetMode::Sprites, dirs: 8, pitch: 35.264, ..Default::default() };
        let v = views(&s, Vec3::splat(-1.0), Vec3::ONE, &ViewCamera::new(ViewKind::Perspective));
        let names: Vec<&str> = v.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, DIRS8);
        // S looks from the model's front (camera at -Y), E shows its front to the right.
        let (s_cam, e_cam) = (&v[4].1, &v[2].1);
        assert!(s_cam.eye().y < -1.0 && s_cam.is_ortho(), "S: camera in front");
        assert!((e_cam.right() - Vec3::NEG_Y).length() < 1e-5, "E: model front points screen-right");
        assert!(v.iter().all(|(_, c)| (c.pitch + 35.264f32.to_radians()).abs() < 1e-5 && c.target == Vec3::ZERO));
        assert_eq!(atlas_grid(8), (8, 1));
        assert_eq!(atlas_grid(4), (4, 1));
        assert_eq!(atlas_grid(24), (8, 3));
        let json = atlas_json(&names.iter().map(|n| n.to_string()).collect::<Vec<_>>(), 64, "hero.png");
        assert!(json.contains("\"SE\": { \"frame\": { \"x\": 192, \"y\": 0, \"w\": 64, \"h\": 64 } }"), "{json}");
        assert!(json.contains("\"size\": { \"w\": 512, \"h\": 64 }"));
        assert_eq!(dir_names(4), ["N", "E", "S", "W"]);
    }

    #[test]
    fn turntable_orbits_a_full_circle() {
        let s = SheetSettings { frames: 12, ..Default::default() };
        let base = ViewCamera::new(ViewKind::Perspective);
        let v = views(&s, Vec3::splat(-1.0), Vec3::ONE, &base);
        assert_eq!(v.len(), 12);
        assert_eq!(v[3].0, "turntable_0003");
        let step = v[1].1.yaw - v[0].1.yaw;
        assert!((step - std::f32::consts::TAU / 12.0).abs() < 1e-5);
        let d0 = v[0].1.eye().distance(Vec3::ZERO);
        assert!(v.iter().all(|(_, c)| (c.eye().distance(Vec3::ZERO) - d0).abs() < 1e-3), "constant orbit radius");
        assert_eq!(views(&SheetSettings { frames: 200, ..s }, Vec3::ZERO, Vec3::ONE, &base).len(), 72, "clamped");
    }

    #[test]
    fn sprite_sheet_renders_to_disk() {
        let dir = std::env::temp_dir().join(format!("mmx_sheet_{}", std::process::id()));
        let mut scene = crate::scene::Scene::default();
        let id = scene.add(crate::scene::BaseObject::Primitive(crate::primitives::Primitive::default_of(crate::primitives::PrimitiveKind::Box)), Vec3::ZERO);
        scene.objects[id].evaluate();
        let cam = ViewCamera::new(ViewKind::Perspective);
        let rs = RenderScene::build(&scene, cam, true);
        let s = SheetSettings { mode: SheetMode::Sprites, dirs: 4, cell: 16, folder: dir.to_string_lossy().into(), name: "box".into(), ..Default::default() };
        let mut cfg = RenderSettings { aa: 1, ..Default::default() };
        cfg.path.integrator = super::super::pathtrace::settings::Integrator::Classic;
        let v = views(&s, Vec3::splat(-0.5), Vec3::splat(0.5), &cam);
        let n = AtomicU32::new(0);
        render_sheet(rs, &cfg, &s, v, &AtomicBool::new(false), &n).unwrap();
        assert_eq!(n.load(Ordering::Relaxed), 4);
        let img = image::open(dir.join("box.png")).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (64, 16));
        assert!(img.pixels().any(|p| p.0[3] == 0) && img.pixels().any(|p| p.0[3] == 255), "transparent background around the box");
        assert!(std::fs::read_to_string(dir.join("box.json")).unwrap().contains("\"W\""));
        let _ = std::fs::remove_dir_all(dir);
    }
}
