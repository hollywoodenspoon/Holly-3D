//! Import PBR Texture Folder: matches image files to Maps rollout channels by
//! file-name keywords (`*_albedo.png`, `*_Roughness.jpg`, ...).

use super::Material;
use super::maps::{MapChannel, MapNode};
use super::texture::{TexHandle, Texture};
use std::path::{Path, PathBuf};

const IMAGE_EXT: [&str; 7] = ["png", "jpg", "jpeg", "exr", "hdr", "tga", "bmp"];

/// Channel a file name belongs to (longest keyword wins, so `basecolor`
/// beats `color` and `metalness` beats `metal`).
pub fn channel_of(file_name: &str) -> Option<MapChannel> {
    let stem = file_name.rsplit_once('.').map_or(file_name, |(s, _)| s).to_ascii_lowercase();
    let tokens: Vec<&str> = stem.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()).collect();
    let mut best: Option<(usize, MapChannel)> = None;
    for ch in MapChannel::ALL {
        for kw in ch.keywords() {
            // Whole token match, or a long keyword embedded in a CamelCase-free stem.
            let hit = tokens.iter().any(|t| t == kw) || (kw.len() >= 5 && stem.contains(kw));
            if hit && best.is_none_or(|(l, _)| kw.len() > l) {
                best = Some((kw.len(), ch));
            }
        }
    }
    best.map(|(_, c)| c)
}

/// Image files in `dir` matched to channels (first file per channel, sorted by name).
pub fn scan_folder(dir: &Path) -> Result<Vec<(MapChannel, PathBuf)>, String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| IMAGE_EXT.contains(&e.to_ascii_lowercase().as_str())))
        .collect();
    files.sort();
    let mut out: Vec<(MapChannel, PathBuf)> = vec![];
    for f in files {
        let Some(name) = f.file_name().and_then(|n| n.to_str()) else { continue };
        if let Some(ch) = channel_of(name)
            && !out.iter().any(|(c, _)| *c == ch)
        {
            out.push((ch, f));
        }
    }
    out.sort_by_key(|(c, _)| *c as usize);
    Ok(out)
}

/// Loads a bitmap map node from disk.
pub fn bitmap_node(path: &Path) -> Result<MapNode, String> {
    let s = path.to_string_lossy().to_string();
    let tex = Texture::load(&s)?;
    Ok(MapNode::Bitmap { path: s, tex: Some(TexHandle::new(tex)) })
}

/// Fills the material's map slots from a folder; returns the channels loaded.
pub fn import_folder(m: &mut Material, dir: &Path) -> Result<Vec<MapChannel>, String> {
    let found = scan_folder(dir)?;
    if found.is_empty() {
        return Err(format!("No PBR textures recognised in {}", dir.display()));
    }
    let mut done = vec![];
    for (ch, path) in found {
        let node = bitmap_node(&path)?;
        let slot = m.map_mut(ch);
        (slot.node, slot.enabled, slot.amount) = (Some(node), true, 100.0);
        done.push(ch);
    }
    // The maps carry the look: neutral scalars so they are not tinted twice.
    if done.contains(&MapChannel::BaseColor) {
        m.base_color = [1.0; 3];
    }
    if done.contains(&MapChannel::Emission) {
        m.emissive_strength = m.emissive_strength.max(1.0);
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_pick_the_most_specific_channel() {
        assert_eq!(channel_of("Rock_BaseColor.png"), Some(MapChannel::BaseColor));
        assert_eq!(channel_of("rock_albedo.jpg"), Some(MapChannel::BaseColor));
        assert_eq!(channel_of("T_Metalness.png"), Some(MapChannel::Metallic));
        assert_eq!(channel_of("wood_rough.exr"), Some(MapChannel::Roughness));
        assert_eq!(channel_of("wood_nrm.png"), Some(MapChannel::Bump));
        assert_eq!(channel_of("leaf_opacity.png"), Some(MapChannel::Opacity));
        assert_eq!(channel_of("lamp_emissive.png"), Some(MapChannel::Emission));
        assert_eq!(channel_of("readme.png"), None);
    }

    #[test]
    fn folder_import_fills_slots() {
        let dir = std::env::temp_dir().join(format!("ilex_pbr_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 100, 50, 255]));
        for name in ["crate_albedo.png", "crate_roughness.png", "crate_metallic.png", "notes.txt"] {
            if name.ends_with(".png") {
                img.save(dir.join(name)).unwrap();
            } else {
                std::fs::write(dir.join(name), "x").unwrap();
            }
        }
        let mut m = Material::default();
        let done = import_folder(&mut m, &dir).unwrap();
        assert_eq!(done, vec![MapChannel::BaseColor, MapChannel::Metallic, MapChannel::Roughness]);
        let (n, a) = m.active_map(MapChannel::BaseColor).unwrap();
        assert_eq!(a, 1.0);
        let s = n.sample(bevy::math::Vec2::splat(0.5), bevy::math::Vec3::ZERO);
        assert!((s.x - 200.0 / 255.0).abs() < 1e-3);
        assert!(m.active_map(MapChannel::Bump).is_none());
        std::fs::remove_dir_all(&dir).ok();
        assert!(import_folder(&mut m, &dir).is_err());
    }
}
