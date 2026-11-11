use super::blend::{Relief, height_weight};
use super::composite::{Composite, Dirty};
use super::*;
use crate::materials::shading::{Surface, linear_to_srgb, surface};
use crate::materials::{Material, Preset};
use crate::paint::texel::{HEIGHT, ROUGH};
use bevy::math::Vec4;

const UV: Vec2 = Vec2::new(0.3, 0.6);

fn gray() -> Surface {
    Surface { base: Vec3::splat(0.2), alpha: 1.0, metallic: 0.0, roughness: 0.5, transmission: 0.0, ior: 1.5, emissive: Vec3::ZERO, ao: 1.0 }
}

/// A stack whose first layer is a fill with `flat` (masked or not).
fn filled(flat: Flat) -> PaintStack {
    let mut s = PaintStack::new(64);
    s.layers[0] = PaintLayer::fill("Fill", Source::Flat(flat), None);
    s
}

#[test]
fn a_new_stack_changes_nothing_and_stacks_compare_by_revision() {
    let s = PaintStack::new(100);
    assert_eq!((s.size, s.layers.len(), s.layers[0].name.as_str()), (128, 1, "Paint Layer"));
    let mut surf = gray();
    assert_eq!(s.apply(UV, Vec3::ZERO, &mut surf, || 0.5), Relief::NONE);
    assert_eq!(surf, gray(), "an empty paint layer shows the base untouched");
    let mut edited = s.clone();
    assert_eq!(edited, s);
    edited.touch();
    assert_ne!(edited, s, "every edit bumps the revision");
    assert_ne!(PaintStack::new(100), s, "separate stacks never compare equal");
}

