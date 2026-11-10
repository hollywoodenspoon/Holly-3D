//! Terrain and Ocean generator tests: grid topology, Material IDs per
//! biome, determinism, erosion and Gerstner crest foam.

use super::ocean::{ID_FOAM, ID_WATER};
use super::terrain_ids::{ID_CLIFF, ID_GRASS, ID_SAND, ID_SNOW};
use super::*;
use crate::mesh::EditablePoly;
use crate::primitives::Primitive;

fn counts(m: &EditablePoly) -> (usize, usize, usize) {
    (m.vertices.len(), m.edges.len(), m.polygons.len())
}

fn grid_counts(segs: usize) -> (usize, usize, usize) {
    ((segs + 1) * (segs + 1), 2 * segs * (segs + 1), segs * segs)
}

fn id_histogram(m: &EditablePoly) -> [usize; 8] {
    let mut h = [0; 8];
    m.polygons.values().for_each(|p| h[p.material_id.min(7) as usize] += 1);
    h
}

fn small(biome: Biome) -> TerrainParams {
    TerrainParams { segments: 48, ..TerrainParams::of(biome) }
}

#[test]
fn grid_topology_and_uvs() {
    let (mut m, ids) = grid(10.0, 6.0, 7, |x, y| x + y);
    m.finalize();
    assert_eq!(counts(&m), grid_counts(7));
    assert_eq!(ids.len(), 64);
    m.validate().unwrap();
    assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| (0.0..=1.0).contains(&c.uv.x) && (0.0..=1.0).contains(&c.uv.y))));
    // Faces point up (+Z) on a flat grid.
    let (flat, _) = grid(4.0, 4.0, 2, |_, _| 0.0);
    assert!(flat.polygons.keys().all(|p| flat.poly_normal(p).z > 0.99));
}

#[test]
fn every_biome_builds_valid_grid_with_terrain_ids() {
    for b in Biome::ALL {
        let p = small(b);
        let m = p.build();
        assert_eq!(counts(&m), grid_counts(48), "{b:?}");
        m.validate().unwrap();
        assert!(m.vertices.values().all(|v| v.position.is_finite()), "{b:?}");
        assert!(m.polygons.values().all(|p| (ID_GRASS..=ID_SAND).contains(&p.material_id)), "{b:?} IDs 1-4");
        let zmax = m.vertices.values().map(|v| v.position.z).fold(f32::MIN, f32::max);
        assert!(zmax > 0.0 && zmax <= p.max_elevation * 1.3, "{b:?} height {zmax}");
    }
}

#[test]
fn biome_ids_match_landforms() {
    let mountains = id_histogram(&small(Biome::JaggedMountains).build());
    assert!(mountains[ID_CLIFF as usize] > 0, "mountains have cliffs {mountains:?}");
    assert!(mountains[ID_SNOW as usize] > 0, "mountains have snow caps {mountains:?}");
    let savannah = id_histogram(&small(Biome::Savannah).build());
    assert!(savannah[ID_SAND as usize] > 0, "savannah riverbeds {savannah:?}");
    let dunes = id_histogram(&small(Biome::Dunes).build());
    let total: usize = dunes.iter().sum();
    assert!(dunes[ID_SAND as usize] * 2 > total, "dunes are mostly sand {dunes:?}");
    let plains = id_histogram(&small(Biome::Plains).build());
    assert!(plains[ID_GRASS as usize] > 0, "plains are grassy {plains:?}");
}

#[test]
fn ids_toggle_off_and_thresholds() {
    let p = TerrainParams { ids: TerrainIds { enabled: false, ..TerrainIds::default() }, ..small(Biome::JaggedMountains) };
    assert!(p.build().polygons.values().all(|p| p.material_id == 1));
    let ids = TerrainIds::default();
    assert_eq!(ids.id_for(80.0, 0.5), ID_CLIFF);
    assert_eq!(ids.id_for(2.0, 1.0), ID_SNOW);
    assert_eq!(ids.id_for(2.0, 0.0), ID_SAND);
    assert_eq!(ids.id_for(2.0, 0.5), ID_GRASS);
}

