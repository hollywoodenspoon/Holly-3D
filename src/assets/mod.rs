//! CC0 asset packs (Preferences -> Assets): 1K photo textures and realistic
//! models from Poly Haven, low-poly models from Kenney's starter kits.
//! Packs are downloaded into the asset folder (`~/.local/share/ilex/assets/`
//! by default), never into the project, and can be removed again.

pub mod download;
pub mod http;
pub mod photo;
pub mod polyhaven;
#[cfg(test)]
mod tests;
pub mod zip;

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Pack {
    Photos,
    LowPoly,
    Realistic,
}

impl Pack {
    pub const ALL: [Pack; 3] = [Pack::Photos, Pack::LowPoly, Pack::Realistic];

    pub fn label(self) -> &'static str {
        match self {
            Pack::Photos => "📷 Photo Textures",
            Pack::LowPoly => "🏠 Low-poly Models",
            Pack::Realistic => "🗿 Realistic Models",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Pack::Photos => "~60 ground, road, wall, roof, wood, stone and metal materials (Poly Haven, CC0)",
            Pack::LowPoly => "Roads, buildings, vehicles and props from Kenney's starter kits (CC0)",
            Pack::Realistic => "Barrels, crates, furniture, rocks and plants (Poly Haven, CC0)",
        }
    }

    pub fn size_hint(self) -> &'static str {
        match self {
            Pack::Photos => "~150–250 MB",
            Pack::LowPoly => "~20–60 MB",
            Pack::Realistic => "~100–300 MB",
        }
    }

    /// Sub-folder of the asset folder.
    pub fn dir(self) -> &'static str {
        match self {
            Pack::Photos => "photos",
            Pack::LowPoly => "lowpoly",
            Pack::Realistic => "realistic",
        }
    }

    pub fn path(self, folder: &str) -> PathBuf {
        Path::new(folder).join(self.dir())
    }
}

/// File a finished download leaves in the pack folder.
const MARKER: &str = "installed.ron";

/// What a finished download recorded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Installed {
    pub files: u32,
    pub bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackState {
    Missing,
    /// A download was started but did not finish (`files` so far).
    Partial { files: u32 },
    Installed(Installed),
}

fn count_files(dir: &Path) -> u32 {
    std::fs::read_dir(dir).map_or(0, |rd| {
        rd.flatten().map(|e| if e.path().is_dir() { count_files(&e.path()) } else { 1 }).sum()
    })
}

/// Whether `pack` is missing, half downloaded or installed in `folder`.
pub fn state(pack: Pack, folder: &str) -> PackState {
    let dir = pack.path(folder);
    if !dir.is_dir() {
        return PackState::Missing;
    }
    match std::fs::read_to_string(dir.join(MARKER)).ok().and_then(|t| ron::from_str::<Installed>(&t).ok()) {
        Some(i) => PackState::Installed(i),
        None => match count_files(&dir) {
            0 => PackState::Missing,
            files => PackState::Partial { files },
        },
    }
}

pub(crate) fn mark_installed(pack: Pack, folder: &str, i: Installed) -> Result<(), String> {
    let text = ron::to_string(&i).map_err(|e| e.to_string())?;
    std::fs::write(pack.path(folder).join(MARKER), text).map_err(|e| e.to_string())
}

/// `[ Remove ]`: deletes the pack's folder (nothing else).
pub fn remove(pack: Pack, folder: &str) -> Result<(), String> {
    let dir = pack.path(folder);
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(())
}

/// A relative path made safe to join under a pack folder: no absolute
/// parts, no `..`, no empty names.
pub fn safe_rel(path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => return None,
            p if p.contains(':') => return None,
            p => out.push(p),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

/// Where the packs come from. The defaults are the public hosts; tests
/// point them at a local server.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Sources {
    /// Poly Haven API root.
    pub api: String,
    /// Zip archives with low-poly models (`.glb` / `.gltf` files are kept).
    pub lowpoly_zips: Vec<String>,
    /// Photo texture resolution: "1k" or "2k".
    pub resolution: String,
    /// Photo materials per category.
    pub photos_per_group: usize,
    /// Realistic models in the pack.
    pub model_count: usize,
}

impl Default for Sources {
    fn default() -> Self {
        let kit = |name: &str| format!("https://github.com/KenneyNL/{name}/archive/refs/heads/main.zip");
        Self {
            api: "https://api.polyhaven.com".into(),
            lowpoly_zips: ["Starter-Kit-City-Builder", "Starter-Kit-Racing", "Starter-Kit-3D-Platformer", "Starter-Kit-FPS"].map(kit).to_vec(),
            resolution: "1k".into(),
            photos_per_group: 9,
            model_count: 40,
        }
    }
}
