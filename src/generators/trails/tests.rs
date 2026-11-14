use super::path::Path;
use super::puffs::{FLAME_ID, MAX_PUFFS, SMOKE_ID};
use super::*;
use crate::generators::clouds::mesh::verts_per_puff;
use bevy::math::Vec3;

#[test]
fn every_type_lays_seeded_puffs_with_the_right_media() {
    for k in TrailKind::ALL {
        let p = TrailParams::of(k);
        let q = p.puffs();
        assert!(q.len() >= 10 && q.len() <= MAX_PUFFS, "{k:?}: {} puffs", q.len());
        assert_eq!(q, p.puffs(), "{k:?} is deterministic");
        assert_ne!(q, TrailParams { seed: 7, ..p.clone() }.puffs(), "{k:?}: the seed changes it");
        assert!(q.iter().all(|x| x.c.is_finite() && x.r > 0.0 && (0.05..=1.0).contains(&x.w)), "{k:?} valid puffs");
        let flames = q.iter().any(|x| x.flame);
        assert_eq!(flames, p.flame_mat().is_some(), "{k:?}: flame puffs match the flame medium");
        assert_eq!(q.iter().any(|x| !x.flame), p.smoke_mat().is_some(), "{k:?}: smoke puffs match the smoke medium");
    }
}

#[test]
fn contrail_has_a_gap_widens_and_fades() {
    let p = TrailParams { breakup: 0.0, spread: 0.0, ..TrailParams::of(TrailKind::Contrail) };
    let q = p.puffs();
    // Source at the origin, trail back along -X.
    assert!(q.iter().all(|x| -x.c.x >= p.start_gap - 1e-3), "nothing inside the start gap");
    assert!(q.iter().all(|x| -x.c.x <= p.length + p.end_radius), "within the length");
    let near: Vec<_> = q.iter().filter(|x| -x.c.x < p.start_gap + 100.0).collect();
    let far: Vec<_> = q.iter().filter(|x| -x.c.x > p.length - 200.0).collect();
    let avg = |v: &[&TrailPuff], f: fn(&TrailPuff) -> f32| v.iter().map(|x| f(x)).sum::<f32>() / v.len() as f32;
    assert!(avg(&far, |x| x.r) > 4.0 * avg(&near, |x| x.r), "widens with age");
    assert!(avg(&far, |x| x.w) < 0.6 * avg(&near, |x| x.w), "fades with age");
    // One trail per engine, side by side.
    let ys: Vec<f32> = near.iter().map(|x| x.c.y).collect();
    assert!(ys.iter().any(|&y| y > 5.0) && ys.iter().any(|&y| y < -5.0), "two engines");
    let one = TrailParams { sources: 1, ..p.clone() }.puffs();
    assert!(one.len() * 2 <= q.len() + 2);
}

#[test]
fn dissipation_and_picked_path_bound_the_trail() {
    let p = TrailParams { dissipation: 300.0, ..TrailParams::of(TrailKind::DamageSmoke) };
    assert!(p.puffs().iter().all(|x| x.c.length() < 300.0 + 3.0 * p.end_radius));
    let path = vec![Vec3::ZERO, Vec3::new(0.0, 50.0, 0.0), Vec3::new(0.0, 50.0, 80.0)];
    let p = TrailParams { path: path.clone(), spread: 0.0, breakup: 0.0, ..TrailParams::of(TrailKind::DamageSmoke) };
    let q = p.puffs();
    assert!(q.iter().any(|x| x.c.z > 60.0), "follows the picked path up");
    assert!(q.iter().all(|x| x.c.x.abs() < 3.0 * p.end_radius), "stays on the path");
    assert!((Path::new(&path).unwrap().len() - 130.0).abs() < 1e-3);
}

#[test]
fn vortices_strengthen_in_turns() {
    let straight = TrailParams { turn: 0.0, breakup: 0.0, ..TrailParams::of(TrailKind::WingtipVortices) };
    let turning = TrailParams { turn: 60.0, ..straight.clone() };
    let mean = |p: &TrailParams| {
        let q = p.puffs();
        q.iter().map(|x| x.w).sum::<f32>() / q.len() as f32
    };
    assert!(mean(&turning) > mean(&straight) * 1.3);
}

#[test]
fn afterburner_throttle_and_diamonds() {
    let p = TrailParams::of(TrailKind::Afterburner);
    let len = |p: &TrailParams| p.puffs().iter().map(|x| -x.c.x).fold(0.0, f32::max);
    assert!(len(&TrailParams { throttle: 0.2, ..p.clone() }) < 0.75 * len(&p), "throttle shortens the flame");
    let hot = p.puffs().iter().filter(|x| x.w > 0.6).count();
    assert!(hot >= p.diamonds as usize, "a hot knot per shock diamond");
    assert!(p.flame_mat().unwrap().emission > TrailParams { throttle: 0.0, ..p }.flame_mat().unwrap().emission);
}

#[test]
fn preview_mesh_counts_and_ids() {
    for k in TrailKind::ALL {
        let p = TrailParams::of(k);
        let q = p.puffs();
        let m = p.build();
        assert_eq!(m.vertices.len(), q.len() * verts_per_puff(2), "{k:?} vertex count");
        assert_eq!(m.polygons.len(), q.len() * 40, "{k:?} polygon count");
        let flames = q.iter().filter(|x| x.flame).count() * 40;
        assert_eq!(m.polygons.values().filter(|f| f.material_id == FLAME_ID).count(), flames);
        assert!(m.polygons.values().all(|f| [FLAME_ID, SMOKE_ID].contains(&f.material_id)));
        let n = m.vertex_normals();
        assert!(n.values().all(|v| v.is_finite() && (v.length() - 1.0).abs() < 1e-3), "{k:?} normals unit");
        assert!(m.polygons.values().flat_map(|f| f.corners.iter()).all(|c| c.uv.is_finite()));
    }
}

#[test]
fn huge_trails_stay_under_the_puff_cap() {
    let p = TrailParams { length: 50_000.0, dissipation: 50_000.0, start_radius: 0.3, sources: 4, ..TrailParams::default() };
    let n = p.puffs().len();
    assert!(n <= MAX_PUFFS && n > MAX_PUFFS / 8, "{n}");
}
