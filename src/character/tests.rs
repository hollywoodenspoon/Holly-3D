use super::makehuman::modifiers::sliders;
use super::params::*;
use super::CharacterParams;
use std::collections::HashSet;

fn mats(m: &crate::mesh::EditablePoly) -> HashSet<u16> {
    m.polygons.values().map(|p| p.material_id).collect()
}

#[test]
fn default_character_is_a_standing_valid_mesh() {
    let p = CharacterParams { eyes: Eyes::Low, hair: Hair::Bald, outfit: Outfit::None, ..Default::default() };
    let m = p.build();
    assert!(m.validate().is_ok(), "{:?}", m.validate());
    let (v, _e, f) = m.counts();
    // 13378 body quads + 86 low-poly eye faces.
    assert_eq!(f, 13378 + 86);
    assert!(v > 13000, "{v}");
    let b = m.bounds().unwrap();
    assert!(b.min.z.abs() < 1e-3, "feet on the ground: {}", b.min.z);
    assert!((1.4..2.0).contains(&b.max.z), "{}", b.max.z);
    assert_eq!(mats(&m), [ids::SKIN, ids::EYES].into());
    // Body UVs come from the hm08 map.
    assert!(m.polygons.values().any(|p| p.corners.iter().any(|c| c.uv != bevy::math::Vec2::ZERO)));
}

#[test]
fn macros_change_height_and_build() {
    let tall = CharacterParams { macros: super::makehuman::macros::Macros { height: 1.0, ..Default::default() }, ..Default::default() };
    let short = CharacterParams { macros: super::makehuman::macros::Macros { height: 0.0, ..Default::default() }, ..Default::default() };
    let (t, s) = (tall.build().bounds().unwrap().max.z, short.build().bounds().unwrap().max.z);
    assert!(t > s + 0.1, "{t} {s}");
}

#[test]
fn archetype_recipes_use_real_sliders_and_add_gear() {
    let keys: HashSet<&str> = sliders().iter().map(|s| s.key.as_str()).collect();
    for &a in Archetype::ALL {
        let mut p = CharacterParams::default();
        a.apply(&mut p);
        for k in p.micro.keys() {
            assert!(keys.contains(k.as_str()), "{a:?}: {k}");
        }
        let m = p.build();
        assert!(m.validate().is_ok(), "{a:?}");
        if p.hair != Hair::Bald {
            assert!(mats(&m).contains(&ids::HAIR), "{a:?} hair");
        }
    }
    let mut knight = CharacterParams::default();
    Archetype::Dwarf.apply(&mut knight);
    let m = knight.build();
    assert!(mats(&m).is_superset(&[ids::METAL, ids::LEATHER, ids::CLOTH].into()));
}

#[test]
fn anime_ratio_sets_toon_normals_on_the_head_only() {
    let p = CharacterParams { anime: 1.0, hair: Hair::Bald, outfit: Outfit::None, ..Default::default() };
    let m = p.build();
    let explicit = m.polygons.values().flat_map(|p| &p.corners).filter(|c| c.explicit_normal.is_some()).count();
    let total = m.polygons.values().map(|p| p.corners.len()).sum::<usize>();
    assert!(explicit > 1000 && explicit < total / 3, "{explicit} of {total}");
    for c in m.polygons.values().flat_map(|p| &p.corners) {
        if let Some(n) = c.explicit_normal {
            assert!((n.length() - 1.0).abs() < 1e-3);
        }
    }
    let plain = CharacterParams { anime: 0.0, ..p.clone() }.build();
    assert!(plain.polygons.values().flat_map(|p| &p.corners).all(|c| c.explicit_normal.is_none()));
}

#[test]
fn every_style_outfit_weapon_and_creature_builds() {
    for &h in Hair::ALL {
        for &o in Outfit::ALL {
            let p = CharacterParams { hair: h, outfit: o, weapon: Weapon::Shield, eyes: Eyes::None, ..Default::default() };
            let r = p.build().validate();
            assert!(r.is_ok(), "{h:?} {o:?} {r:?}");
        }
    }
    for &w in Weapon::ALL {
        let p = CharacterParams { weapon: w, ..Default::default() };
        assert!(p.build().validate().is_ok(), "{w:?}");
    }
    for &c in Creature::ALL.iter().skip(1) {
        let m = CharacterParams { creature: c, ..Default::default() }.build();
        assert!(m.validate().is_ok(), "{c:?}");
        let (v, _, f) = m.counts();
        assert!(v > 50 && f > 40, "{c:?} {v} {f}");
        assert!(m.bounds().unwrap().min.z > -0.05, "{c:?} stands on the ground");
    }
}

#[test]
fn high_poly_eyes_and_mirrored_micro() {
    let mut p = CharacterParams { eyes: Eyes::High, hair: Hair::Bald, outfit: Outfit::None, ..Default::default() };
    p.set_micro("ears/r-ear-scale-decr|incr", 1.0);
    assert_eq!(p.micro.get("ears/l-ear-scale-decr|incr"), Some(&1.0));
    let lo = CharacterParams { eyes: Eyes::Low, ..p.clone() }.build().counts().2;
    assert!(p.build().counts().2 > lo);
}
