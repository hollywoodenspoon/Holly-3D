//! The user's own MakeHuman assets: hair, clothes, brows, lashes, teeth,
//! tongue and body proxies imported from a folder or `.zip` (for example
//! a community asset download) into `~/.local/share/ilex/makehuman/
//! <category>/<name>/`. They show up next to the bundled items and fit
//! exactly the same way. Each item folder is flat: its `.mhclo` (or
//! `.proxy`), `.obj`, `.mhmat` and textures side by side.

use super::library::{Category, Item, MhMat};
use super::mhclo::Proxy;
use super::obj::BaseMesh;
use std::path::{Path, PathBuf};

/// `Item::folder` prefix that marks a user item (followed by its path).
pub const USER: &str = "user:";

/// Where imported assets live (`ILEX_MH_DIR` overrides it, for tests).
pub fn root() -> PathBuf {
    match std::env::var_os("ILEX_MH_DIR") {
        Some(d) => PathBuf::from(d),
        None => crate::prefs::data_dir().join("makehuman"),
    }
}

fn ext(cat: Category) -> &'static str {
    if cat == Category::Proxy { "proxy" } else { "mhclo" }
}

fn has_ext(p: &Path, e: &str) -> bool {
    p.extension().is_some_and(|x| x.eq_ignore_ascii_case(e))
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    v.sort();
    v
}

/// Imported item names in `cat`, sorted.
pub fn names(cat: Category) -> Vec<String> {
    let dir = root().join(cat.dir());
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| files(&e.path()).iter().any(|f| has_ext(f, ext(cat))))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    v.sort();
    v
}

/// The imported item `name`, parsed from disk.
pub fn parse(cat: Category, name: &str) -> Option<Item> {
    if name.contains(['/', '\\']) || name.starts_with('.') {
        return None;
    }
    let dir = root().join(cat.dir()).join(name);
    let all = files(&dir);
    let clo = std::fs::read_to_string(all.iter().find(|f| has_ext(f, ext(cat)))?).ok()?;
    let proxy = Proxy::parse(&clo);
    let obj_name = Path::new(&proxy.obj_file).file_name()?.to_owned();
    let mesh = BaseMesh::parse(&std::fs::read_to_string(dir.join(obj_name)).ok()?);
    let wanted = clo.lines().find_map(|l| l.strip_prefix("material ")).and_then(|s| Path::new(s.trim()).file_name().map(|f| dir.join(f)));
    let mat_path = wanted.filter(|p| p.is_file()).or_else(|| all.iter().find(|f| has_ext(f, "mhmat")).cloned());
    let mut mat = mat_path.and_then(|p| std::fs::read_to_string(p).ok()).map(|s| MhMat::parse(&s)).unwrap_or_default();
    // A texture the .mhmat names under another extension (png vs jpg).
    if let Some(t) = mat.diffuse_texture.clone().filter(|t| !dir.join(t).is_file()) {
        let stem = Path::new(&t).file_stem().map(|s| s.to_ascii_lowercase());
        mat.diffuse_texture = all.iter().find(|f| f.file_stem().map(|s| s.to_ascii_lowercase()) == stem && is_image(f)).and_then(|f| Some(f.file_name()?.to_str()?.to_string()));
    }
    Some(Item { folder: format!("{USER}{}", dir.display()), proxy, mesh, mat })
}

fn is_image(p: &Path) -> bool {
    ["png", "jpg", "jpeg", "tga", "bmp", "webp", "tif", "tiff"].iter().any(|e| has_ext(p, e))
}

/// Deletes an imported item.
pub fn remove(cat: Category, name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return Err("bad item name".into());
    }
    std::fs::remove_dir_all(root().join(cat.dir()).join(name)).map_err(|e| e.to_string())?;
    super::library::forget(cat, name);
    Ok(())
}

