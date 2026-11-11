//! Engine tests across modules: cooking a surface recipe, what each
//! weathering slider does, the derived PBR channels, cache hits and
//! background cooking.

use super::surface::{COOK_SIZE, ProcMaps, cook};
use super::*;
use crate::materials::{MapNode, Material};
use bevy::math::Vec2;

fn recipe() -> ProcSurface {
    ProcSurface::styled(Pattern::Bricks, 0)
}

/// Mean of a channel picked by `f` over every pixel.
fn mean(g: &Grid, f: impl Fn(&[f32; 4]) -> f32) -> f32 {
    g.data.iter().map(f).sum::<f32>() / g.data.len() as f32
}

#[test]
fn every_style_cooks_valid_tiling_maps() {
    for p in Pattern::ALL {
        for (i, s) in p.styles().iter().enumerate() {
            let maps = cook(&ProcSurface::styled(p, i as u8), 0.6, s.metal, 48);
            for g in [&maps.base, &maps.orm, &maps.normal] {
                assert_eq!(g.data.len(), 48 * 48);
                assert!(g.data.iter().flatten().all(|v| v.is_finite() && (-1e-4..=1.0001).contains(v)), "{}", s.label);
            }
            // Colour varies across the tile, and the left edge continues into the right.
            let lum: Vec<f32> = maps.base.data.iter().map(|c| c[0] + c[1] + c[2]).collect();
            let (lo, hi) = lum.iter().fold((9.0f32, 0.0f32), |a, v| (a.0.min(*v), a.1.max(*v)));
            assert!(hi - lo > 0.05, "{} is flat", s.label);
            let (mut seam, mut inner) = (0.0, 0.0);
            for y in 0..48i64 {
                seam += (maps.base.texel(47, y).w - maps.base.texel(48, y).w).abs();
                inner += (maps.base.texel(23, y).w - maps.base.texel(24, y).w).abs();
            }
            assert!(seam <= inner * 3.0 + 2.0, "{}: seam {seam} vs inner {inner}", s.label);
        }
    }
    // No pattern: a flat, smooth surface in the material's colour.
    let flat = cook(&ProcSurface::default(), 0.4, 0.0, 16);
    assert!(flat.normal.data.iter().all(|n| *n == [0.5, 0.5, 1.0, 1.0]) && flat.orm.data.iter().all(|o| o[1] == 0.4));
}

#[test]
fn normals_ao_and_roughness_are_derived_from_the_height_map() {
    let on = cook(&recipe(), 0.6, 0.0, 96);
    // Brick bevels tilt the normals; mortar joints are occluded and rougher.
    let tilted = on.normal.data.iter().filter(|n| n[2] < 0.97).count();
    assert!(tilted > 300, "{tilted} tilted normals");
    let (mut joint_ao, mut face_ao, mut joint_rough, mut face_rough) = (vec![], vec![], vec![], vec![]);
    for (b, o) in on.base.data.iter().zip(&on.orm.data) {
        if b[3] < 0.02 {
            joint_ao.push(o[0]);
            joint_rough.push(o[1]);
        } else if b[3] > 0.7 {
            face_ao.push(o[0]);
            face_rough.push(o[1]);
        }
    }
    let avg = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    assert!(avg(&joint_ao) < avg(&face_ao) - 0.1, "joints {} vs faces {}", avg(&joint_ao), avg(&face_ao));
    assert!(avg(&joint_rough) > avg(&face_rough) + 0.05);
    // More relief, steeper normals and deeper AO.
    let strong = cook(&ProcSurface { relief: 2.0, ..recipe() }, 0.6, 0.0, 96);
    assert!(mean(&strong.normal, |n| n[2]) < mean(&on.normal, |n| n[2]));
    assert!(mean(&strong.orm, |o| o[0]) < mean(&on.orm, |o| o[0]));
    // Auto-derive off: flat normals, no AO, roughness only from the pattern.
    let off = cook(&ProcSurface { auto_pbr: false, ..recipe() }, 0.6, 0.0, 96);
    assert!(off.normal.data.iter().all(|n| *n == [0.5, 0.5, 1.0, 1.0]) && off.orm.data.iter().all(|o| o[0] == 1.0));
}

