//! Photo materials: the installed photo-texture pack as Material Library
//! items. Each texture folder holds `diff`, `nor` and `rough` images; the
//! pack's index says which shelf each belongs on.

use super::polyhaven::{Group, Pick};
use crate::materials::{MapChannel, MapNode, MapSlot, Material, TexHandle, Texture};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Index file the downloader writes into the photo pack folder.
pub const INDEX: &str = "index.ron";

/// One installed photo texture.
#[derive(Clone, Debug)]
pub struct PhotoMat {
    /// "📷 Rock Wall 10" (leaked once per scan, so names can be `&'static`).
    pub name: &'static str,
    pub group: Group,
    pub dir: PathBuf,
}

static PHOTOS: RwLock<Vec<PhotoMat>> = RwLock::new(vec![]);

/// The photo textures installed in `folder` (the asset folder), by name.
pub fn scan(folder: &str) -> Vec<PhotoMat> {
    let dir = super::Pack::Photos.path(folder);
    let picks: Vec<Pick> = std::fs::read_to_string(dir.join(INDEX)).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
    let mut found: Vec<PhotoMat> = picks
        .into_iter()
        .filter(|p| super::safe_rel(&p.slug).is_some() && image_in(&dir.join(&p.slug), "diff").is_some())
        .map(|p| PhotoMat { name: Box::leak(format!("📷 {}", p.name).into_boxed_str()), group: p.group, dir: dir.join(&p.slug) })
        .collect();
    found.sort_by_key(|p| p.name);
    found.dedup_by_key(|p| p.name);
    found
}

/// Makes the photo pack in `folder` available to the Material Library
/// (call at startup and after a download or removal). Returns the count.
pub fn rescan(folder: &str) -> usize {
    let found = scan(folder);
    let n = found.len();
    *PHOTOS.write().unwrap_or_else(|e| e.into_inner()) = found;
    n
}

/// Every installed photo texture.
pub fn all() -> Vec<PhotoMat> {
    PHOTOS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// `dir/<stem>.jpg` or `.png`, whichever exists.
fn image_in(dir: &Path, stem: &str) -> Option<PathBuf> {
    ["jpg", "png"].iter().map(|ext| dir.join(format!("{stem}.{ext}"))).find(|p| p.is_file())
}

fn bitmap(path: &Path) -> Option<MapSlot> {
    let p = path.to_string_lossy().into_owned();
    let tex = Texture::load(&p).ok()?;
    Some(MapSlot { enabled: true, amount: 100.0, node: Some(MapNode::Bitmap { path: p, tex: Some(TexHandle::new(tex)) }) })
}

impl PhotoMat {
    /// The material: photo base colour, normal map and roughness map.
    pub fn material(&self) -> Material {
        let mut m = Material { roughness: 0.8, ..Material::named(self.name.trim_start_matches("📷 ")) };
        m.metallic = if self.group == Group::Metals { 0.8 } else { 0.0 };
        for (stem, ch) in [("diff", MapChannel::BaseColor), ("nor", MapChannel::Bump), ("rough", MapChannel::Roughness)] {
            if let Some(slot) = image_in(&self.dir, stem).and_then(|p| bitmap(&p)) {
                *m.map_mut(ch) = slot;
            }
        }
        m
    }
}