/// Which category an asset file belongs to: its folder names first
/// (`hair/`, `clothes/`...), then its `tag` lines, else Clothes.
fn category(path: &str, text: &str) -> Category {
    if path.to_ascii_lowercase().ends_with(".proxy") {
        return Category::Proxy;
    }
    let path = path.to_ascii_lowercase();
    let tags: String = text.lines().filter(|l| l.starts_with("tag")).collect::<Vec<_>>().join(" ").to_ascii_lowercase();
    let order = [Category::Eyebrows, Category::Eyelashes, Category::Teeth, Category::Tongue, Category::Hair];
    let keys = |c: Category| match c {
        Category::Eyebrows => ["eyebrow", "eyebrow"],
        Category::Eyelashes => ["eyelash", "lashes"],
        Category::Teeth => ["teeth", "teeth"],
        Category::Tongue => ["tongue", "tongue"],
        _ => ["hair", "hair"],
    };
    let dir_hit = order.into_iter().find(|&c| path.split('/').any(|seg| keys(c).iter().any(|k| seg.starts_with(k))));
    dir_hit.or_else(|| order.into_iter().find(|&c| keys(c).iter().any(|k| tags.contains(k)))).unwrap_or(Category::Clothes)
}

fn clean(s: &str) -> String {
    let s: String = s.to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() { "item".into() } else { s }
}

/// A name not used by a bundled or an imported item.
fn free_name(cat: Category, want: &str) -> String {
    let taken = |n: &str| super::library::items(cat).contains(&n) || root().join(cat.dir()).join(n).exists();
    let base = clean(want);
    (1..).map(|i| if i == 1 { base.clone() } else { format!("{base}_{i}") }).find(|n| !taken(n)).unwrap_or(base)
}

/// Imports every asset in `files` (archive-relative paths): each `.mhclo`
/// or `.proxy` takes the files of its own folder (and sub-folders) with
/// it. `force` overrides the guessed category. Returns what was added.
pub fn import_files(files: &[(String, Vec<u8>)], force: Option<Category>) -> Result<Vec<(Category, String)>, String> {
    let mut added = Vec::new();
    for (path, data) in files.iter().filter(|(p, _)| has_ext(Path::new(p), "mhclo") || has_ext(Path::new(p), "proxy")) {
        let text = String::from_utf8_lossy(data);
        let cat = force.unwrap_or_else(|| category(path, &text));
        let folder = path.rsplit_once('/').map_or("", |(d, _)| d);
        let stem = Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("item");
        let want = folder.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(stem);
        let name = free_name(cat, want);
        let dest = root().join(cat.dir()).join(&name);
        std::fs::create_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        let mine = |p: &str| folder.is_empty() || p.starts_with(&format!("{folder}/"));
        for (p, d) in files.iter().filter(|(p, _)| mine(p)) {
            let Some(file) = Path::new(p).file_name() else { continue };
            // Only this asset's binding: sibling assets keep their own.
            let other = (has_ext(Path::new(p), "mhclo") || has_ext(Path::new(p), "proxy")) && p != path;
            if !other {
                std::fs::write(dest.join(file), d).map_err(|e| e.to_string())?;
            }
        }
        if parse(cat, &name).is_none() {
            let _ = std::fs::remove_dir_all(&dest);
            return Err(format!("{path}: its .obj is missing or unreadable"));
        }
        super::library::forget(cat, &name);
        added.push((cat, name));
    }
    if added.is_empty() {
        return Err("no .mhclo or .proxy file found".into());
    }
    Ok(added)
}

/// Reads a folder (recursively) or a `.zip` and imports it.
pub fn import(src: &Path, force: Option<Category>) -> Result<Vec<(Category, String)>, String> {
    let files = if src.is_dir() {
        let mut out = Vec::new();
        walk(src, src, &mut out, 0);
        // Keep the picked folder's own name (and its parent's, which is
        // often `hair` or `clothes`) so the item and category are known.
        let names: Vec<String> = [src.parent().and_then(|p| p.file_name()), src.file_name()].into_iter().flatten().map(|n| n.to_string_lossy().into_owned()).collect();
        let pre = names.join("/");
        out.into_iter().map(|(p, d)| (format!("{pre}/{p}"), d)).collect()
    } else if has_ext(src, "zip") {
        super::unzip::entries(&std::fs::read(src).map_err(|e| e.to_string())?)?
    } else {
        // A single .mhclo: take its whole folder.
        return import(src.parent().ok_or("no folder")?, force);
    };
    import_files(&files, force)
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>, depth: u32) {
    for p in std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()) {
        if p.is_dir() && depth < 6 {
            walk(base, &p, out, depth + 1);
        } else if let (Ok(rel), Ok(d)) = (p.strip_prefix(base), std::fs::read(&p)) {
            out.push((rel.to_string_lossy().replace('\\', "/"), d));
        }
    }
}
