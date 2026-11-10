//! Native `.mmax` scene files: the whole [`Scene`] (objects with base
//! objects, modifier stacks, splines, painted colours and textures,
//! materials, lights, environment) as RON. Evaluated meshes are caches and
//! are rebuilt on load.

use crate::scene::Scene;
use std::path::Path;

pub const EXT: &str = "mmax";
/// Bumped on incompatible format changes.
pub const VERSION: u32 = 1;

#[derive(serde::Serialize)]
struct FileOut<'a> {
    format: &'a str,
    version: u32,
    scene: &'a Scene,
}

#[derive(serde::Deserialize)]
struct FileIn {
    format: String,
    version: u32,
    scene: Scene,
}

pub fn to_string(scene: &Scene) -> Result<String, String> {
    let cfg = ron::ser::PrettyConfig::new().indentor(" ").compact_arrays(true);
    ron::ser::to_string_pretty(&FileOut { format: "MiniMax", version: VERSION, scene }, cfg).map_err(|e| e.to_string())
}

pub fn from_str(s: &str) -> Result<Scene, String> {
    let f: FileIn = ron::from_str(s).map_err(|e| format!("Not a valid MiniMax scene: {e}"))?;
    if f.format != "MiniMax" || f.version > VERSION {
        return Err(format!("Unsupported scene file (format {}, version {})", f.format, f.version));
    }
    let mut scene = f.scene;
    for o in scene.objects.values_mut() {
        o.evaluate();
    }
    Ok(scene)
}

pub fn save(scene: &Scene, path: &Path) -> Result<(), String> {
    std::fs::write(path, to_string(scene)?).map_err(|e| format!("Could not write {}: {e}", path.display()))
}

pub fn load(path: &Path) -> Result<Scene, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    from_str(&s)
}
