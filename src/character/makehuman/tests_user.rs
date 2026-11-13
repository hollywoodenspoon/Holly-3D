use super::library::{self, Category};
use super::user_lib;

/// The bundled `long01` hair as archive files under `prefix/`.
fn long01(prefix: &str) -> Vec<(String, Vec<u8>)> {
    library::system()
        .records
        .iter()
        .filter_map(|(k, v)| Some((format!("{prefix}/{}", k.strip_prefix("hair/long01/")?), v.to_vec())))
        .collect()
}

#[test]
fn imported_assets_join_the_library_and_can_be_removed() {
    let dir = std::env::temp_dir().join(format!("ilex_mh_test_{}", std::process::id()));
    // SAFETY: only this test sets the variable; other tests just read
    // the (empty) user folder it points at.
    unsafe { std::env::set_var("ILEX_MH_DIR", &dir) };
    let files = long01("downloads/hair/myhair");
    assert!(files.iter().any(|(p, _)| p.ends_with(".mhclo")));
    let added = user_lib::import_files(&files, None).unwrap();
    assert_eq!(added, vec![(Category::Hair, "myhair".to_string())]);
    assert!(library::all_items(Category::Hair).contains(&"myhair".to_string()));
    assert!(library::is_user(Category::Hair, "myhair") && !library::is_user(Category::Hair, "long01"));
    let mine = library::item(Category::Hair, "myhair").unwrap();
    let theirs = library::item(Category::Hair, "long01").unwrap();
    assert_eq!(mine.mesh.coords.len(), theirs.mesh.coords.len());
    assert_eq!(mine.proxy.bindings.len(), theirs.proxy.bindings.len());
    assert!(mine.folder.starts_with(user_lib::USER));

    // Same name again, from a zip without folders: a free name, and the
    // forced category wins.
    let owned = long01("x");
    let flat: Vec<(&str, &[u8])> = owned.iter().map(|(p, d)| (&p[2..], d.as_slice())).collect();
    let z = super::unzip::tests::stored(&flat);
    let zpath = dir.join("x.zip");
    std::fs::write(&zpath, z).unwrap();
    let again = user_lib::import(&zpath, Some(Category::Hair)).unwrap();
    assert_eq!(again, vec![(Category::Hair, "long01_2".to_string())]);
    // A name clashing with a bundled item never shadows it.
    assert!(library::item(Category::Hair, "long01").unwrap().folder.starts_with("hair/"));

    user_lib::remove(Category::Hair, "myhair").unwrap();
    assert!(library::item(Category::Hair, "myhair").is_none());
    assert!(user_lib::remove(Category::Hair, "../etc").is_err());
    assert!(user_lib::import_files(&[("a/readme.txt".into(), b"hi".to_vec())], None).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
