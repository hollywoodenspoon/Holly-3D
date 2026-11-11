use super::*;
use bevy::math::{Vec2, Vec3};
use std::collections::HashSet;

#[test]
fn library_is_big_named_uniquely_and_every_category_has_entries() {
    let all = all();
    assert!(all.len() >= 270, "{} materials", all.len());
    let mut names = HashSet::new();
    for (_, i) in &all {
        assert!(names.insert(i.name().to_lowercase()), "duplicate {}", i.name());
    }
    for c in Category::ALL {
        assert!(!c.items().is_empty(), "{}", c.label());
    }
    assert_eq!(find("red brick").map(|i| i.name()), Some("Red Brick"));
    assert!(find("Brushed Steel / Gunmetal").is_some(), "classic presets are in the library");
    assert!(find("no such").is_none());
}

#[test]
fn every_material_is_valid_and_samples_in_range() {
    for (c, i) in all() {
        let m = i.material();
        assert_eq!(m.name.as_str(), match i {
            Item::Classic(_) => m.name.as_str(),
            Item::Lib(s) => s.name,
            Item::Proc(s) => s.name,
        });
        for v in [m.metallic, m.roughness, m.opacity, m.transmission] {
            assert!((0.0..=1.0).contains(&v), "{}: {v}", i.name());
        }
        assert!(m.base_color.iter().all(|v| (0.0..=1.0).contains(v)), "{}", i.name());
        for ch in MapChannel::ALL {
            if let Some((n, _)) = m.active_map(ch) {
                for k in 0..16 {
                    let uv = Vec2::new(k as f32 / 16.0 + 0.01, (k * 5 % 16) as f32 / 16.0 + 0.02);
                    let s = n.sample(uv, Vec3::new(uv.x * 2.0, uv.y, 0.3));
                    assert!(s.is_finite() && s.min_element() >= 0.0 && s.max_element() <= 1.0 + 1e-4, "{} {}", c.label(), i.name());
                }
            }
        }
    }
}

#[test]
fn specs_turn_into_the_right_shading() {
    let ruby = find("Ruby").unwrap().material();
    assert_eq!(ruby.model, ShadingModel::Crystal);
    assert!(ruby.ior > 1.7 && ruby.transmission > 0.5);
    let neon = find("Neon Pink").unwrap().material();
    assert!(neon.emissive_strength > 1.0);
    let toon = find("Toon Red").unwrap().material();
    assert!(toon.model == ShadingModel::Toon && toon.outline.enabled);
    let brick = find("Red Brick").unwrap().material();
    assert!(matches!(brick.active_map(MapChannel::BaseColor), Some((MapNode::Bricks { .. }, _))));
    assert!(brick.active_map(MapChannel::Bump).is_some(), "patterns add bump");
    let rust = find("Heavy Rust").unwrap().material();
    assert!(rust.active_map(MapChannel::Metallic).is_some() && rust.active_map(MapChannel::Roughness).is_some());
    // Weathered surfaces use the Procedural Pattern & Weathering rollout.
    let moss = find("Mossy Brick Wall").unwrap().material();
    assert!(moss.proc.active() && moss.proc.moss > 0.5 && moss.proc.auto_pbr && moss.roughness == 0.85);
    assert!(crate::materials::bake::bake_normal(&moss, 16).is_some(), "it derives its own normal map");
    let plate = find("Rusty Diamond Plate").unwrap().material();
    assert!(plate.metallic == 1.0 && plate.proc.scorch > 0.5);
    assert!(Category::Weathered.items().len() >= 30);
    let line = find("Road: White Centre Line").unwrap().material();
    let Some((MapNode::Stripes { color_a, color_b, .. }, _)) = line.active_map(MapChannel::BaseColor) else { panic!() };
    assert!(color_a[0] > 0.8 && color_b[0] < 0.3, "white line on dark asphalt");
}
