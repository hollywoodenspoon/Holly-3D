//! Native `.ilex` scene files: the whole [`Scene`] (objects with base
//! objects, modifier stacks, splines, painted colours and textures,
//! materials, lights, environment) as RON. Evaluated meshes are caches and
//! are rebuilt on load. Files from before the Ilex rename (`.mmax`, with a
//! `"MiniMax"` header) open exactly as before.

use crate::scene::Scene;
use std::path::Path;

pub const EXT: &str = "ilex";
/// Scene extensions from before the rename, still opened everywhere.
pub const LEGACY_EXTS: [&str; 1] = ["mmax"];
/// Every extension Open / the command line treat as a scene.
pub const ALL_EXTS: [&str; 2] = [EXT, LEGACY_EXTS[0]];
/// Custom pose files (Easy Pose) and their pre-rename extension.
pub const POSE_EXT: &str = "ilexpose";
pub const LEGACY_POSE_EXTS: [&str; 1] = ["mmaxpose"];
/// The header written into new files, and every header still accepted.
pub const FORMAT: &str = "Ilex";
pub const FORMATS: [&str; 2] = [FORMAT, "MiniMax"];
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
    ron::ser::to_string_pretty(&FileOut { format: FORMAT, version: VERSION, scene }, cfg).map_err(|e| e.to_string())
}

pub fn from_str(s: &str) -> Result<Scene, String> {
    let f: FileIn = ron::from_str(s).map_err(|e| format!("Not a valid Ilex scene: {e}"))?;
    if !FORMATS.contains(&f.format.as_str()) || f.version > VERSION {
        return Err(format!("Unsupported scene file (format {}, version {})", f.format, f.version));
    }
    let mut scene = f.scene;
    for o in scene.objects.values_mut() {
        o.evaluate();
    }
    Ok(scene)
}

/// True for `.ilex` and the legacy `.mmax` (any letter case).
pub fn is_scene_path(path: &Path) -> bool {
    has_ext(path, &ALL_EXTS)
}

/// True for `.ilexpose` and the legacy `.mmaxpose`.
pub fn is_pose_path(path: &Path) -> bool {
    has_ext(path, &[POSE_EXT, LEGACY_POSE_EXTS[0]])
}

fn has_ext(path: &Path, exts: &[&str]) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

pub fn save(scene: &Scene, path: &Path) -> Result<(), String> {
    std::fs::write(path, to_string(scene)?).map_err(|e| format!("Could not write {}: {e}", path.display()))
}

pub fn load(path: &Path) -> Result<Scene, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    from_str(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_legacy_extensions_and_headers() {
        for p in ["a.ilex", "b.MMAX", "dir/c.mmax", "D.Ilex"] {
            assert!(is_scene_path(Path::new(p)), "{p}");
        }
        assert!(!is_scene_path(Path::new("a.obj")) && !is_scene_path(Path::new("ilex")));
        assert!(is_pose_path(Path::new("wave.ilexpose")) && is_pose_path(Path::new("old.mmaxpose")));
        let s = to_string(&Scene::default()).unwrap();
        assert!(s.contains("format: \"Ilex\""), "new files carry the Ilex header");
        assert!(from_str(&s.replacen("\"Ilex\"", "\"MiniMax\"", 1)).is_ok(), "old MiniMax header still loads");
        assert!(from_str(&s.replacen("\"Ilex\"", "\"Other\"", 1)).is_err());
    }
}
