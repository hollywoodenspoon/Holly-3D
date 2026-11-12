//! The downloader against a local stand-in for the asset hosts, so the
//! tests run offline: a tiny HTTP server on 127.0.0.1 serving fixture
//! JSON, images and a ZIP archive.

use super::download::{self, Phase};
use super::polyhaven::{self, Group};
use super::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Serves the routes `make(base_url)` returns (path -> body) until the
/// test ends; returns the base URL and a counter of requests answered.
fn serve_with(make: impl FnOnce(&str) -> HashMap<String, Vec<u8>>) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}", listener.local_addr().unwrap());
    let routes = make(&base);
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
            counter.fetch_add(1, Ordering::Relaxed);
            let (status, body) = routes.get(&path).map_or(("404 Not Found", b"missing".to_vec()), |b| ("200 OK", b.clone()));
            let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = s.write_all(head.as_bytes()).and_then(|()| s.write_all(&body));
        }
    });
    (base, hits)
}

fn serve(routes: HashMap<String, Vec<u8>>) -> (String, Arc<AtomicUsize>) {
    serve_with(|_| routes)
}

fn temp(name: &str) -> String {
    let d = std::env::temp_dir().join(format!("ilex_assets_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d.to_string_lossy().into_owned()
}

/// A 4 x 4 JPEG.
fn jpeg(rgb: [u8; 3]) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(4, 4, image::Rgb(rgb));
    let mut out = std::io::Cursor::new(vec![]);
    img.write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
    out.into_inner()
}

/// The texture listing and file replies for two brick walls and a roof.
fn photo_routes(base: &str) -> HashMap<String, Vec<u8>> {
    let listing = serde_json::json!({
        "red_brick": { "name": "Red Brick", "categories": ["brick", "wall"], "download_count": 900 },
        "old_plaster": { "name": "Old Plaster", "categories": ["plaster"], "download_count": 500 },
        "worn_plaster": { "categories": ["plaster"], "download_count": 100 },
        "clay_roof": { "name": "Clay Roof", "categories": ["roofing"], "download_count": 50 },
        "silk": { "name": "Silk", "categories": ["fabric"], "download_count": 99999 },
    });
    let files = |slug: &str, maps: &[&str]| {
        let mut f = serde_json::Map::new();
        for m in maps {
            f.insert(m.to_string(), serde_json::json!({ "1k": { "jpg": { "url": format!("{base}/dl/{slug}_{m}_1k.jpg"), "size": 100 } }, "2k": { "jpg": { "url": format!("{base}/dl/{slug}_{m}_2k.jpg") } } }));
        }
        serde_json::to_vec(&serde_json::Value::Object(f)).unwrap()
    };
    let mut r = HashMap::new();
    r.insert("/assets?t=textures".to_string(), serde_json::to_vec(&listing).unwrap());
    r.insert("/files/red_brick".to_string(), files("red_brick", &["Diffuse", "nor_gl", "Rough"]));
    // No roughness map listed; the normal map is listed but the host lost it.
    r.insert("/files/old_plaster".to_string(), files("old_plaster", &["Diffuse", "nor_gl"]));
    r.insert("/files/clay_roof".to_string(), files("clay_roof", &["Diffuse"]));
    for (slug, maps) in [("red_brick", vec!["Diffuse", "nor_gl", "Rough"]), ("old_plaster", vec!["Diffuse"]), ("clay_roof", vec!["Diffuse"])] {
        for m in maps {
            r.insert(format!("/dl/{slug}_{m}_1k.jpg"), jpeg([200, 60, 40]));
        }
    }
    r
}

#[test]
fn packs_are_missing_then_partial_then_installed_then_removed() {
    let folder = temp("state");
    assert_eq!(state(Pack::Photos, &folder), PackState::Missing);
    let dir = Pack::Photos.path(&folder);
    assert!(dir.ends_with("photos"));
    std::fs::create_dir_all(dir.join("red_brick")).unwrap();
    assert_eq!(state(Pack::Photos, &folder), PackState::Missing, "an empty folder is nothing yet");
    std::fs::write(dir.join("red_brick/diff.jpg"), b"x").unwrap();
    assert_eq!(state(Pack::Photos, &folder), PackState::Partial { files: 1 });
    mark_installed(Pack::Photos, &folder, Installed { files: 1, bytes: 1 }).unwrap();
    assert_eq!(state(Pack::Photos, &folder), PackState::Installed(Installed { files: 1, bytes: 1 }));
    assert_eq!(state(Pack::LowPoly, &folder), PackState::Missing, "packs are separate");
    remove(Pack::Photos, &folder).unwrap();
    assert_eq!(state(Pack::Photos, &folder), PackState::Missing);
    remove(Pack::Photos, &folder).unwrap();
    // Paths from the network never escape the pack folder.
    assert_eq!(safe_rel("textures/a.jpg").unwrap(), std::path::Path::new("textures").join("a.jpg"));
    assert!(safe_rel("../secret").is_none() && safe_rel("a/../../b").is_none() && safe_rel("C:/x").is_none() && safe_rel("").is_none());
    assert_eq!(safe_rel("/abs/file").unwrap(), std::path::Path::new("abs").join("file"));
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn textures_are_picked_per_shelf_by_popularity() {
    let listing: serde_json::Value = serde_json::from_slice(&photo_routes("http://x")["/assets?t=textures"]).unwrap();
    let picks = polyhaven::pick_textures(&listing, 2);
    let got: Vec<(&str, &str, Group)> = picks.iter().map(|p| (p.slug.as_str(), p.name.as_str(), p.group)).collect();
    assert_eq!(got, [("clay_roof", "Clay Roof", Group::Roofs), ("red_brick", "Red Brick", Group::Walls), ("old_plaster", "Old Plaster", Group::Walls)], "two walls at most, fabric is no shelf of ours");
    assert_eq!(polyhaven::pick_textures(&listing, 9).iter().find(|p| p.slug == "worn_plaster").unwrap().name, "Worn Plaster", "a missing name is made from the slug");
    assert!(polyhaven::pick_textures(&serde_json::json!([]), 9).is_empty());
    // Files: the wanted resolution, diffuse required, the rest optional.
    let files: serde_json::Value = serde_json::from_slice(&photo_routes("http://x")["/files/red_brick"]).unwrap();
    let list = polyhaven::texture_files("red_brick", &files, "2k");
    assert_eq!(list.iter().map(|f| (f.path.as_str(), f.required)).collect::<Vec<_>>(), [("red_brick/diff.jpg", true), ("red_brick/nor.jpg", false), ("red_brick/rough.jpg", false)]);
    assert!(list[0].url.ends_with("red_brick_Diffuse_2k.jpg"));
    // Models: the glTF and everything it includes, safely placed.
    let model = serde_json::json!({ "gltf": { "1k": { "gltf": { "url": "http://x/barrel.gltf", "include": {
        "barrel.bin": { "url": "http://x/barrel.bin" }, "textures/barrel_diff_1k.jpg": { "url": "http://x/d.jpg" }, "../evil": { "url": "http://x/e" } } } } } });
    let paths: Vec<String> = polyhaven::model_files("barrel", &model, "1k").into_iter().map(|f| f.path).collect();
    assert_eq!(paths, ["barrel/barrel.gltf", "barrel/barrel.bin", "barrel/textures/barrel_diff_1k.jpg"]);
    assert_eq!(polyhaven::pick_models(&serde_json::json!({ "a": { "download_count": 1 }, "b": { "download_count": 5 } }), 1), ["b"]);
}

#[test]
fn the_photo_pack_downloads_installs_resumes_and_feeds_the_library() {
    let (base, hits) = serve_with(photo_routes);
    let folder = temp("photos");
    let sources = Sources { api: base, photos_per_group: 2, ..Default::default() };
    let job = download::blocking_job();
    let installed = download::run(Pack::Photos, &folder, &sources, &job).expect("download");
    let p = job.snapshot();
    assert_eq!((p.files_done, p.files_total, p.skipped), (6, 6, 1), "5 images arrive; the lost normal map is skipped: {p:?}");
    assert!(p.bytes > 500 && (p.fraction() - 1.0).abs() < 1e-6);
    assert_eq!(state(Pack::Photos, &folder), PackState::Installed(installed));
    let dir = Pack::Photos.path(&folder);
    for f in ["red_brick/diff.jpg", "red_brick/nor.jpg", "red_brick/rough.jpg", "old_plaster/diff.jpg", "clay_roof/diff.jpg", "index.ron", "installed.ron"] {
        assert!(dir.join(f).is_file(), "{f}");
    }
    assert!(!dir.join("old_plaster/nor.jpg").exists());
    // The Material Library's photo shelf.
    let photos = photo::scan(&folder);
    assert_eq!(photos.iter().map(|p| (p.name, p.group)).collect::<Vec<_>>(), [("📷 Clay Roof", Group::Roofs), ("📷 Old Plaster", Group::Walls), ("📷 Red Brick", Group::Walls)]);
    let m = photos[2].material();
    assert_eq!(m.name, "Red Brick");
    use crate::materials::{MapChannel, MapNode};
    for ch in [MapChannel::BaseColor, MapChannel::Bump, MapChannel::Roughness] {
        assert!(matches!(m.map(ch).node, Some(MapNode::Bitmap { tex: Some(_), .. })), "{ch:?}");
    }
    assert!(crate::materials::bake::direct_base(&m).is_some(), "the photo is used at full resolution");
    assert!(photos[0].material().map(MapChannel::Bump).node.is_none(), "no normal map, no bump slot");
    // Running again fetches no image twice (only the listings).
    let before = hits.load(Ordering::Relaxed);
    download::run(Pack::Photos, &folder, &sources, &download::blocking_job()).unwrap();
    assert_eq!(hits.load(Ordering::Relaxed) - before, 5, "1 listing + 3 file lists + the still-missing normal map");
    remove(Pack::Photos, &folder).unwrap();
    assert!(photo::scan(&folder).is_empty());
}

#[test]
fn the_low_poly_pack_unzips_models_and_failures_leave_a_partial_pack() {
    let zip = zip::build(&[
        ("Starter-Kit-main/models/road-straight.glb", b"glTF-road"),
        ("Starter-Kit-main/models/Textures/colormap.png", b"png-bytes"),
        ("Starter-Kit-main/scripts/builder.gd", b"extends Node"),
        ("Starter-Kit-main/../../escape.glb", b"nope"),
    ]);
    let entries = zip::entries(&zip).unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].read().unwrap(), b"glTF-road");
    assert!(zip::entries(b"not a zip").is_err());
    let (base, _) = serve(HashMap::from([("/kit.zip".to_string(), zip)]));
    let folder = temp("lowpoly");
    let sources = Sources { lowpoly_zips: vec![format!("{base}/kit.zip")], ..Default::default() };
    let job = download::blocking_job();
    download::run(Pack::LowPoly, &folder, &sources, &job).expect("download");
    let kit = Pack::LowPoly.path(&folder).join("kit01");
    assert_eq!(std::fs::read(kit.join("road-straight.glb")).unwrap(), b"glTF-road");
    assert!(kit.join("colormap.png").is_file() && !kit.join("builder.gd").exists(), "models and their textures only");
    assert!(!std::path::Path::new(&folder).join("escape.glb").exists() && !kit.join("escape.glb").exists(), "paths with .. are dropped");
    assert!(matches!(state(Pack::LowPoly, &folder), PackState::Installed(i) if i.files == 2));
    // A dead host: the error names it, and nothing claims to be installed.
    let dead = Sources { lowpoly_zips: vec![format!("{base}/gone.zip")], ..Default::default() };
    let folder2 = temp("lowpoly_dead");
    let err = download::run(Pack::LowPoly, &folder2, &dead, &download::blocking_job()).unwrap_err();
    assert!(err.contains("HTTP 404") && err.contains("gone.zip"), "{err}");
    assert_eq!(state(Pack::LowPoly, &folder2), PackState::Missing);
    // The background job reports the same failure, and a cancelled one says so.
    let job = download::start(Pack::LowPoly, folder2.clone(), dead);
    while job.running() {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(matches!(job.snapshot().phase, Phase::Failed(e) if e.contains("404")));
    let cancelled = download::blocking_job();
    cancelled.cancel();
    assert_eq!(download::run(Pack::LowPoly, &folder2, &sources, &cancelled).unwrap_err(), "cancelled");
    for f in [folder, folder2] {
        let _ = std::fs::remove_dir_all(f);
    }
}

#[test]
fn defaults_point_at_the_public_cc0_hosts() {
    let s = Sources::default();
    assert_eq!((s.api.as_str(), s.resolution.as_str(), s.photos_per_group), ("https://api.polyhaven.com", "1k", 9));
    assert!(s.lowpoly_zips.len() >= 3 && s.lowpoly_zips.iter().all(|u| u.starts_with("https://github.com/KenneyNL/") && u.ends_with(".zip")));
    for p in Pack::ALL {
        assert!(!p.label().is_empty() && !p.about().is_empty() && !p.size_hint().is_empty());
    }
    // Old preference files without a `sources` entry still load.
    let old: crate::prefs::AssetPrefs = ron::from_str("(folder:\"/x\",photos:true,lowpoly:false,realistic:false)").unwrap();
    assert_eq!(old.sources, s);
}
