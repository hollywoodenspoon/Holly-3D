//! Ilex is its own app: no other modelling app or plugin is named in the
//! source, except where old files must keep loading.

use std::path::Path;

/// Names that must not appear in `src/`.
const BANNED: [&str; 7] = ["3ds Max", "Max-style", "Blender", "GhostTown", "ProBoolean", "Cycles", "Tom Hudson"];

/// Files that read old saved data and must spell the old names.
const LEGACY: [&str; 4] = [
    "render/tests_render.rs", // pre-11.5 Render Setup file
    "io/scene_file.rs",       // old modifier names in scenes
    "modifiers/catalog.rs",   // old modifier names in button sets
    "brand_tests.rs",
];

fn scan(dir: &Path, root: &Path, bad: &mut Vec<String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            scan(&p, root, bad);
            continue;
        }
        let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if p.extension().is_none_or(|x| x != "rs") || LEGACY.contains(&rel.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        for (i, line) in text.lines().enumerate() {
            for b in BANNED.iter().filter(|b| line.contains(*b)) {
                bad.push(format!("{rel}:{}: {b}", i + 1));
            }
        }
    }
}

#[test]
fn source_names_no_other_modelling_apps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bad = Vec::new();
    scan(&root, &root, &mut bad);
    assert!(bad.is_empty(), "other apps named in src:\n{}", bad.join("\n"));
}
