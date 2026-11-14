use super::mesh::{CLOUD_ID, verts_per_puff};
use super::*;
use bevy::math::Vec3;
use std::f32::consts::PI;

#[test]
fn every_type_builds_a_seeded_cloud_on_its_base() {
    for k in CloudType::ALL {
        let p = CloudParams::of(k);
        let s = p.shape();
        assert!(s.count_at(0) >= 5, "{k:?} has body puffs");
        assert_eq!(s, p.shape(), "{k:?} is deterministic");
        let other = CloudParams { seed: 9, ..p.clone() }.shape();
        assert_ne!(s, other, "{k:?}: the seed changes the layout");
        let (lo, hi) = s.bounds();
        assert!(lo.z > -0.35 * p.height, "{k:?}: base near z = 0 (got {})", lo.z);
        assert!(hi.z > 0.75 * p.height && hi.z < 1.3 * p.height, "{k:?}: top near the height ({} vs {})", hi.z, p.height);
        let span = (hi - lo).truncate().max_element();
        assert!(span > 0.5 * p.size && span < 1.6 * p.size, "{k:?}: footprint near the size ({span} vs {})", p.size);
    }
}

#[test]
fn detail_levels_grow_puffs_on_puffs() {
    let base = CloudParams::of(CloudType::Cumulus);
    let one = CloudParams { detail: 1, ..base.clone() }.shape();
    assert_eq!(one.count_at(1), 0);
    let three = CloudParams { detail: 3, ..base.clone() }.shape();
    assert!(three.count_at(1) > one.count_at(0) * 2, "level 1 detail");
    assert!(three.count_at(2) > three.count_at(1), "level 2 detail");
    let small = three.puffs.iter().filter(|q| q.level == 2).map(|q| q.r).fold(0.0, f32::max);
    let big = three.puffs.iter().filter(|q| q.level == 0).map(|q| q.r).fold(f32::INFINITY, f32::min);
    assert!(small < big, "detail puffs are smaller than body puffs");
}

#[test]
fn preview_mesh_counts_material_and_flat_base() {
    for k in CloudType::ALL {
        let p = CloudParams { base_flatness: 1.0, ..CloudParams::of(k) };
        let s = p.shape();
        let m = p.build();
        let want: usize = s.puffs.iter().map(|q| verts_per_puff(q.level)).sum();
        assert_eq!(m.vertices.len(), want, "{k:?} vertex count");
        let faces: usize = s.puffs.iter().map(|q| { let (lon, lat) = mesh::segments(q.level); lon * lat }).sum();
        assert_eq!(m.polygons.len(), faces, "{k:?} polygon count");
        assert!(m.polygons.values().all(|f| f.material_id == CLOUD_ID && f.corners.len() >= 3));
        assert!(m.polygons.values().flat_map(|f| f.corners.iter()).all(|c| c.uv.is_finite() && (0.0..=1.0).contains(&c.uv.x)));
        let lo = m.vertices.values().map(|v| v.position.z).fold(f32::INFINITY, f32::min);
        assert!(lo > -1e-3, "{k:?}: Base Flatness 1 cuts the underside flat ({lo})");
        assert!(m.vertices.values().all(|v| v.position.is_finite()));
        let n = m.vertex_normals();
        assert!(n.values().all(|v| v.is_finite() && (v.length() - 1.0).abs() < 1e-3), "{k:?} normals unit");
    }
}

#[test]
fn wind_shear_leans_the_top_downwind() {
    let calm = CloudParams { shear: 0.0, ..CloudParams::of(CloudType::Congestus) }.shape();
    let windy = CloudParams { shear: 1.0, shear_dir: 90.0, ..CloudParams::of(CloudType::Congestus) }.shape();
    let top_y = |s: &CloudShape| {
        let h = s.bounds().1.z;
        let top: Vec<_> = s.puffs.iter().filter(|q| q.c.z > 0.7 * h).collect();
        top.iter().map(|q| q.c.y).sum::<f32>() / top.len().max(1) as f32
    };
    assert!(top_y(&windy) > top_y(&calm) + 50.0, "the top leans toward +Y (90 deg)");
}

#[test]
fn cumulonimbus_has_a_wide_flat_anvil() {
    let p = CloudParams::of(CloudType::Cumulonimbus);
    let s = p.shape();
    let anvil: Vec<_> = s.puffs.iter().filter(|q| q.level == 0 && q.sq < 0.5).collect();
    assert!(anvil.len() >= 8, "anvil puffs");
    assert!(anvil.iter().all(|q| q.c.z > 0.6 * p.height), "anvil sits high");
    let tower_w = s.puffs.iter().filter(|q| q.level == 0 && q.sq >= 0.99).map(|q| q.c.truncate().length() + q.r).fold(0.0, f32::max);
    let anvil_w = anvil.iter().map(|q| q.c.truncate().length() + q.r).fold(0.0, f32::max);
    assert!(anvil_w > tower_w, "the anvil spreads wider than the tower");
}

