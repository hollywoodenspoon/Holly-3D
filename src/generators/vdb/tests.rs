use super::write::{TestGrid, vdb_bytes};
use super::*;
use std::path::PathBuf;

fn temp(name: &str, bytes: &[u8]) -> String {
    let dir = std::env::temp_dir().join(format!("ilex_vdb_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p: PathBuf = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p.to_string_lossy().into_owned()
}

fn block(at: [u32; 3], n: u32, v: f32) -> Vec<([u32; 3], f32)> {
    let mut out = Vec::new();
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                out.push(([at[0] + x, at[1] + y, at[2] + z], v));
            }
        }
    }
    out
}

fn grid_of(name: &str, voxels: Vec<([u32; 3], f32)>) -> TestGrid {
    TestGrid { name: name.into(), voxels, tiles: Vec::new(), voxel_size: 0.1, translation: [0.0; 3] }
}

/// Smoke above a hot core: a density block and a smaller temperature block.
fn smoke_file(name: &str) -> String {
    let fire = grid_of("temperature", block([100, 100, 100], 4, 3.0));
    let smoke = grid_of("density", block([96, 96, 96], 12, 0.5));
    temp(name, &vdb_bytes(&[smoke, fire, grid_of("vel_magnitude", block([0, 0, 0], 2, 1.0))]))
}

#[test]
fn reads_float_grids_and_picks_smoke_and_fire() {
    let f = smoke_file("smoke.vdb");
    assert_eq!(float_grids(&f).unwrap(), ["density", "temperature", "vel_magnitude"]);
    let p = VdbParams::for_file(&f).unwrap();
    assert_eq!((p.density_grid.as_str(), p.fire_grid.as_str()), ("density", "temperature"));
    // Z-up, full resolution: one dense voxel per file voxel, normalised.
    let g = grid(&f, "density", UpAxis::Z, 192).unwrap();
    assert_eq!((g.vox.dims, g.active), ([12; 3], 12 * 12 * 12));
    assert!((g.peak - 0.5).abs() < 1e-6 && g.vox.data.iter().all(|&d| (d - 1.0).abs() < 1e-6));
    // Voxels are centred on their index: 96 * 0.1 - 0.05 .. 108 * 0.1 - 0.05.
    assert!((g.lo - Vec3::splat(9.55)).abs().max_element() < 1e-4 && (g.hi - Vec3::splat(10.75)).abs().max_element() < 1e-4, "{:?}", (g.lo, g.hi));
}

#[test]
fn y_up_files_stand_up_in_z() {
    let tall = grid_of("density", (0..20).map(|j| ([0, j, 5], 1.0)).collect());
    let f = temp("tall.vdb", &vdb_bytes(&[tall]));
    let z = grid(&f, "density", UpAxis::Z, 192).unwrap();
    let y = grid(&f, "density", UpAxis::Y, 192).unwrap();
    assert_eq!((z.vox.dims, y.vox.dims), ([1, 20, 1], [1, 1, 20]));
    // File +Z (index 5) becomes Ilex -Y.
    assert!((y.lo.y - -0.55).abs() < 1e-4 && (y.hi.y - -0.45).abs() < 1e-4 && (y.hi.z - z.hi.y).abs() < 1e-4, "{:?}", (y.lo, y.hi));
}

#[test]
fn big_grids_box_filter_to_the_resolution_budget() {
    // A 64-voxel line at resolution 16: blocks of 4 average together.
    let line = || (0..64).map(|i| ([i, 0, 0], if i < 32 { 1.0 } else { 0.5 }, ));
    let g = resample(|| line().map(|(c, v)| (c.map(i64::from), 1, v)), [1.0; 3], [0.0; 3], UpAxis::Z, 16).unwrap();
    assert_eq!(g.vox.dims, [16, 1, 1]);
    // Each coarse cell holds 4 of its 64 fine voxels.
    assert!((g.peak - 4.0 / 64.0).abs() < 1e-6);
    assert!((g.vox.data[0] - 1.0).abs() < 1e-6 && (g.vox.data[15] - 0.5).abs() < 1e-6);
    assert_eq!((g.hi - g.lo).x, 64.0);
    // An 8³ tile fills 8 cells per axis at full resolution.
    let t = resample(|| [([8i64, 8, 8], 8i64, 2.0f32)].into_iter(), [0.5; 3], [0.0; 3], UpAxis::Z, 64).unwrap();
    assert_eq!((t.vox.dims, t.peak), ([8; 3], 2.0));
    assert!(resample(std::iter::empty, [1.0; 3], [0.0; 3], UpAxis::Z, 64).is_none());
}

