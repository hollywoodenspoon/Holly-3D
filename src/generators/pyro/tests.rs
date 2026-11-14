use super::layout::{MAX_PUFFS, spark_stride};
use super::sparks::arc;
use super::*;
use crate::generators::clouds::mesh::verts_per_puff;

fn share(p: &PyroParams, m: Medium) -> f32 {
    // Share among the fire and smoke body (not sparks, dust or vapour).
    let q: Vec<_> = p.puffs().into_iter().filter(|x| matches!(x.medium, Medium::Fire | Medium::Smoke)).collect();
    q.iter().filter(|x| x.medium == m).count() as f32 / q.len().max(1) as f32
}

fn mean_z(p: &PyroParams) -> f32 {
    let q = p.puffs();
    q.iter().map(|x| x.c.z).sum::<f32>() / q.len() as f32
}

#[test]
fn every_type_lays_seeded_valid_puffs() {
    for k in PyroKind::ALL {
        let p = PyroParams::of(k);
        let q = p.puffs();
        assert!(q.len() >= 30 && q.len() <= MAX_PUFFS, "{k:?}: {} puffs", q.len());
        assert_eq!(q, p.puffs(), "{k:?} is deterministic");
        assert_ne!(q, PyroParams { seed: 9, ..p.clone() }.puffs(), "{k:?}: the seed changes it");
        assert!(q.iter().all(|x| x.c.is_finite() && x.r > 0.0 && x.sq > 0.0 && (0.05..=1.0).contains(&x.w)), "{k:?} valid");
        if let Some(x) = q.iter().find(|x| x.c.z <= -1e-3) { panic!("{k:?} below the ground: {x:?}") }
        let early = PyroParams { stage: 0.05, ..p.clone() };
        assert!(share(&early, Medium::Fire) > 0.3, "{k:?} starts as fire");
        for m in Medium::ALL {
            assert!(q.iter().all(|x| x.medium != m) || p.mat(m).is_some(), "{k:?}: {m:?} has a material");
        }
    }
}

#[test]
fn stage_grows_lifts_and_cools_the_fireball() {
    let p = PyroParams::of(PyroKind::Fireball);
    let (a, b) = (PyroParams { stage: 0.05, ..p.clone() }, PyroParams { stage: 0.9, ..p.clone() });
    assert!(share(&a, Medium::Fire) > 0.8 && share(&b, Medium::Fire) < 0.05, "fire cools into smoke");
    assert!(mean_z(&b) > mean_z(&a) + p.size, "it rises");
    let width = |p: &PyroParams| p.puffs().iter().map(|x| x.c.x.abs() + x.r).fold(0.0, f32::max);
    assert!(width(&b) > 1.5 * width(&a), "it grows");
    let (ma, mb) = (a.mat(Medium::Smoke).unwrap(), b.mat(Medium::Smoke).unwrap());
    assert!(mb.albedo().length() < ma.albedo().length(), "smoke darkens as it matures");
    assert!(b.mat(Medium::Fire).unwrap().emission < a.mat(Medium::Fire).unwrap().emission);
}

#[test]
fn mushroom_cloud_goes_through_its_stages() {
    let at = |s: f32| PyroParams { stage: s, ..PyroParams::of(PyroKind::Mushroom) };
    let size = at(0.0).size;
    let has = |p: &PyroParams, m: Medium| p.puffs().iter().any(|x| x.medium == m);
    let p0 = at(0.0);
    assert!(!has(&p0, Medium::Vapor) && !has(&p0, Medium::Dust), "a bare ground fireball");
    assert!(p0.puffs().iter().all(|x| x.c.z < 2.5 * size));
    let p6 = at(0.6);
    assert!(has(&p6, Medium::Vapor), "condensation skirt");
    let stem = p6.puffs().iter().filter(|x| x.c.truncate().length() < size * 0.5 && x.c.z < size * 3.0).count();
    assert!(stem >= 9, "a stem: {stem}");
    assert!(p6.puffs().iter().map(|x| x.c.z).fold(0.0, f32::max) > 4.0 * size, "the cap has risen");
    let p1 = at(1.0);
    assert!(!has(&p1, Medium::Fire), "mature cloud has no fire left");
    let ice = p1.puffs().iter().filter(|x| x.medium == Medium::Vapor).all(|x| x.sq < 0.5 && x.c.z > 5.0 * size);
    assert!(ice && has(&p1, Medium::Vapor), "ice cap on top");
    let surge = p1.puffs().iter().filter(|x| x.medium == Medium::Dust && x.c.z < size).map(|x| x.c.truncate().length()).fold(0.0, f32::max);
    assert!(surge > 4.0 * size, "base surge spreads: {surge}");
}

