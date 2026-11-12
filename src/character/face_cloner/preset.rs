//! Cloned Face Presets: a cloned face (its head sliders, fine warp and
//! photo skin) saved under a name in `~/.local/share/ilex/faces/`, so it
//! can be put on another character, or back on this one later.

use super::clone::{owned_keys, reset};
use super::FaceClone;
use crate::character::params::CharacterParams;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One saved face.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FacePreset {
    pub name: String,
    /// The cloner's head sliders (only the keys it owns).
    pub micro: BTreeMap<String, f32>,
    pub face: FaceClone,
}

pub fn dir() -> PathBuf {
    crate::prefs::data_dir().join("faces")
}

/// A file name for `name`: letters, digits, spaces, `-` and `_` kept.
fn stem(name: &str) -> String {
    let s: String = name.trim().chars().map(|c| if c.is_alphanumeric() || " -_".contains(c) { c } else { '_' }).collect();
    if s.is_empty() { "Face".into() } else { s }
}

fn file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.ron", stem(name)))
}

/// The face of `p` as a preset named `name` (None when `p` has no cloned face).
pub fn of(name: &str, p: &CharacterParams) -> Option<FacePreset> {
    if p.face.views == 0 {
        return None;
    }
    let keys = owned_keys();
    let micro = p.micro.iter().filter(|(k, _)| keys.contains(k)).map(|(k, v)| (k.clone(), *v)).collect();
    Some(FacePreset { name: name.trim().to_string(), micro, face: p.face.clone() })
}

/// `p` wearing the preset's face (its own face sliders and warp replaced).
pub fn apply(f: &FacePreset, p: &CharacterParams) -> CharacterParams {
    let mut q = reset(p);
    q.micro.extend(f.micro.iter().map(|(k, v)| (k.clone(), *v)));
    q.face = f.face.clone();
    q
}

pub fn save_in(dir: &Path, f: &FacePreset) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let text = ron::to_string(f).map_err(|e| e.to_string())?;
    let path = file(dir, &f.name);
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

pub fn load_in(dir: &Path, name: &str) -> Result<FacePreset, String> {
    let path = file(dir, name);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn delete_in(dir: &Path, name: &str) -> Result<(), String> {
    let path = file(dir, name);
    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The saved presets' names, sorted.
pub fn list_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ron"))
        .filter_map(|p| Some(p.file_stem()?.to_str()?.to_string()))
        .collect();
    v.sort_by_key(|s| s.to_lowercase());
    v
}

pub fn save(f: &FacePreset) -> Result<PathBuf, String> {
    save_in(&dir(), f)
}

pub fn load(name: &str) -> Result<FacePreset, String> {
    load_in(&dir(), name)
}

pub fn delete(name: &str) -> Result<(), String> {
    delete_in(&dir(), name)
}

pub fn list() -> Vec<String> {
    list_in(&dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::texture::{TexHandle, Texture};

    #[test]
    fn a_face_saves_loads_onto_another_character_and_deletes() {
        let dir = std::env::temp_dir().join(format!("ilex-faces-test-{}", std::process::id()));
        let key = owned_keys()[0].clone();
        let mut p = CharacterParams::default();
        assert!(of("x", &p).is_none(), "no cloned face, nothing to save");
        p.micro.insert(key.clone(), 0.4);
        p.micro.insert("not-a-face-slider".into(), 0.9);
        p.face = FaceClone { deltas: vec![(7, [0.01, 0.0, -0.02])], views: 12, texture: Some(TexHandle::new(Texture::new(8, 8, [180, 130, 110, 255]))), ..FaceClone::default() };
        let f = of("Holly / test", &p).unwrap();
        assert!(!f.micro.contains_key("not-a-face-slider"));
        save_in(&dir, &f).unwrap();
        assert_eq!(list_in(&dir), vec!["Holly _ test".to_string()]);
        let back = load_in(&dir, "Holly / test").unwrap();
        assert_eq!((back.micro.clone(), back.face.deltas.clone(), back.face.views), (f.micro.clone(), f.face.deltas.clone(), 12));
        assert_eq!(back.face.texture.as_ref().unwrap().tex.data, f.face.texture.as_ref().unwrap().tex.data);
        // On another character: its other sliders stay, its old face goes.
        let mut other = CharacterParams::default();
        other.micro.insert("not-a-face-slider".into(), 0.2);
        other.micro.insert(owned_keys()[1].clone(), -0.5);
        let q = apply(&back, &other);
        assert_eq!(q.micro.get(&key), Some(&0.4));
        assert_eq!(q.micro.get("not-a-face-slider"), Some(&0.2));
        assert!(!q.micro.contains_key(&owned_keys()[1]));
        assert_eq!(q.face.views, 12);
        delete_in(&dir, "Holly / test").unwrap();
        assert!(list_in(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
