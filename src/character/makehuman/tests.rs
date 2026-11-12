use super::macros::{age_weights, Macros};
use super::mhclo::{bind_point, eval, Proxy};
use super::obj::{base, Part};
use super::target::{pack, Target};
use super::{assets, modifiers};
use bevy::math::Vec3;

#[test]
fn hm08_base_mesh_loads_with_groups_and_masks() {
    let b = base();
    assert_eq!(b.coords.len(), 19158);
    assert_eq!(b.uvs.len(), 21334);
    assert_eq!(b.faces.len(), 18486);
    assert_eq!(b.groups.len(), 172);
    let body = b.faces.iter().filter(|f| b.part(f) == Part::Body).count();
    let helper = b.faces.iter().filter(|f| b.part(f) == Part::Helper).count();
    assert_eq!((body, helper), (13378, 2650 + 684));
    let mask = b.vertex_mask(Part::Body);
    let n = mask.iter().filter(|m| **m).count();
    assert!((13000..13500).contains(&n), "{n} body verts");
    // Stands ~1.7 m tall in scene units.
    let h = b.coords.iter().map(|p| super::obj::to_scene(*p).z).fold(f32::MIN, f32::max)
        - b.coords.iter().map(|p| super::obj::to_scene(*p).z).fold(f32::MAX, f32::min);
    assert!((1.4..2.0).contains(&h), "{h}");
}

#[test]
fn pack_decodes_the_same_values_as_the_ascii_target() {
    let p = pack();
    assert_eq!(p.len(), 868);
    let t = p.get("nose/nose-base-up").unwrap();
    assert_eq!(t.index.len(), 305);
    let ascii = Target::parse_ascii("# comment\n161 0 .001 0\n197 0 .008 0\n198 0 .003 0\n");
    for k in 0..3 {
        assert_eq!(t.index[k], ascii.index[k]);
        assert!((t.delta[k] - ascii.delta[k]).length() < 1e-3, "{:?} {:?}", t.delta[k], ascii.delta[k]);
    }
}

#[test]
fn applying_a_target_moves_only_its_vertices() {
    let mut c = base().coords.clone();
    let t = pack().get("nose/nose-base-up").unwrap();
    t.apply(&mut c, 1.0);
    let moved = c.iter().zip(&base().coords).filter(|(a, b)| a != b).count();
    assert!(moved <= 305 && moved > 200, "{moved}");
}

#[test]
fn macro_weights_partition_and_find_targets() {
    for a in [0.0, 0.1, 0.1875, 0.3, 0.5, 0.8, 1.0] {
        let w = age_weights(a);
        assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-5, "{a} {w:?}");
    }
    assert_eq!(age_weights(0.5), [0.0, 0.0, 1.0, 0.0]);
    let m = Macros::default();
    let names = m.weighted_targets();
    let found = names.iter().filter(|(n, _)| pack().contains(n)).count();
    assert!(found >= 6 && found == names.len() - names.iter().filter(|(n, _)| !pack().contains(n)).count());
    let missing: Vec<_> = names.iter().filter(|(n, _)| !pack().contains(n) && !n.starts_with("breast")).collect();
    assert!(missing.is_empty(), "{missing:?}");
    // A heavy old male differs from the default.
    let mut c0 = base().coords.clone();
    m.apply(pack(), &mut c0);
    let mut c1 = base().coords.clone();
    Macros { gender: 1.0, age: 0.9, weight: 1.0, ..m }.apply(pack(), &mut c1);
    let diff: f32 = c0.iter().zip(&c1).map(|(a, b)| a.distance(*b)).sum();
    assert!(diff > 50.0, "{diff}");
}

#[test]
fn micro_sliders_load_and_resolve() {
    let s = modifiers::sliders();
    let keys: std::collections::HashSet<_> = s.iter().map(|s| &s.key).collect();
    assert_eq!(keys.len(), s.len(), "slider keys are unique");
    assert!(s.len() >= 230, "{}", s.len());
    for sl in s {
        assert!(pack().contains(&sl.pos), "{}", sl.pos);
        if let Some(n) = &sl.neg {
            assert!(pack().contains(n), "{n}");
        }
    }
}

#[test]
fn eye_proxies_fit_and_follow_morphs() {
    let p = Proxy::parse(assets::text(assets::eyes_low_mhclo()));
    assert_eq!(p.bindings.len(), 96);
    assert!(p.scale.iter().all(Option::is_some));
    let b = base();
    let fitted = p.fit(&b.coords);
    // The eyes sit in the head (top 12% of the body).
    let (lo, hi) = b.coords.iter().fold((f32::MAX, f32::MIN), |(a, c), p| (a.min(p.y), c.max(p.y)));
    assert!(fitted.iter().all(|q| q.y > lo + 0.88 * (hi - lo)), "{lo} {hi}");
    let mut c = b.coords.clone();
    for v in &mut c {
        *v *= 1.1;
    }
    let grown = p.fit(&c);
    for (a, g) in fitted.iter().zip(&grown) {
        assert!((*a * 1.1 - *g).length() < 0.02, "{a} {g}");
    }
    let bnd = bind_point(&b.coords, &[0, 1, 2, 3, 4, 5], Vec3::new(0.0, 7.0, 1.0));
    assert!((eval(&bnd, &b.coords) - Vec3::new(0.0, 7.0, 1.0)).length() < 1e-4);
}