#[test]
fn sparks_fly_on_ballistic_arcs_and_cool() {
    let (p0, v0) = (Vec3::new(1.0, 2.0, 3.0), Vec3::new(5.0, 0.0, 10.0));
    assert!(arc(p0, v0, 1.2, 0.0).distance(p0) < 1e-5);
    assert!(arc(p0, v0, 1.2, 1e-3).distance(p0 + v0 * 1e-3) < 1e-4, "starts at the launch velocity");
    assert!(arc(p0, Vec3::ZERO, 1.2, 3.0).z < p0.z - 5.0, "gravity pulls down");
    let p = PyroParams { stage: 0.15, ..PyroParams::of(PyroKind::AerialExplosion) };
    let n = |p: &PyroParams| p.puffs().iter().filter(|x| x.medium == Medium::Sparks).count();
    assert!(n(&p) > 1000, "{}", n(&p));
    assert!(n(&PyroParams { sparks: 100, ..p.clone() }) < n(&p) / 5);
    let w = |p: &PyroParams| {
        let q: Vec<_> = p.puffs().into_iter().filter(|x| x.medium == Medium::Sparks).collect();
        q.iter().map(|x| x.w).sum::<f32>() / q.len().max(1) as f32
    };
    assert!(w(&PyroParams { spark_cooling: 1.0, ..p.clone() }) < w(&PyroParams { spark_cooling: 0.0, ..p.clone() }));
    let reach = |p: &PyroParams| p.puffs().iter().filter(|x| x.medium == Medium::Sparks).map(|x| x.c.distance(super::layout::origin(p))).fold(0.0, f32::max);
    assert!(reach(&PyroParams { spark_speed: 80.0, ..p.clone() }) > reach(&p), "faster sparks fly further");
    assert_eq!(n(&PyroParams { stage: 0.0, ..p.clone() }), 0, "no sparks before the blast");
    let ramp = |c| PyroParams { spark_color: c, ..p.clone() }.mat(Medium::Sparks).unwrap().ramp;
    assert_ne!(ramp(SparkColor::Orange), ramp(SparkColor::WhiteGold));
}

#[test]
fn burning_debris_leaves_smoke_with_a_glowing_tip() {
    let p = PyroParams { stage: 0.15, sparks: 0, debris: 8, ..PyroParams::of(PyroKind::GroundExplosion) };
    let base = PyroParams { debris: 0, ..p.clone() };
    let extra = |m: Medium| p.puffs().iter().filter(|x| x.medium == m).count() - base.puffs().iter().filter(|x| x.medium == m).count();
    assert!(extra(Medium::Smoke) >= 8 * 5, "smoke trails");
    assert!(extra(Medium::Fire) >= 4, "glowing tips");
}

#[test]
fn preview_mesh_counts_and_ids() {
    for k in PyroKind::ALL {
        let p = PyroParams::of(k);
        let q = p.puffs();
        let sparks = q.iter().filter(|x| x.medium == Medium::Sparks).count();
        let shown = q.len() - sparks + sparks.div_ceil(spark_stride(&q));
        let m = p.build();
        assert_eq!(m.vertices.len(), shown * verts_per_puff(2), "{k:?} vertex count");
        assert_eq!(m.polygons.len(), shown * 40, "{k:?} polygon count");
        assert!(m.polygons.values().all(|f| (1..=3).contains(&f.material_id)), "{k:?} pack IDs");
        let n = m.vertex_normals();
        assert!(n.values().all(|v| v.is_finite() && (v.length() - 1.0).abs() < 1e-3), "{k:?} normals unit");
        assert!(m.polygons.values().flat_map(|f| f.corners.iter()).all(|c| c.uv.is_finite()));
    }
}

#[test]
fn explosion_light_follows_the_fire() {
    let p = PyroParams { stage: 0.2, ..PyroParams::of(PyroKind::Fireball) };
    let (c, col) = p.light().expect("a light while it burns");
    let fire: Vec<_> = p.puffs().into_iter().filter(|x| x.medium == Medium::Fire).collect();
    let mid = fire.iter().map(|x| x.c).sum::<Vec3>() / fire.len() as f32;
    assert!(c.distance(mid) < p.size * 0.5, "at the fire's centre");
    assert!(col.x > col.z && col.min_element() >= 0.0, "warm");
    let (_, half) = PyroParams { light_strength: 0.5, ..p.clone() }.light().unwrap();
    assert!((half.x * 2.0 - col.x).abs() < 1e-3 * col.x);
    assert!(PyroParams { light: false, ..p.clone() }.light().is_none());
    assert!(PyroParams { stage: 1.0, ..p.clone() }.light().is_none(), "out when the fire is gone");
    let late = PyroParams { stage: 0.6, ..p.clone() }.light().map_or(0.0, |l| l.1.x);
    assert!(late < col.x, "dims as it cools");
}