#[test]
fn tiles_in_the_file_are_read() {
    let g = TestGrid { tiles: vec![([16, 16, 16], 0.8)], ..grid_of("density", vec![([0, 0, 0], 0.8)]) };
    let f = temp("tiles.vdb", &vdb_bytes(&[g]));
    let d = grid(&f, "density", UpAxis::Z, 192).unwrap();
    assert_eq!((d.active, d.vox.dims), (2, [24; 3]));
    let full = d.vox.data.iter().filter(|&&v| v > 0.99).count();
    assert_eq!(full, 8 * 8 * 8 + 1, "the tile and the voxel");
}

#[test]
fn bad_files_are_errors_not_crashes() {
    let not = temp("not.vdb", b"just some text, not a volume");
    assert!(float_grids(&not).unwrap_err().contains("not a VDB"));
    let good = vdb_bytes(&[grid_of("density", block([0, 0, 0], 3, 1.0))]);
    for cut in [12, 40, 90, good.len() - 600, good.len() - 10] {
        let f = temp(&format!("cut{cut}.vdb"), &good[..cut]);
        assert!(grid(&f, "density", UpAxis::Y, 64).is_err(), "cut at {cut}");
    }
    let f = temp("ok.vdb", &good);
    assert!(grid(&f, "smoke", UpAxis::Y, 64).unwrap_err().contains("no float grid"));
    assert!(grid("/nonexistent/x.vdb", "density", UpAxis::Y, 64).is_err());
}

#[test]
fn params_render_smoke_and_fire_and_show_a_cage() {
    let f = smoke_file("params.vdb");
    let p = VdbParams { scale: 2.0, up: UpAxis::Z, ..VdbParams::for_file(&f).unwrap() };
    assert_eq!(p.problem(), "");
    let parts = p.parts(None);
    assert_eq!(parts.len(), 2, "smoke and fire");
    assert!(matches!(parts[0].src, Source::Grid(_)) && parts[1].mat.emission > 0.0 && parts[0].mat.emission == 0.0);
    assert!((parts[0].lo - Vec3::splat(19.1)).abs().max_element() < 1e-3, "Scale applies: {:?}", parts[0].lo);
    let (lo, hi) = p.bounds().unwrap();
    let cage = p.build();
    cage.validate().unwrap();
    let b = cage.bounds().unwrap();
    assert!((b.min - lo).abs().max_element() < 0.1 && (b.max - hi).abs().max_element() < 0.1);
    assert_eq!(cage.polygons.len(), 12 * 6);
    // Fire 0 % and an empty density grid name drop their media.
    assert_eq!(VdbParams { fire: 0.0, ..p.clone() }.parts(None).len(), 1);
    assert!(VdbParams { density_grid: String::new(), fire: 0.0, ..p.clone() }.parts(None).is_empty());
    // A missing file keeps a 1 m cage and says why.
    let gone = VdbParams { path: "/nonexistent/smoke.vdb".into(), ..p.clone() };
    assert!(gone.parts(None).is_empty() && !gone.problem().is_empty() && gone.build().bounds().is_some());
    let back: VdbParams = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(back, p);
    assert_eq!(serde_json::from_str::<VdbParams>("{}").unwrap(), VdbParams::default());
}

#[test]
fn import_makes_a_volume_object_the_renderers_see() {
    let f = smoke_file("scene.vdb");
    let mut ed = crate::editor::Editor::default();
    let id = ed.import_vdb(&f).unwrap();
    let o = &ed.scene.objects[id];
    assert!(o.is_fx() && o.selected && o.name.starts_with("VDB"), "{}", o.name);
    let v = crate::render::volumes::Volumes::build(&ed.scene);
    assert_eq!(v.list.len(), 2, "smoke and fire");
    assert!(v.list.iter().all(|q| matches!(q.src, Source::Grid(_))));
    let p = Vec3::new(10.15, -10.15, 10.15);
    assert!(v.list[0].density(p) > 0.9, "inside the smoke (Y-up turned to Z-up)");
    assert!(ed.import_vdb("/nonexistent/none.vdb").is_err());
    assert_eq!(ed.scene.objects.len(), 1);
}
