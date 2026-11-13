//! The bundled MakeHuman system assets (CC0): eyebrows, eyelashes, teeth,
//! tongue, hair, clothes, low-poly body proxies, skins and eye colours.
//! Items are parsed on first use and cached; textures are decoded once
//! and shared by every material that uses them.

use super::assets;
use super::mhclo::Proxy;
use super::obj::BaseMesh;
use super::pack::Pack;
use crate::materials::texture::{TexHandle, Texture};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Eyebrows,
    Eyelashes,
    Teeth,
    Tongue,
    Hair,
    Clothes,
    /// Alternative (low-poly) body topologies.
    Proxy,
}

impl Category {
    pub const ALL: [Category; 7] = [Self::Eyebrows, Self::Eyelashes, Self::Teeth, Self::Tongue, Self::Hair, Self::Clothes, Self::Proxy];

    pub fn dir(self) -> &'static str {
        match self {
            Self::Eyebrows => "eyebrows",
            Self::Eyelashes => "eyelashes",
            Self::Teeth => "teeth",
            Self::Tongue => "tongue",
            Self::Hair => "hair",
            Self::Clothes => "clothes",
            Self::Proxy => "proxymeshes",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Eyebrows => "Eyebrows",
            Self::Eyelashes => "Eyelashes",
            Self::Teeth => "Teeth",
            Self::Tongue => "Tongue",
            Self::Hair => "Hair",
            Self::Clothes => "Clothes",
            Self::Proxy => "Body Topology",
        }
    }
}

/// The parts of a `.mhmat` material Ilex uses.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MhMat {
    pub name: String,
    pub diffuse_color: [f32; 3],
    pub diffuse_texture: Option<String>,
    pub transparent: bool,
    pub shininess: f32,
}

impl MhMat {
    pub fn parse(src: &str) -> Self {
        let mut m = MhMat { diffuse_color: [1.0; 3], shininess: 0.5, ..Default::default() };
        for line in src.lines() {
            let f: Vec<&str> = line.split_ascii_whitespace().collect();
            match f.as_slice() {
                ["name", rest @ ..] => m.name = rest.join(" "),
                ["diffuseColor", r, g, b, ..] => m.diffuse_color = [r, g, b].map(|s| s.parse().unwrap_or(1.0)),
                ["diffuseTexture", t, ..] => m.diffuse_texture = Some(t.rsplit('/').next().unwrap_or(t).to_string()),
                ["transparent", v, ..] => m.transparent = v.eq_ignore_ascii_case("true"),
                ["shininess", v, ..] => m.shininess = v.parse().unwrap_or(0.5),
                _ => {}
            }
        }
        m
    }
}

/// One fitted asset: its binding, mesh and material.
#[derive(Debug)]
pub struct Item {
    /// `category/item` folder in the pack.
    pub folder: String,
    pub proxy: Proxy,
    pub mesh: BaseMesh,
    pub mat: MhMat,
}

impl Item {
    pub fn texture(&self) -> Option<TexHandle> {
        texture(&format!("{}/{}", self.folder, self.mat.diffuse_texture.as_deref()?))
    }
}

pub fn system() -> &'static Pack<'static> {
    static CELL: OnceLock<Pack<'static>> = OnceLock::new();
    CELL.get_or_init(|| Pack::parse(assets::system_pack()))
}

fn textures() -> &'static Pack<'static> {
    static CELL: OnceLock<Pack<'static>> = OnceLock::new();
    CELL.get_or_init(|| Pack::parse(assets::textures_pack()))
}

/// Sorted sub-folder names under `dir/` that hold a file ending in `ext`.
fn folders(dir: &str, ext: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = system()
        .records
        .keys()
        .filter_map(|k| {
            let rest = k.strip_prefix(dir)?.strip_prefix('/')?;
            let (item, file) = rest.split_once('/')?;
            file.ends_with(ext).then_some(item)
        })
        .collect();
    out.dedup();
    out
}

/// Item names in `cat`, sorted.
pub fn items(cat: Category) -> Vec<&'static str> {
    folders(cat.dir(), if cat == Category::Proxy { ".proxy" } else { ".mhclo" })
}

/// Skin material names (`skins/<name>/<name>.mhmat`).
pub fn skins() -> Vec<&'static str> {
    folders("skins", ".mhmat")
}

/// Eye colour names (`eyes/materials/<name>.mhmat`).
pub fn eye_colors() -> Vec<&'static str> {
    system().records.keys().filter_map(|k| k.strip_prefix("eyes/materials/")?.strip_suffix(".mhmat")).collect()
}

/// Bundled then imported item names in `cat` (imported names never
/// clash with bundled ones).
pub fn all_items(cat: Category) -> Vec<String> {
    let mut v: Vec<String> = items(cat).into_iter().map(str::to_string).collect();
    v.extend(super::user_lib::names(cat));
    v
}

