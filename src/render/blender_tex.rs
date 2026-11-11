//! Texture export for the Cycles-through-Blender bridge: materials with
//! maps or a procedural surface get their base colour, occlusion /
//! roughness / metallic and normal maps baked to PNGs the loader wires
//! into the Principled BSDF.

use crate::materials::bake::{bake_base, bake_metal_rough, bake_normal};
use crate::materials::proc_engine::surface::COOK_SIZE;
use crate::materials::{Material, Texture};
use serde_json::{Value, json};
use std::path::Path;

fn save(t: &Texture, path: &Path) -> Result<String, String> {
    image::save_buffer(path, &t.bytes(), t.width, t.height, image::ColorType::Rgba8).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Bakes material `i`'s maps into `dir`; returns `{ "base", "orm",
/// "normal" }` with the PNG paths (null for channels with no map).
pub fn export(m: &Material, i: usize, dir: &Path) -> Result<Value, String> {
    let size = if m.proc.active() { COOK_SIZE.max(256) } else { 256 };
    let write = |t: Option<Texture>, kind: &str| t.map(|t| save(&t, &dir.join(format!("mat{i}_{kind}.png")))).transpose();
    Ok(json!({
        "base": write(bake_base(m, size), "base")?,
        "orm": write(bake_metal_rough(m, size), "orm")?,
        "normal": write(bake_normal(m, size), "normal")?,
    }))
}

/// Adds a `textures` entry to every material of a scene description.
pub fn attach(scene: &mut Value, materials: &[Material], dir: &Path) -> Result<(), String> {
    for (i, m) in materials.iter().enumerate() {
        scene["materials"][i]["textures"] = export(m, i, dir)?;
    }
    Ok(())
}