#[test]
fn layers_add_remove_reorder_and_get_unique_names() {
    let mut s = PaintStack::new(64);
    let a = s.add(PaintLayer::paint(s.unique_name("Paint Layer"), 64));
    assert_eq!((a, s.active, s.layers[1].name.as_str()), (1, 1, "Paint Layer 2"));
    s.active = 0;
    s.add(PaintLayer::fill("Fill", Source::default(), None));
    assert_eq!(s.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Paint Layer", "Fill", "Paint Layer 2"], "added above the active layer");
    s.shift(1, true);
    assert_eq!((s.layers[2].name.as_str(), s.active), ("Fill", 2), "the active layer follows its move");
    s.shift(0, false);
    assert_eq!(s.layers[0].name, "Paint Layer", "the bottom layer cannot move below the base");
    s.remove(2);
    assert_eq!((s.layers.len(), s.active), (2, 1));
    s.remove(9);
    assert_eq!(s.layers.len(), 2);
}

#[test]
fn blend_modes_opacity_and_visibility() {
    let white = Flat { color: [1.0; 3], ..Flat::coat([1.0; 3], 1.0, 1.0) };
    let mut s = filled(white.clone());
    let eval = |s: &PaintStack| {
        let mut surf = gray();
        s.apply(UV, Vec3::ZERO, &mut surf, || 0.5);
        surf
    };
    // Normal at full opacity replaces colour, roughness and metallic.
    let n = eval(&s);
    assert!((n.base - Vec3::ONE).length() < 1e-4 && n.roughness == 1.0 && n.metallic == 1.0);
    // Half opacity: half way in sRGB (paint-like), half way for scalars.
    s.layers[0].opacity = 0.5;
    let h = eval(&s);
    let mid = (linear_to_srgb(0.2) + 1.0) / 2.0;
    assert!((linear_to_srgb(h.base.x) - mid).abs() < 1e-4 && (h.roughness - 0.75).abs() < 1e-5);
    // Multiply by white keeps the colour; by a dark colour darkens it.
    s.layers[0].opacity = 1.0;
    s.layers[0].blend = BlendMode::Multiply;
    assert!((eval(&s).base - gray().base).length() < 1e-4);
    s.layers[0].kind = LayerKind::Fill { source: Source::Flat(Flat { color: [0.5; 3], ..white.clone() }), mask: None };
    assert!(eval(&s).base.x < gray().base.x);
    // Add and Screen only brighten.
    for mode in [BlendMode::Add, BlendMode::Screen] {
        s.layers[0].blend = mode;
        assert!(eval(&s).base.x > gray().base.x, "{mode:?}");
    }
    // Hidden layers and zero opacity do nothing.
    s.layers[0].visible = false;
    assert_eq!(eval(&s), gray());
}

#[test]
fn height_blend_fills_low_crevices_before_high_peaks() {
    assert!(height_weight(0.3, 0.05) > 0.99, "deep crevice: covered");
    assert!(height_weight(0.3, 0.9) < 0.01, "high peak: untouched");
    assert!(height_weight(0.3, 0.3) > 0.4 && height_weight(0.3, 0.3) < 0.6, "soft shoreline at the fill level");
    assert_eq!(height_weight(1.0, 0.99), 1.0, "risen all the way: covers everything");
    let mut s = filled(Flat { color: [0.0, 1.0, 0.0], ..Default::default() });
    s.layers[0].blend = BlendMode::HeightBlend;
    s.layers[0].opacity = 0.4;
    assert!(s.needs_base_height());
    let green_at = |base_h: f32| {
        let mut surf = gray();
        s.apply(UV, Vec3::ZERO, &mut surf, || base_h);
        surf.base.y
    };
    assert!(green_at(0.1) > 0.9 && (green_at(0.9) - 0.2).abs() < 1e-4, "moss in the gaps, bare stone on top");
}

#[test]
fn masks_emission_and_relief() {
    // A mask covering the left half only.
    let mut mask = Tiles::<u8>::new(64);
    mask.fill_with(|x, _| if x < 32 { 255 } else { 0 });
    let mut s = PaintStack::new(64);
    let glow = Flat { color: [1.0, 0.5, 0.0], emission: 1.0, height: 0.2, channels: [true, false, false, true, true], ..Default::default() };
    s.layers[0] = PaintLayer::fill("Embers", Source::Flat(glow), Some(mask));
    let (mut left, mut right) = (gray(), gray());
    let r_left = s.apply(Vec2::new(0.25, 0.5), Vec3::ZERO, &mut left, || 0.5);
    let r_right = s.apply(Vec2::new(0.75, 0.5), Vec3::ZERO, &mut right, || 0.5);
    assert_eq!(right, gray(), "outside the mask the base shows");
    assert_eq!(r_right, Relief::NONE);
    assert!(left.emissive.x > 3.0 && left.emissive.x > left.emissive.z * 4.0, "orange glow: {}", left.emissive);
    assert!((r_left.height - 0.2).abs() < 1e-3 && (r_left.cover - 1.0).abs() < 1e-3, "{r_left:?}");
    assert_eq!(left.roughness, 0.5, "channels that are off stay as the base has them");
    // The step in height at the mask edge tilts the normal; flat areas stay flat.
    let edge = s.tangent_normal(Vec2::new(0.5, 0.5), Vec3::ZERO, None).unwrap();
    assert!(edge.x < -0.1, "height rises towards +U, so the normal leans to -U: {edge}");
    assert!(s.tangent_normal(Vec2::new(0.8, 0.5), Vec3::ZERO, None).is_none(), "nothing painted, nothing tilted");
    let inside = s.tangent_normal(Vec2::new(0.2, 0.5), Vec3::ZERO, Some(Vec3::new(0.6, 0.0, 0.8))).unwrap();
    assert!(inside.z > 0.999, "paint with height covers the base relief: {inside}");
}

#[test]
fn material_sources_carry_colour_relief_roughness_and_metal() {
    let steel = Source::material(Preset::BrushedSteel.material());
    let s = steel.sample(UV, Vec3::ZERO);
    assert!((s.pbr[crate::paint::texel::METAL] - 0.9).abs() < 1e-5 && (s.pbr[ROUGH] - 0.25).abs() < 1e-5 && s.a == 1.0);
    assert_eq!(s.cov[HEIGHT], 0.0, "a flat preset leaves the height alone");
    // A procedural brick preset brings its relief and varies across the surface.
    let brick = Source::material(crate::materials::library::find("Red Brick").unwrap().material());
    let samples: Vec<_> = (0..64).map(|i| brick.sample(Vec2::new(i as f32 * 0.0137, i as f32 * 0.0291), Vec3::ZERO)).collect();
    assert!(samples.iter().any(|a| (a.color - samples[0].color).length() > 0.05), "bricks and mortar differ");
    assert_eq!(steel.label(), Preset::BrushedSteel.material().name);
}

#[test]
fn painted_materials_shade_through_surface_and_keep_their_base() {
    let mut m = Preset::Clay.material();
    let before = surface(&m, UV, Vec3::ZERO, Vec4::ONE);
    m.paint = Some(filled(Flat::coat([0.0, 0.0, 1.0], 0.1, 1.0)));
    let painted = surface(&m, UV, Vec3::ZERO, Vec4::ONE);
    assert!(painted.base.z > 0.9 && painted.base.x < 0.01 && painted.metallic == 1.0);
    // The base material itself is untouched: hide the layer and it is back.
    m.paint.as_mut().unwrap().layers[0].visible = false;
    assert_eq!(surface(&m, UV, Vec3::ZERO, Vec4::ONE), before);
    // Stacks survive a save and load.
    let text = ron::to_string(&m).unwrap();
    let back: Material = ron::from_str(&text).unwrap();
    assert_eq!(back.paint.as_ref().unwrap().layers.len(), 1);
    let old: Material = ron::from_str(&ron::to_string(&Preset::Clay.material()).unwrap().replace(",paint:None", "")).unwrap();
    assert!(old.paint.is_none(), "scenes saved before paint layers still load");
}

#[test]
fn the_composite_matches_surface_and_updates_only_changed_tiles() {
    let mut m = Preset::Clay.material();
    m.paint = Some(PaintStack::new(128));
    let mut c = Composite::new(128);
    assert_eq!(c.update(&m, None, 0), Dirty::All, "first build composites everything");
    assert_eq!(c.update(&m, None, 0), Dirty::None, "nothing changed, nothing to do");
    let clay = c.color.data[0];
    assert!(c.color.data.iter().all(|p| *p == clay), "an empty layer shows the base everywhere");
    // Paint one texel in tile (1, 0).
    let stack = m.paint.as_mut().unwrap();
    if let LayerKind::Paint(t) = &mut stack.layers[0].kind {
        t.set(70, 3, crate::paint::texel::Texel { rgba: [0, 0, 255, 255], pbr: [0, 0, 255, 0], cov: [0, 0, 255, 0] });
    }
    stack.touch();
    let Dirty::Tiles(tiles) = c.update(&m, None, 0) else { panic!("a stroke is an incremental update") };
    assert_eq!(tiles, vec![0, 1, 3], "tile 1 and the tiles next to it (normals read across borders)");
    assert_eq!(c.color.data[3 * 128 + 70], [0, 0, 255, 255]);
    assert_eq!(c.color.data[100 * 128 + 10], clay);
    // The composite is what the renderer shades at the same texel.
    let uv = Vec2::new(70.5 / 128.0, 1.0 - 3.5 / 128.0);
    let s = surface(&m, uv, Vec3::ZERO, Vec4::ONE);
    assert!((s.base.z - 1.0).abs() < 1e-3 && s.base.x < 1e-3);
    // The bump next to the painted texel leans the normal map.
    let n = c.normal.data[3 * 128 + 71];
    assert!(n[0] > 128, "height falls towards +U, so the normal leans to +U: {n:?}");
    // Changing a layer setting or the base material re-composites everything.
    m.paint.as_mut().unwrap().layers[0].opacity = 0.5;
    m.paint.as_mut().unwrap().touch();
    assert_eq!(c.update(&m, None, 0), Dirty::All);
    m.base_color = [1.0, 0.0, 0.0];
    m.paint.as_mut().unwrap().touch();
    assert_eq!(c.update(&m, None, 0), Dirty::All);
    assert_eq!(Composite::build(&m, None).color.data, c.color.data, "incremental and full builds agree");
}
