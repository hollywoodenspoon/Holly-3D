//! Poly Haven's public API (https://api.polyhaven.com): choosing which
//! assets go into a pack and finding the files to download for each.
//! Everything here is plain JSON in, download list out.

use serde_json::Value;

/// One file to fetch into a pack folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fetch {
    pub url: String,
    /// Path inside the pack folder.
    pub path: String,
    /// A missing optional file (say, a roughness map) is skipped.
    pub required: bool,
}

/// Library shelf a photo texture belongs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Group {
    Ground,
    Roads,
    Walls,
    Roofs,
    Wood,
    Stone,
    Metals,
}

impl Group {
    pub const ALL: [Group; 7] = [Group::Roads, Group::Roofs, Group::Walls, Group::Wood, Group::Metals, Group::Stone, Group::Ground];

    /// Poly Haven categories that put a texture in this group.
    fn keys(self) -> &'static [&'static str] {
        match self {
            Group::Roads => &["asphalt", "road", "cobblestone"],
            Group::Roofs => &["roofing", "roof"],
            Group::Walls => &["brick", "plaster", "wall"],
            Group::Wood => &["wood", "bark"],
            Group::Metals => &["metal"],
            Group::Stone => &["rock", "concrete", "stone", "tiles"],
            Group::Ground => &["terrain", "sand", "snow", "ground", "soil"],
        }
    }
}

/// An asset picked for a pack.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pick {
    pub slug: String,
    pub name: String,
    pub group: Group,
}

fn categories(asset: &Value) -> Vec<String> {
    asset["categories"].as_array().map_or(vec![], |a| a.iter().filter_map(|c| c.as_str()).map(str::to_lowercase).collect())
}

fn downloads(asset: &Value) -> u64 {
    asset["download_count"].as_u64().unwrap_or(0)
}

/// Title for a slug when the API gives no name: "rock_wall_10" -> "Rock Wall 10".
pub fn title(slug: &str) -> String {
    slug.split(['_', '-']).filter(|w| !w.is_empty()).map(|w| w[..1].to_uppercase() + &w[1..]).collect::<Vec<_>>().join(" ")
}

/// The most downloaded textures of every group, `per_group` each, from
/// the `/assets?t=textures` listing. Slugs are sorted so the pick is stable.
pub fn pick_textures(listing: &Value, per_group: usize) -> Vec<Pick> {
    let Some(map) = listing.as_object() else { return vec![] };
    let mut out = vec![];
    let mut taken = std::collections::HashSet::new();
    for group in Group::ALL {
        let mut hits: Vec<(&String, &Value)> = map.iter().filter(|(slug, a)| !taken.contains(*slug) && categories(a).iter().any(|c| group.keys().contains(&c.as_str()))).collect();
        hits.sort_by(|a, b| downloads(b.1).cmp(&downloads(a.1)).then(a.0.cmp(b.0)));
        for (slug, a) in hits.into_iter().take(per_group) {
            taken.insert(slug.clone());
            out.push(Pick { slug: slug.clone(), name: a["name"].as_str().map_or_else(|| title(slug), str::to_string), group });
        }
    }
    out
}

/// The most downloaded models from the `/assets?t=models` listing.
pub fn pick_models(listing: &Value, count: usize) -> Vec<String> {
    let Some(map) = listing.as_object() else { return vec![] };
    let mut all: Vec<(&String, &Value)> = map.iter().collect();
    all.sort_by(|a, b| downloads(b.1).cmp(&downloads(a.1)).then(a.0.cmp(b.0)));
    all.into_iter().take(count).map(|(s, _)| s.clone()).collect()
}

/// `files[map][resolution][format].url`, trying each spelling of the map name.
fn file_url<'a>(files: &'a Value, maps: &[&str], res: &str, formats: &[&str]) -> Option<(&'a str, &'static str)> {
    let obj = files.as_object()?;
    for want in maps {
        let Some((_, m)) = obj.iter().find(|(k, _)| k.eq_ignore_ascii_case(want)) else { continue };
        for (fmt, ext) in formats.iter().zip(["jpg", "png"]) {
            if let Some(url) = m[res][fmt]["url"].as_str() {
                return Some((url, ext));
            }
        }
    }
    None
}

/// Downloads for one photo texture from its `/files/<slug>` reply: the
/// diffuse map (required), plus normal and roughness when it has them.
pub fn texture_files(slug: &str, files: &Value, res: &str) -> Vec<Fetch> {
    let mut out = vec![];
    let maps: [(&str, &[&str], bool); 3] = [("diff", &["Diffuse", "diff", "Base Color", "col"], true), ("nor", &["nor_gl", "Normal"], false), ("rough", &["Rough", "Roughness"], false)];
    for (name, keys, required) in maps {
        if let Some((url, ext)) = file_url(files, keys, res, &["jpg", "png"]) {
            out.push(Fetch { url: url.to_string(), path: format!("{slug}/{name}.{ext}"), required });
        }
    }
    out
}

/// Downloads for one model from its `/files/<slug>` reply: the glTF file
/// and everything it includes (buffers and textures).
pub fn model_files(slug: &str, files: &Value, res: &str) -> Vec<Fetch> {
    let g = &files["gltf"][res]["gltf"];
    let Some(url) = g["url"].as_str() else { return vec![] };
    let mut out = vec![Fetch { url: url.to_string(), path: format!("{slug}/{slug}.gltf"), required: true }];
    if let Some(inc) = g["include"].as_object() {
        for (rel, f) in inc {
            if let (Some(url), Some(rel)) = (f["url"].as_str(), super::safe_rel(rel)) {
                out.push(Fetch { url: url.to_string(), path: format!("{slug}/{}", rel.to_string_lossy().replace('\\', "/")), required: true });
            }
        }
    }
    out
}