#[test]
fn field_covers_its_layer_without_overlaps() {
    let f = CloudFieldParams::default();
    let pl = f.placements();
    assert!(pl.len() >= 10, "{} clouds", pl.len());
    assert_eq!(pl, f.placements(), "deterministic");
    let area: f32 = pl.iter().map(|p| PI * (p.params.size * 0.4).powi(2)).sum();
    let cov = area / (f.width * f.depth);
    assert!((cov - f.coverage).abs() < 0.12, "coverage {cov} vs {}", f.coverage);
    for (i, a) in pl.iter().enumerate() {
        assert!(a.offset.x.abs() <= f.width * 0.5 && a.offset.y.abs() <= f.depth * 0.5);
        assert!((a.offset.z - f.base_height).abs() <= f.thickness * 0.06);
        assert!(a.params.height <= f.thickness + 1e-3, "regular clouds fit the layer");
        for b in &pl[i + 1..] {
            let gap = a.offset.truncate().distance(b.offset.truncate());
            assert!(gap >= (a.params.size + b.params.size) * 0.4 * 0.75 - 1e-3, "blue-noise spacing");
        }
    }
    let seeds: std::collections::HashSet<_> = pl.iter().map(|p| p.params.seed).collect();
    assert!(seeds.len() > pl.len() / 2, "each cloud has its own seed");
}

#[test]
fn field_type_mix_controls_the_types() {
    let only_sc = CloudFieldParams { mix: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], ..Default::default() };
    assert!(only_sc.placements().iter().all(|p| p.params.kind == CloudType::Stratocumulus));
    let mixed = CloudFieldParams { mix: [1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], coverage: 0.5, ..Default::default() };
    let kinds: std::collections::HashSet<_> = mixed.placements().iter().map(|p| p.params.kind).collect();
    assert!(kinds.contains(&CloudType::Cumulus) && kinds.contains(&CloudType::Congestus));
    assert!(CloudFieldParams { mix: [0.0; 8], ..Default::default() }.placements().is_empty());
    let none = CloudFieldParams { coverage: 0.0, ..Default::default() };
    assert!(none.placements().is_empty() && none.build().vertices.is_empty());
}

#[test]
fn field_mesh_sits_on_the_layer() {
    let f = CloudFieldParams { width: 1500.0, depth: 1500.0, detail: 1, ..Default::default() };
    let m = f.build();
    assert!(!m.vertices.is_empty());
    let lo = m.vertices.values().map(|v| v.position.z).fold(f32::INFINITY, f32::min);
    assert!(lo > f.base_height - 0.2 * f.thickness, "the field floats at its base height ({lo})");
    assert!(m.vertices.values().all(|v| v.position.is_finite() && v.position.distance(Vec3::ZERO) < 1e5));
}

#[test]
fn altocumulus_is_a_sheet_of_many_small_puffs() {
    let p = CloudParams::of(CloudType::Altocumulus);
    let s = p.shape();
    let body: Vec<_> = s.puffs.iter().filter(|q| q.level == 0).collect();
    assert!(body.len() >= 50, "many puffs ({})", body.len());
    assert!(body.iter().all(|q| q.r < p.size * 0.06), "each puff is small");
}

#[test]
fn mammatus_hangs_pouches_below_its_layer() {
    let p = CloudParams::of(CloudType::Mammatus);
    let s = p.shape();
    let body: Vec<_> = s.puffs.iter().filter(|q| q.level == 0).collect();
    let low = body.iter().filter(|q| q.c.z < p.height * 0.35).count();
    let high = body.iter().filter(|q| q.c.z > p.height * 0.7).count();
    assert!(low >= 10 && high >= 10, "pouches ({low}) under a layer ({high})");
    assert!(CloudType::Mammatus.stormy() && !CloudType::Lenticular.stormy());
}

#[test]
fn lenticular_lenses_are_smooth_and_shrink_upward() {
    let p = CloudParams::of(CloudType::Lenticular);
    assert_eq!(p.detail, 1, "smooth: no puffs on puffs");
    let s = p.shape();
    let width_at = |z0: f32, z1: f32| s.puffs.iter().filter(|q| q.c.z >= z0 && q.c.z < z1).map(|q| q.c.truncate().length()).fold(0.0f32, f32::max);
    let h = p.height;
    let (a, b, c) = (width_at(0.0, h / 3.0), width_at(h / 3.0, 2.0 * h / 3.0), width_at(2.0 * h / 3.0, h * 1.5));
    assert!(a > b && b > c && c > 0.0, "stacked lenses shrink upward ({a} {b} {c})");
}

#[test]
fn cloud_bank_is_a_long_narrow_wall() {
    let p = CloudParams { shear: 0.0, ..CloudParams::of(CloudType::CloudBank) };
    let (lo, hi) = p.shape().bounds();
    let d = hi - lo;
    assert!(d.x > 3.0 * d.y, "long along X ({} vs {})", d.x, d.y);
}

#[test]
fn old_field_with_four_type_weights_loads() {
    let json = r#"{"seed":3,"mix":[0.5,0.0,0.0,1.0]}"#;
    let f: CloudFieldParams = serde_json::from_str(json).unwrap();
    assert_eq!(f.mix, [0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(f.seed, 3);
}
