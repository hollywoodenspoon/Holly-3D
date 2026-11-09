use super::*;

fn check(m: &EditablePoly, what: &str) {
    assert!(m.validate().is_ok(), "{what}: {:?}", m.validate());
    assert!(m.vertices.values().all(|v| v.position.is_finite()), "{what}: finite");
    assert!(m.signed_volume() > 0.0, "{what}: outward normals");
    assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())), "{what}: UVs");
}

#[test]
fn quad_sphere_counts() {
    let m = quad_sphere(4);
    assert_eq!(m.counts(), (6 * 16 + 2, 12 * 16, 6 * 16));
    assert!(m.is_closed());
    check(&m, "quad sphere");
}

#[test]
fn every_mode_builds_a_closed_valid_mesh() {
    for mode in GeoMode::ALL {
        let p = GeoGenParams::default_of(mode);
        let m = p.build();
        assert!(m.polygons.len() > 20, "{mode:?}");
        check(&m, mode.short());
        let b = m.bounds().unwrap();
        assert!(b.max.z <= p.max_height * 1.3 + 0.2, "{mode:?} height {}", b.max.z);
    }
}

#[test]
fn single_rock_keeps_quad_sphere_topology() {
    let p = GeoGenParams { count: 1, ..GeoGenParams::default_of(GeoMode::Rock) };
    let m = p.build();
    assert_eq!(m.counts(), quad_sphere(8).counts(), "cuts and noise move vertices only");
    assert!(m.is_closed());
    assert!(m.bounds().unwrap().min.z.abs() < 1e-4, "pivot at the base");
    // planar cuts make flat facets: many faces share an exact normal
    let mut groups: Vec<u32> = m.polygons.values().map(|p| p.smoothing_group).collect();
    groups.sort();
    groups.dedup();
    assert!(groups.len() > 1, "auto smooth keeps ridges sharp");
}

#[test]
fn seeds_are_deterministic() {
    let p = GeoGenParams::default_of(GeoMode::Crystal);
    let a: Vec<Vec3> = p.build().vertices.values().map(|v| v.position).collect();
    let b: Vec<Vec3> = p.build().vertices.values().map(|v| v.position).collect();
    assert_eq!(a, b);
    let c: Vec<Vec3> = GeoGenParams { seed: 2, ..p }.build().vertices.values().map(|v| v.position).collect();
    assert_ne!(a, c);
}

#[test]
fn crystal_spire_and_cluster_counts() {
    let s = spires::spire(0.1, 1.0, 0.3, &mut rng(1, 0));
    // 2 hex rings + welded apex; 6 body quads + 6 tip triangles + bottom cap
    assert_eq!(s.counts(), (13, 24, 13));
    let p = GeoGenParams { count: 4, side_shards: false, cluster_base: false, ..GeoGenParams::default_of(GeoMode::Crystal) };
    assert_eq!(p.build().counts(), (4 * 13, 4 * 24, 4 * 13));
}

#[test]
fn cliff_strata_counts() {
    let p = GeoGenParams { count: 3, ..GeoGenParams::default_of(GeoMode::Cliff) };
    let m = p.build();
    let n = cliff::CLIFF_SIDES;
    assert_eq!(m.counts(), (6 * n, 6 * n + 5 * n, 5 * n + 2), "3 strata x 2 rings, walls + ledges + 2 caps");
    assert!(m.is_closed());
}

#[test]
fn stalactite_hangs_down() {
    let p = GeoGenParams { hanging: true, ..GeoGenParams::default_of(GeoMode::Stalagmite) };
    let m = p.build();
    let b = m.bounds().unwrap();
    assert!(b.max.z < 0.1 && b.min.z < -0.9, "{b:?}");
    check(&m, "stalactite");
}

#[test]
fn modifier_grows_on_selected_polygons_and_deforms() {
    let base = crate::primitives::Primitive::default_of(crate::primitives::PrimitiveKind::Box).to_editable_poly();
    let mut m = base.clone();
    let top = m.polygons.keys().max_by(|&a, &b| m.poly_centroid(a).z.total_cmp(&m.poly_centroid(b).z)).unwrap();
    m.polygons[top].selected = true;
    let g = GeoGenMod::default();
    g.apply(&mut m);
    let one = g.params.build();
    assert_eq!(m.polygons.len(), base.polygons.len() + one.polygons.len(), "one cluster on the selected top");
    assert!(m.validate().is_ok());
    let mut all = base.clone();
    g.apply(&mut all);
    assert!(all.polygons.len() > m.polygons.len() + 4 * base.polygons.len(), "no selection: every face grows");
    let mut d = base.clone();
    GeoGenMod { deform: true, ..g }.apply(&mut d);
    assert_eq!(d.counts(), base.counts(), "deform keeps topology");
    assert!(d.vertices.values().zip(base.vertices.values()).any(|(a, b)| a.position != b.position));
}

#[test]
fn grow_cap_spreads_over_the_whole_mesh() {
    let base = crate::primitives::Primitive::default_of(crate::primitives::PrimitiveKind::Sphere).to_editable_poly();
    assert!(base.polygons.len() > MAX_GROWN * 2);
    let mut m = base.clone();
    GeoGenMod::default().apply(&mut m);
    // 3 spires of 13 polygons each, plus up to one 13-polygon side shard per spire
    let grown = m.polygons.len() - base.polygons.len();
    assert!((MAX_GROWN * 39..=MAX_GROWN * 78).contains(&grown), "capped at MAX_GROWN formations: {grown}");
    let b = m.bounds().unwrap();
    let sb = base.bounds().unwrap();
    assert!(b.max.z > sb.max.z && b.min.z < sb.min.z, "formations reach both poles");
}
