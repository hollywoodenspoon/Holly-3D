//! Keeps a character's Character Pack in step with its MakeHuman assets:
//! each fitted piece gets a textured sub-material at its Material ID, the
//! picked skin (or the Face Cloner's photo skin) and eye colour texture
//! IDs 1 and 2. A sub-material whose
//! name already matches is left alone, so edits made in the Material
//! Editor survive later slider changes.

use super::MhAssets;
use crate::character::makehuman::library::{self, Category, MhMat};
use crate::character::params::ids;
use crate::materials::maps::{MapChannel, MapNode};
use crate::materials::multi::{skin_material, MultiSub, SubMaterial};
use crate::materials::texture::TexHandle;
use crate::materials::Material;

/// Name prefixes of the sub-materials this module owns.
const PREFIXES: [&str; 6] = ["Eyebrows: ", "Eyelashes: ", "Teeth: ", "Tongue: ", "Hair: ", "Clothes: "];

fn prefix(cat: Category) -> &'static str {
    match cat {
        Category::Eyebrows => PREFIXES[0],
        Category::Eyelashes => PREFIXES[1],
        Category::Teeth => PREFIXES[2],
        Category::Tongue => PREFIXES[3],
        Category::Hair => PREFIXES[4],
        Category::Clothes | Category::Proxy => PREFIXES[5],
    }
}

fn set_base_map(m: &mut Material, key: &str, tex: TexHandle) {
    let has_alpha = tex.tex.data.iter().any(|p| p[3] < 250);
    let slot = m.map_mut(MapChannel::BaseColor);
    (slot.node, slot.enabled, slot.amount) = (Some(MapNode::Bitmap { path: format!("makehuman:{key}"), tex: Some(tex) }), true, 100.0);
    m.base_color = [1.0; 3];
    if has_alpha {
        // Just under 1 so the viewport blends the texture's alpha.
        m.opacity = 0.999;
    }
}

/// A material from a `.mhmat` and its texture.
pub fn from_mhmat(name: String, mm: &MhMat, key: &str, tex: Option<TexHandle>) -> Material {
    let mut m = Material::named(name);
    m.base_color = mm.diffuse_color;
    m.roughness = (1.0 - mm.shininess).clamp(0.3, 0.9);
    if let Some(t) = tex {
        set_base_map(&mut m, key, t);
    }
    if mm.transparent {
        m.opacity = m.opacity.min(0.999);
    }
    m
}

fn put(ms: &mut MultiSub, id: u16, name: &str, make: impl FnOnce() -> Material) {
    let mut mat = make();
    mat.name = name.to_string();
    let s = SubMaterial { id, name: name.to_string(), enabled: true, mat };
    match ms.subs.iter_mut().find(|s| s.id == id) {
        Some(cur) if cur.name == name => {}
        Some(cur) => *cur = s,
        None => ms.subs.push(s),
    }
}

/// Name of the skin sub-material that carries a Face Cloner photo skin.
pub const CLONED_SKIN: &str = "Skin: Cloned Face";

/// Puts the photo skin `tex` on ID 1. A re-bake only swaps the bitmap, so
/// edits made to the material in the Material Editor survive.
fn put_cloned_skin(ms: &mut MultiSub, tex: &TexHandle) {
    put(ms, ids::SKIN, CLONED_SKIN, skin_material);
    if let Some(cur) = ms.subs.iter_mut().find(|s| s.id == ids::SKIN) {
        let same = matches!(&cur.mat.map_mut(MapChannel::BaseColor).node, Some(MapNode::Bitmap { tex: Some(t), .. }) if t == tex);
        if !same {
            set_base_map(&mut cur.mat, "", tex.clone());
            cur.mat.map_mut(MapChannel::BaseColor).node = Some(MapNode::Bitmap { path: "face-cloner:photo skin".into(), tex: Some(tex.clone()) });
        }
    }
}

/// Brings the Character Pack `ms` up to date with `mh` and, when the
/// character has one, its Face Cloner photo skin `face_skin`.
pub fn sync(mh: &MhAssets, face_skin: Option<&TexHandle>, ms: &mut MultiSub) {
    let pieces = mh.pieces();
    // Drop pieces that were taken off (only the ones this module made).
    ms.subs.retain(|s| s.id < ids::EYEBROWS || !PREFIXES.iter().any(|p| s.name.starts_with(p)) || pieces.iter().any(|(_, _, id)| *id == s.id));
    for (cat, name, id) in pieces {
        let Some(it) = library::item(cat, name) else { continue };
        let label = format!("{}{}", prefix(cat), library::pretty(name));
        let key = format!("{}/{}", it.folder, it.mat.diffuse_texture.clone().unwrap_or_default());
        put(ms, id, &label, || from_mhmat(label.clone(), &it.mat, &key, it.texture()));
    }
    match &mh.skin {
        _ if face_skin.is_some() => put_cloned_skin(ms, face_skin.unwrap()),
        Some(s) => {
            let label = format!("Skin: {}", library::pretty(s));
            put(ms, ids::SKIN, &label, || {
                let mut m = skin_material();
                let mm = library::material(&format!("skins/{s}/{s}.mhmat")).unwrap_or_default();
                if let Some(t) = mm.diffuse_texture.as_ref() {
                    let key = format!("skins/{s}/{t}");
                    if let Some(tex) = library::texture(&key) {
                        set_base_map(&mut m, &key, tex);
                    }
                }
                m
            });
        }
        None => reset_if(ms, ids::SKIN, "Skin: ", skin_material),
    }
    match &mh.eye_color {
        Some(c) => {
            let label = format!("Eyes: {}", library::pretty(c));
            put(ms, ids::EYES, &label, || {
                let mm = library::material(&format!("eyes/materials/{c}.mhmat")).unwrap_or_default();
                let key = format!("eyes/materials/{}", mm.diffuse_texture.clone().unwrap_or_default());
                let mut m = from_mhmat(label.clone(), &mm, &key, library::texture(&key));
                (m.roughness, m.opacity) = (0.08, 1.0);
                m
            });
        }
        None => reset_if(ms, ids::EYES, "Eyes: ", eyes_material),
    }
}

pub fn eyes_material() -> Material {
    Material { base_color: [0.32, 0.22, 0.14], roughness: 0.08, ..Material::named("Eyes") }
}

/// Puts back the plain pack material when a skin / eye colour is cleared.
fn reset_if(ms: &mut MultiSub, id: u16, pre: &str, make: fn() -> Material) {
    if let Some(cur) = ms.subs.iter_mut().find(|s| s.id == id && s.name.starts_with(pre)) {
        let mat = make();
        (cur.name, cur.mat) = (mat.name.clone(), mat);
    }
}