/// True for an item the user imported.
pub fn is_user(cat: Category, name: &str) -> bool {
    !items(cat).contains(&name) && super::user_lib::names(cat).iter().any(|n| n == name)
}

/// "male_casualsuit01" -> "Male Casualsuit 01".
pub fn pretty(name: &str) -> String {
    let mut out = String::new();
    let mut prev_digit = false;
    for (i, w) in name.split(['_', '-']).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        for (j, c) in w.chars().enumerate() {
            if c.is_ascii_digit() && !prev_digit && j > 0 {
                out.push(' ');
            }
            out.push(if j == 0 { c.to_ascii_uppercase() } else { c });
            prev_digit = c.is_ascii_digit();
        }
        prev_digit = false;
    }
    out
}

fn parse_item(cat: Category, name: &str) -> Option<Item> {
    let sys = system();
    let folder = format!("{}/{name}", cat.dir());
    let ext = if cat == Category::Proxy { "proxy" } else { "mhclo" };
    let clo_key = sys.records.keys().find(|k| k.starts_with(&format!("{folder}/")) && k.ends_with(&format!(".{ext}")))?;
    let clo = sys.text(clo_key)?;
    let proxy = Proxy::parse(clo);
    let mesh = BaseMesh::parse(sys.text(&format!("{folder}/{}", proxy.obj_file))?);
    let mat_file = clo.lines().find_map(|l| l.strip_prefix("material ")).map(|s| s.trim().rsplit('/').next().unwrap_or("").to_string());
    let mat_src = mat_file
        .and_then(|f| sys.text(&format!("{folder}/{f}")))
        .or_else(|| sys.records.iter().find(|(k, _)| k.starts_with(&format!("{folder}/")) && k.ends_with(".mhmat")).and_then(|(_, v)| std::str::from_utf8(v).ok()));
    Some(Item { folder, proxy, mesh, mat: mat_src.map(MhMat::parse).unwrap_or_default() })
}

type ItemCache = Mutex<HashMap<(Category, String), Option<Arc<Item>>>>;

fn item_cache() -> &'static ItemCache {
    static CACHE: OnceLock<ItemCache> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Drops a cached item (after an import or removal).
pub fn forget(cat: Category, name: &str) {
    if let Ok(mut c) = item_cache().lock() {
        c.remove(&(cat, name.to_string()));
    }
}

/// The parsed item (cached): bundled first, then the user's imports.
/// None if neither has it.
pub fn item(cat: Category, name: &str) -> Option<Arc<Item>> {
    let cache = item_cache();
    let key = (cat, name.to_string());
    if let Some(hit) = cache.lock().ok()?.get(&key) {
        return hit.clone();
    }
    let parsed = parse_item(cat, name).or_else(|| super::user_lib::parse(cat, name)).map(Arc::new);
    cache.lock().ok()?.insert(key, parsed.clone());
    parsed
}

/// A material file outside the proxy folders (skins, eye colours).
pub fn material(key: &str) -> Option<MhMat> {
    system().text(key).map(MhMat::parse)
}

fn decode(key: &str) -> Option<Texture> {
    if let Some(path) = key.strip_prefix(super::user_lib::USER) {
        let rgba = image::open(path).ok()?.into_rgba8();
        let (w, h) = rgba.dimensions();
        return Some(Texture { width: w, height: h, data: rgba.pixels().map(|p| p.0).collect() });
    }
    let rgb = image::load_from_memory_with_format(textures().get(&format!("{key}.rgb"))?, image::ImageFormat::Jpeg).ok()?.into_rgb8();
    let (w, h) = rgb.dimensions();
    let alpha = textures()
        .get(&format!("{key}.alpha"))
        .and_then(|b| image::load_from_memory_with_format(b, image::ImageFormat::Png).ok())
        .map(|a| a.into_luma8())
        .filter(|a| a.dimensions() == (w, h));
    let data = rgb.pixels().enumerate().map(|(i, p)| [p[0], p[1], p[2], alpha.as_ref().map_or(255, |a| a.as_raw()[i])]).collect();
    Some(Texture { width: w, height: h, data })
}

/// The decoded texture `folder/file.png` (shared; follows `.link` records
/// for textures that several items use).
pub fn texture(key: &str) -> Option<TexHandle> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<TexHandle>>>> = OnceLock::new();
    let key = system().text(&format!("{key}.link")).unwrap_or(key);
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok()?.get(key) {
        return hit.clone();
    }
    let tex = decode(key).map(TexHandle::new);
    cache.lock().ok()?.insert(key.to_string(), tex.clone());
    tex
}