#[test]
fn each_weathering_slider_changes_the_surface_its_own_way() {
    let clean = cook(&recipe(), 0.6, 0.0, 96);
    let green = |g: &Grid| mean(g, |c| c[1] - (c[0] + c[2]) * 0.5);
    let lum = |g: &Grid| mean(g, |c| c[0] + c[1] + c[2]);
    // Moss turns the surface green and rough, starting in the joints.
    let moss = cook(&ProcSurface { moss: 0.6, ..recipe() }, 0.6, 0.0, 96);
    assert!(green(&moss.base) > green(&clean.base) + 0.02);
    let (mut joint, mut face) = (0.0, 0.0);
    for (m, c) in moss.base.data.iter().zip(&clean.base.data) {
        let d = (m[1] - c[1]).abs() + (m[0] - c[0]).abs();
        if c[3] < 0.02 { joint += d } else if c[3] > 0.7 { face += d }
    }
    let joints = clean.base.data.iter().filter(|c| c[3] < 0.02).count() as f32;
    let faces = clean.base.data.iter().filter(|c| c[3] > 0.7).count() as f32;
    assert!(joint / joints > face / faces, "moss fills gaps first");
    // Cavity dirt and scorch darken; wetness darkens and smooths, with level puddles.
    assert!(lum(&cook(&ProcSurface { cavity_dirt: 0.7, ..recipe() }, 0.6, 0.0, 96).base) < lum(&clean.base));
    assert!(lum(&cook(&ProcSurface { scorch: 0.7, ..recipe() }, 0.6, 0.0, 96).base) < lum(&clean.base) - 0.1);
    let wet = cook(&ProcSurface { wetness: 0.8, ..recipe() }, 0.6, 0.0, 96);
    assert!(mean(&wet.orm, |o| o[1]) < mean(&clean.orm, |o| o[1]) - 0.1, "wet is glossier");
    assert!(wet.orm.data.iter().filter(|o| o[1] < 0.1).count() > 100, "puddles");
    // Edge wear brightens raised edges; on metal it bares bright metal, and scorch becomes rust.
    let worn = cook(&ProcSurface { edge_wear: 0.8, ..recipe() }, 0.6, 0.0, 96);
    assert!(lum(&worn.base) > lum(&clean.base));
    let plate = ProcSurface::styled(Pattern::Metal, 1);
    let rusty = cook(&ProcSurface { scorch: 0.8, ..plate.clone() }, 0.35, 1.0, 96);
    let bare = cook(&plate, 0.35, 1.0, 96);
    assert!(mean(&rusty.orm, |o| o[2]) < mean(&bare.orm, |o| o[2]) - 0.2, "rust is not metal");
    assert!(mean(&rusty.base, |c| c[0] - c[2]) > mean(&bare.base, |c| c[0] - c[2]) + 0.03, "rust is orange");
    // Every slider at once still gives valid maps.
    let all = cook(&ProcSurface { edge_wear: 1.0, cavity_dirt: 1.0, moss: 1.0, scorch: 1.0, wetness: 1.0, ..recipe() }, 0.6, 0.0, 48);
    assert!(all.base.data.iter().chain(&all.orm.data).flatten().all(|v| v.is_finite() && (-1e-4..=1.0001).contains(v)));
}

#[test]
fn recipes_are_content_addressed_and_cooked_once() {
    let p = ProcSurface { seed: 4242, ..recipe() };
    assert_eq!(p.key(0.6, 0.0, COOK_SIZE), p.clone().key(0.6, 0.0, COOK_SIZE));
    // Any spinner, the seed, the style or the material's own values change the key.
    let base = p.key(0.6, 0.0, COOK_SIZE);
    for q in [
        ProcSurface { tiling: 2.0, ..p.clone() },
        ProcSurface { seed: 4243, ..p.clone() },
        ProcSurface { style: 1, ..p.clone() },
        ProcSurface { gap: 0.2, ..p.clone() },
        ProcSurface { moss: 0.1, ..p.clone() },
        ProcSurface { color_a: [0.1, 0.2, 0.3], ..p.clone() },
        ProcSurface { auto_pbr: false, ..p.clone() },
    ] {
        assert_ne!(q.key(0.6, 0.0, COOK_SIZE), base);
    }
    assert_ne!(p.key(0.5, 0.0, COOK_SIZE), base);
    assert_ne!(p.key(0.6, 0.0, 128), base);
    // The first request cooks; the second is the same Arc out of the cache.
    let cooks = cache::stats().1;
    let a = surface::maps(&p, 0.6, 0.0).unwrap();
    let b = surface::maps(&p, 0.6, 0.0).unwrap();
    assert!(std::sync::Arc::ptr_eq(&a, &b));
    assert!(cache::stats().1 > cooks);
    assert!(cache::peek::<ProcMaps>(base).is_some());
    assert!(surface::maps(&ProcSurface::default(), 0.6, 0.0).is_none(), "no pattern, nothing to cook");
    // Sampling reads the cooked grids.
    let s = a.at(Vec2::new(0.3, 0.001));
    assert!(s.height < 0.5 && s.ao < 0.97, "a mortar joint: {s:?}");
}

#[test]
fn the_worker_cooks_in_the_background_until_the_maps_are_ready() {
    let m = Material { proc: ProcSurface { seed: 9191, ..recipe() }, ..Default::default() };
    assert!(worker::ready(&Material::default()), "nothing to cook");
    let mut ready = worker::ready(&m);
    for _ in 0..600 {
        if ready {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        ready = worker::ready(&m);
    }
    assert!(ready, "the worker finished");
    assert!(cache::peek::<ProcMaps>(m.proc.key(m.roughness, m.metallic, COOK_SIZE)).is_some());
}

#[test]
fn filter_nodes_share_baked_inputs_through_the_cache() {
    let bricks = MapNode::tile(TileKind::Bricks, [0.0; 3], [1.0; 3]);
    assert_eq!(node_key::content_key(&bricks), node_key::content_key(&bricks.clone()));
    let mut other = bricks.clone();
    if let MapNode::Tile { seed, .. } = &mut other {
        *seed = 77;
    }
    assert_ne!(node_key::content_key(&bricks), node_key::content_key(&other));
    // Two filters over the same input reuse one baked grid of it.
    let normal = MapNode::NormalFromHeight { input: Box::new(other.clone()), intensity: 1.0 };
    let blur = MapNode::SlopeBlur { input: Box::new(other.clone()), slope: Box::new(other.clone()), intensity: 6.0, samples: 4 };
    assert_ne!(node_key::content_key(&normal), node_key::content_key(&blur));
    let first = node_key::grid(&other);
    let _ = (node_key::grid(&normal), node_key::grid(&blur));
    assert!(std::sync::Arc::ptr_eq(&first, &node_key::grid(&other)));
    // Editing a filter's input changes the filter's key too.
    let edited = MapNode::NormalFromHeight { input: Box::new(bricks), intensity: 1.0 };
    assert_ne!(node_key::content_key(&normal), node_key::content_key(&edited));
    // The filter samples as a normal map: blue-ish, tilted on bevels.
    let n = normal.sample(Vec2::new(0.125, 0.0625), bevy::math::Vec3::ZERO);
    assert!(n.z > 0.9 && (n.x - 0.5).abs() < 0.05, "a brick face is flat: {n}");
}
