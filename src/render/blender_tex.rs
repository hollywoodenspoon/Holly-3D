//! Texture export for the Cycles-through-Blender bridge: materials with
//! maps or a procedural surface get their base colour, occlusion /
//! roughness / metallic and normal maps baked to PNGs the loader wires
//! into the Principled BSDF.

use crate::materials::bake::{bake_base, bake_metal_rough, bake_normal};
use crate::materials::proc_engine::surface::COOK_SIZE;
use crate::materials::{Material, Texture};
use crate::paint::layers::composite::Composite;
use serde_json::{Value, json};
use std::path::Path;

fn save(t: &Texture, path: &Path) -> Result<String, String> {
    image::save_buffer(path, &t.bytes(), t.width, t.height, image::ColorType::Rgba8).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Bakes material `i`'s maps into `dir`; returns `{ "base", "orm",
/// "normal" }` with the PNG paths (null for channels with no map).
/// Painted materials export their layer composite (with `"emissive"`);
/// World / Triplanar ones add `"triplanar": { size, blend }`.
pub fn export(m: &Material, i: usize, dir: &Path) -> Result<Value, String> {
    let write = |t: Option<&Texture>, kind: &str| t.map(|t| save(t, &dir.join(format!("mat{i}_{kind}.png")))).transpose();
    if m.paint.is_some() {
        let c = Composite::build(m, None);
        return Ok(json!({
            "base": write(Some(&c.color), "base")?,
            "orm": write(Some(&c.orm), "orm")?,
            "normal": write(Some(&c.normal), "normal")?,
            "emissive": write(Some(&c.emissive), "emissive")?,
            "emissive_peak": c.peak,
        }));
    }
    let size = if m.proc.active() { COOK_SIZE.max(256) } else { 256 };
    let mut tex = json!({
        "base": write(bake_base(m, size).as_ref(), "base")?,
        "orm": write(bake_metal_rough(m, size).as_ref(), "orm")?,
        "normal": write(bake_normal(m, size).as_ref(), "normal")?,
    });
    if m.projection.on() {
        tex["triplanar"] = json!({ "size": m.projection.size, "blend": m.projection.blend });
    }
    Ok(tex)
}

/// Adds a `textures` entry to every material of a scene description.
pub fn attach(scene: &mut Value, materials: &[Material], dir: &Path) -> Result<(), String> {
    for (i, m) in materials.iter().enumerate() {
        scene["materials"][i]["textures"] = export(m, i, dir)?;
    }
    Ok(())
}
