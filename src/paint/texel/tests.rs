use super::*;

fn red(a: u8) -> Texel {
    Texel { rgba: [255, 0, 0, a], ..Default::default() }
}

#[test]
fn unpainted_images_allocate_nothing_and_writes_allocate_one_tile() {
    let mut t = Tiles::<Texel>::new(256);
    assert_eq!((t.size(), t.tile_count(), t.allocated(), t.bytes()), (256, 16, 0, 0));
    assert_eq!(t.get(100, 100), Texel::default());
    // Writing the default into an empty tile stays free.
    t.set(5, 5, Texel::default());
    assert!(t.is_empty());
    t.set(70, 3, red(255));
    assert_eq!((t.allocated(), t.bytes()), (1, 64 * 64 * 12), "one 64 x 64 tile of 12-byte texels");
    assert_eq!(t.get(70, 3), red(255));
    assert_eq!(t.get(70 + 256, 3 - 256), red(255), "coordinates wrap");
    assert_eq!(Tiles::<u8>::new(100).size(), 128, "sizes round up to whole tiles");
}

#[test]
fn snapshots_share_tiles_and_a_write_copies_only_its_tile() {
    let mut t = Tiles::<Texel>::new(256);
    for (x, y) in [(3, 3), (70, 3), (200, 200)] {
        t.set(x, y, red(255));
    }
    assert_eq!(t.allocated(), 3);
    // Ctrl+Z snapshot: pointer copies only.
    let snapshot = t.clone();
    assert_eq!(t.shared_with(&snapshot), 3);
    assert!(t.changed_tiles(&snapshot).is_empty());
    // One stroke touches one tile: only that tile is copied.
    t.set(71, 4, red(128));
    assert_eq!(t.changed_tiles(&snapshot), vec![1], "tile (1, 0) only");
    assert_eq!(t.shared_with(&snapshot), 2, "the other two tiles are still shared");
    // The snapshot is untouched: undo restores it exactly.
    assert_eq!(snapshot.get(71, 4), Texel::default());
    assert_eq!(snapshot.get(70, 3), red(255));
    // Writing the same value again copies nothing.
    let again = t.clone();
    t.set(71, 4, red(128));
    assert_eq!(t.shared_with(&again), 3);
    // A new tile shows up as changed too; a different size differs everywhere.
    t.set(130, 130, red(1));
    assert_eq!(t.changed_tiles(&snapshot), vec![1, 10]);
    assert_eq!(t.changed_tiles(&Tiles::new(128)).len(), 16);
}

#[test]
fn bilinear_sampling_weights_by_coverage_and_skips_empty_tiles() {
    let mut t = Tiles::<Texel>::new(64);
    assert!(t.sample(Vec2::splat(0.5)).is_none(), "nothing painted");
    t.set(10, 10, Texel { rgba: [255, 128, 0, 255], pbr: [200, 0, 64, 0], cov: [255, 0, 255, 0] });
    // At the texel centre: exactly that texel.
    let uv = Vec2::new(10.5 / 64.0, 1.0 - 10.5 / 64.0);
    let s = t.sample(uv).unwrap();
    assert!((s.a - 1.0).abs() < 1e-5 && (s.color - Vec3::new(1.0, 128.0 / 255.0, 0.0)).length() < 1e-5);
    assert!((s.pbr[ROUGH] - 200.0 / 255.0).abs() < 1e-5 && s.cov[METAL] == 0.0 && (s.pbr[HEIGHT] - 64.0 / 255.0).abs() < 1e-5);
    // Half a texel away: half the coverage, but the colour is not darkened
    // by the unpainted neighbour.
    let half = t.sample(uv + Vec2::new(0.5 / 64.0, 0.0)).unwrap();
    assert!((half.a - 0.5).abs() < 1e-4 && (half.color.x - 1.0).abs() < 1e-4, "{half:?}");
    assert!((half.faded(0.5).a - 0.25).abs() < 1e-4);
    // Masks.
    let mut m = Tiles::<u8>::new(64);
    assert_eq!(m.sample(uv), 0.0);
    m.set(10, 10, 255);
    assert!((m.sample(uv) - 1.0).abs() < 1e-5);
}

#[test]
fn fill_with_covers_everything_and_keeps_empty_tiles_free() {
    let mut m = Tiles::<u8>::new(128);
    m.fill_with(|x, _| if x < 64 { 255 } else { 0 });
    assert_eq!(m.allocated(), 2, "the right half stays unallocated");
    assert_eq!((m.get(0, 0), m.get(63, 127), m.get(64, 0)), (255, 255, 0));
    m.clear();
    assert!(m.is_empty());
}

#[test]
fn tiles_round_trip_through_the_scene_format_storing_only_painted_tiles() {
    let mut t = Tiles::<Texel>::new(256);
    t.set(70, 3, Texel { rgba: [1, 2, 3, 4], pbr: [5, 6, 7, 8], cov: [9, 10, 11, 12] });
    let text = ron::to_string(&t).unwrap();
    assert!(text.len() < 600, "one mostly empty tile compresses small: {} bytes", text.len());
    let back: Tiles<Texel> = ron::from_str(&text).unwrap();
    assert_eq!((back.size(), back.allocated()), (256, 1));
    assert_eq!(back.get(70, 3), t.get(70, 3));
    let mut m = Tiles::<u8>::new(64);
    m.set(1, 2, 77);
    let back: Tiles<u8> = ron::from_str(&ron::to_string(&m).unwrap()).unwrap();
    assert_eq!(back.get(1, 2), 77);
    assert!(ron::from_str::<Tiles<u8>>("(size:0,tiles:[])").is_err());
}