#[test]
fn terrain_is_deterministic_per_seed() {
    let a = small(Biome::RollingHills);
    let pos = |m: &EditablePoly| m.vertices.values().map(|v| v.position).collect::<Vec<_>>();
    assert_eq!(pos(&a.build()), pos(&a.build()));
    let b = TerrainParams { seed: 77, ..a.clone() };
    assert_ne!(pos(&a.build()), pos(&b.build()));
}

#[test]
fn erosion_softens_steps_and_terraces_quantize() {
    let raw = TerrainParams { erosion: 0.0, ..small(Biome::JaggedMountains) }.heightmap();
    let eroded = TerrainParams { erosion: 1.0, ..small(Biome::JaggedMountains) }.heightmap();
    assert!(eroded.max_step() < raw.max_step(), "{} !< {}", eroded.max_step(), raw.max_step());
    assert!(eroded.h.iter().all(|h| h.is_finite()));
    assert_eq!(terrain::terrace(0.0, 4), 0.0);
    assert!((terrain::terrace(1.0, 4) - 1.0).abs() < 1e-4);
}

#[test]
fn terrain_modifier_displaces_plane_keeping_topology() {
    let mut m = Primitive::Plane { width: 50.0, length: 50.0, width_segs: 12, length_segs: 12 }.to_editable_poly();
    let before = counts(&m);
    TerrainMod { params: small(Biome::RollingHills) }.apply(&mut m);
    assert_eq!(counts(&m), before);
    assert!(m.vertices.values().any(|v| v.position.z.abs() > 0.1));
    assert!(m.polygons.values().all(|p| (1..=4).contains(&p.material_id)));
}

#[test]
fn ocean_topology_foam_ids_and_colors() {
    let p = OceanParams { segments: 64, wave_height: 6.0, choppiness: 1.0, foam_threshold: 0.4, ..OceanParams::default() };
    let m = p.build();
    assert_eq!(counts(&m), grid_counts(64));
    m.validate().unwrap();
    assert!(m.vertices.values().all(|v| v.position.is_finite() && v.color.is_finite()));
    let h = id_histogram(&m);
    assert!(h[ID_WATER as usize] > 0 && h[ID_FOAM as usize] > 0, "water and foam {h:?}");
    assert_eq!(h[ID_WATER as usize] + h[ID_FOAM as usize], 64 * 64);
    // Foam whitens the vertex color.
    let max_r = m.vertices.values().map(|v| v.color.x).fold(0.0, f32::max);
    assert!(max_r > 0.5, "foam is white {max_r}");
    // Without Auto-Crest Foam everything is water.
    let calm = OceanParams { auto_foam: false, ..p.clone() }.build();
    assert!(calm.polygons.values().all(|p| p.material_id == ID_WATER));
    // Crests stay inside the summed amplitude.
    let zmax = m.vertices.values().map(|v| v.position.z.abs()).fold(0.0, f32::max);
    assert!(zmax <= p.crest() + 1e-3);
}

#[test]
fn ocean_time_scrubs_waves() {
    let p = OceanParams { segments: 16, ..OceanParams::default() };
    let s0 = p.sample(bevy::math::Vec2::new(3.0, 4.0)).offset;
    let s1 = OceanParams { time: 2.0, ..p.clone() }.sample(bevy::math::Vec2::new(3.0, 4.0)).offset;
    assert_ne!(s0, s1);
    let flat = OceanParams { wave_height: 0.0, ..p }.build();
    assert!(flat.vertices.values().all(|v| v.position.z.abs() < 1e-5));
}

#[test]
fn ocean_modifier_keeps_topology() {
    let mut m = Primitive::Plane { width: 80.0, length: 80.0, width_segs: 20, length_segs: 20 }.to_editable_poly();
    let before = counts(&m);
    OceanMod { params: OceanParams::default() }.apply(&mut m);
    assert_eq!(counts(&m), before);
    assert!(m.polygons.values().all(|p| p.material_id == ID_WATER || p.material_id == ID_FOAM));
}

#[test]
fn world_gen_dispatch_and_packs() {
    for k in WorldKind::ALL {
        let w = WorldGen::default_of(k);
        assert_eq!(w.kind(), k);
        assert_eq!(w.short(), k.label());
    }
    assert!(hash(1, 2) >= 0.0 && hash(1, 2) < 1.0);
    assert_eq!(hash(5, 9), hash(5, 9));
}
