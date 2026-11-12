//! `.ilexpose` pose files (RON, `format: "Ilex"`). Legacy `.mmaxpose`
//! files and the `"MiniMax"` header load too.

use super::pose::Pose;
use crate::io::scene_file::{FORMAT, FORMATS, POSE_EXT};
use std::path::{Path, PathBuf};

pub const VERSION: u32 = 1;

#[derive(serde::Serialize)]
struct Out<'a> {
    format: &'a str,
    version: u32,
    pose: &'a Pose,
}

#[derive(serde::Deserialize)]
struct In {
    format: String,
    version: u32,
    pose: Pose,
}

pub fn to_string(pose: &Pose) -> Result<String, String> {
    let cfg = ron::ser::PrettyConfig::new().indentor(" ").compact_arrays(true);
    ron::ser::to_string_pretty(&Out { format: FORMAT, version: VERSION, pose }, cfg).map_err(|e| e.to_string())
}

pub fn from_str(s: &str) -> Result<Pose, String> {
    let f: In = ron::from_str(s).map_err(|e| format!("Not a valid Ilex pose: {e}"))?;
    if !FORMATS.contains(&f.format.as_str()) || f.version > VERSION {
        return Err(format!("Unsupported pose file (format {}, version {})", f.format, f.version));
    }
    Ok(f.pose)
}

/// Writes `pose`, forcing the `.ilexpose` extension; returns the path.
pub fn save(pose: &Pose, path: &Path) -> Result<PathBuf, String> {
    let path = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(POSE_EXT)) { path.to_path_buf() } else { path.with_extension(POSE_EXT) };
    std::fs::write(&path, to_string(pose)?).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

pub fn load(path: &Path) -> Result<Pose, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    from_str(&s)
}
