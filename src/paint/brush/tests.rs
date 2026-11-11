use super::*;
use crate::mesh::EditablePoly;
use crate::paint::layers::{LayerKind, PaintLayer, Source};
use crate::paint::texel::{EMISSION, HEIGHT, METAL, ROUGH, Texel, Tiles};
use crate::paint::{Brush, BrushTool, paint_tris};
use crate::primitives::{Primitive, PrimitiveKind};
use bevy::math::{Mat4, Vec2, Vec3};

const SIZE: u32 = 128;

fn plane() -> EditablePoly {
    // 2x2 plane (4x4 quads), UVs 0..1, Z up.
    Primitive::default_of(PrimitiveKind::Plane).to_editable_poly()
}

fn texels(l: &PaintLayer) -> &Tiles<Texel> {
    match &l.kind {
        LayerKind::Paint(t) => t,
        _ => panic!("paint layer"),
    }
}

fn at(center: Vec3) -> Stamp {
    Stamp { center, normal: Vec3::Z, tangent: Vec3::X, view_dir: Vec3::NEG_Z }
}

fn round(size: f32) -> Brush {
    Brush { size, opacity: 1.0, falloff: 0.0, color: [0.0, 0.0, 1.0], tool: BrushTool::Paint, alpha: Alpha::HardRound }
}

/// A fresh layer and a stroke on the plane.
fn setup() -> (PaintLayer, LayerStroke, Vec3, f32) {
    let m = plane();
    let b = m.bounds().unwrap();
    let mut layer = PaintLayer::paint("L", SIZE);
    let stroke = LayerStroke::new(paint_tris(&m, Mat4::IDENTITY, false), &mut layer, SIZE, 7);
    (layer, stroke, b.center(), b.size().x)
}

#[test]
fn a_stamp_projects_through_the_uvs_and_respects_the_view() {
    let (mut layer, mut stroke, c, w) = setup();
    let blue = Source::Flat(crate::paint::Flat { color: [0.0, 0.0, 1.0], ..Default::default() });
    let hit = stroke.stamp(&mut layer, &at(c), &round(w * 0.1), &blue);
    assert!(hit > 0);
    // The plane centre maps to UV (0.5, 0.5).
    let t = texels(&layer);
    assert_eq!(t.get(64, 64).rgba, [0, 0, 255, 255]);
    assert_eq!(t.get(4, 4), Texel::default(), "far texels untouched");
    assert_eq!(t.get(64, 64).cov, [0; 4], "a colour brush leaves roughness, metal, height and glow alone");
    // Seen from behind, nothing is painted.
    let (mut back, mut s2, ..) = setup();
    assert_eq!(s2.stamp(&mut back, &Stamp { view_dir: Vec3::Z, ..at(c) }, &round(w * 0.1), &blue), 0);
    assert!(texels(&back).is_empty());
    // Tangent projection: a brush hovering far off the surface misses it.
    assert_eq!(s2.stamp(&mut back, &at(c + Vec3::Z * w), &round(w * 0.1), &blue), 0);
}

#[test]
fn opacity_is_per_stroke_and_erasing_reveals_what_is_below() {
    let (mut layer, mut stroke, c, w) = setup();
    let src = Source::Flat(crate::paint::Flat::coat([1.0, 0.0, 0.0], 0.2, 1.0));
    let half = Brush { opacity: 0.5, ..round(w * 0.1) };
    stroke.stamp(&mut layer, &at(c), &half, &src);
    stroke.stamp(&mut layer, &at(c), &half, &src);
    let t = texels(&layer).get(64, 64);
    assert!((t.rgba[3] as i32 - 128).abs() <= 1, "two stamps of one stroke do not build past 50%: {t:?}");
    assert_eq!((t.rgba[0], t.pbr[METAL], t.pbr[ROUGH]), (255, 255, 51), "coverage is stored apart from the values");
    assert!((t.cov[METAL] as i32 - 128).abs() <= 1 && t.cov[HEIGHT] == 0);
    // A second stroke builds on the first.
    let m = plane();
    let mut again = LayerStroke::new(paint_tris(&m, Mat4::IDENTITY, false), &mut layer, SIZE, 8);
    again.stamp(&mut layer, &at(c), &half, &src);
    assert!((texels(&layer).get(64, 64).rgba[3] as i32 - 191).abs() <= 1);
    // Erasing clears the layer there: the texel is free again.
    let mut eraser = LayerStroke::new(paint_tris(&m, Mat4::IDENTITY, false), &mut layer, SIZE, 9);
    eraser.stamp(&mut layer, &at(c), &Brush { tool: BrushTool::Erase, ..round(w * 0.1) }, &src);
    assert_eq!(texels(&layer).get(64, 64), Texel::default());
    assert!(layer.sample(Vec2::splat(0.5), Vec3::ZERO).is_none(), "the base material shows through again");
}

#[test]
fn material_brushes_paint_every_pbr_channel_in_one_stroke() {
    let (mut layer, mut stroke, c, w) = setup();
    let steel = Source::material(crate::materials::Preset::BrushedSteel.material());
    stroke.stamp(&mut layer, &at(c), &round(w * 0.1), &steel);
    let t = texels(&layer).get(64, 64);
    assert_eq!(t.rgba[3], 255);
    assert!(t.pbr[METAL] > 220 && t.cov[METAL] == 255 && t.cov[ROUGH] == 255 && t.cov[EMISSION] == 255, "{t:?}");
}

#[test]
fn every_alpha_stays_in_range_and_stamps_bring_their_own_look() {
    for a in Alpha::ALL {
        let mut total = 0.0;
        for i in 0..40 {
            for j in 0..40 {
                let q = Vec2::new(i as f32 / 19.5 - 1.0, j as f32 / 19.5 - 1.0);
                let d = a.eval(q, 0.5, 3);
                assert!((0.0..=1.0).contains(&d.cov), "{a:?} at {q}: {}", d.cov);
                assert!(q.length() < 1.0 || d.cov == 0.0, "{a:?} paints outside the brush");
                assert_eq!(d.look.is_some() && d.cov > 0.0, a.is_stamp() && d.cov > 0.0, "{a:?}");
                if let Some(f) = &d.look {
                    assert!((0.0..=1.0).contains(&f.height) && (0.0..=1.0).contains(&f.rough) && (0.0..=1.0).contains(&f.emission));
                }
                total += d.cov;
            }
        }
        assert!(total > 2.0, "{a:?} draws something");
        assert!(a.spacing() > 0.0 && !a.label().is_empty());
    }
    assert_eq!(Alpha::ALL.iter().filter(|a| a.is_stamp()).count(), 7);
    // The centre of a bullet hole is a dark pit with a raised lip around it.
    let pit = Alpha::BulletHole.eval(Vec2::ZERO, 0.5, 1).look.unwrap();
    let lip = Alpha::BulletHole.eval(Vec2::new(0.31, 0.0), 0.5, 1).look.unwrap();
    assert!(pit.height < 0.2 && pit.color[0] < 0.1 && lip.height > 0.5, "{pit:?} {lip:?}");
    // Ember cracks glow; a rivet only raises the surface.
    let glows = (0..400).filter_map(|i| Alpha::EmberCracks.eval(Vec2::new((i % 20) as f32 / 14.0 - 0.7, (i / 20) as f32 / 14.0 - 0.7), 0.5, 5).look).filter(|f| f.emission > 0.3).count();
    assert!(glows > 5, "{glows} glowing texels");
    let rivet = Alpha::Rivet.eval(Vec2::ZERO, 0.5, 1).look.unwrap();
    assert!(rivet.height > 0.85 && rivet.channels == [false, false, false, true, false]);
}

#[test]
fn damage_stamps_write_depth_roughness_and_glow_and_can_overlap() {
    let (mut layer, mut stroke, c, w) = setup();
    let unused = Source::default();
    let brush = Brush { alpha: Alpha::BulletHole, ..round(w * 0.2) };
    stroke.stamp(&mut layer, &at(c), &brush, &unused);
    let pit = texels(&layer).get(64, 64);
    assert!(pit.pbr[HEIGHT] < 60 && pit.cov[HEIGHT] == 255 && pit.rgba[0] < 30 && pit.pbr[ROUGH] > 230, "{pit:?}");
    assert!(layer.sample(Vec2::splat(0.5), Vec3::ZERO).unwrap().pbr[HEIGHT] < 0.25);
    // A second stamp of the same stroke lands on top of the first.
    let before = texels(&layer).allocated();
    stroke.stamp(&mut layer, &at(c + Vec3::X * w * 0.15), &brush, &unused);
    assert!(texels(&layer).allocated() >= before);
    let second = texels(&layer).get(64 + (0.15 * SIZE as f32) as i64, 64);
    assert!(second.pbr[HEIGHT] < 60, "the second pit is cut into the first stamp's soot: {second:?}");
    // Ember cracks write emission.
    let (mut embers, mut s2, ..) = setup();
    s2.stamp(&mut embers, &at(c), &Brush { alpha: Alpha::EmberCracks, ..round(w * 0.3) }, &unused);
    let mut hot = 0;
    for y in 40..90 {
        for x in 40..90 {
            let t = texels(&embers).get(x, y);
            hot += (t.cov[EMISSION] > 0 && t.pbr[EMISSION] > 80) as usize;
        }
    }
    assert!(hot > 10, "{hot} glowing texels");
}

#[test]
fn strokes_on_fill_layers_edit_the_mask() {
    let m = plane();
    let c = m.bounds().unwrap().center();
    let w = m.bounds().unwrap().size().x;
    let mut layer = PaintLayer::fill("Fill", Source::default(), None);
    // Erasing a fill layer without a mask gives it a full mask first.
    let mut stroke = LayerStroke::new(paint_tris(&m, Mat4::IDENTITY, false), &mut layer, SIZE, 1);
    stroke.stamp(&mut layer, &at(c), &Brush { tool: BrushTool::Erase, ..round(w * 0.1) }, &Source::default());
    let LayerKind::Fill { mask: Some(mask), .. } = &layer.kind else { panic!("mask") };
    assert_eq!((mask.get(64, 64), mask.get(4, 4)), (0, 255));
    // Painting brings it back.
    let mut stroke = LayerStroke::new(paint_tris(&m, Mat4::IDENTITY, false), &mut layer, SIZE, 2);
    stroke.stamp(&mut layer, &at(c), &round(w * 0.1), &Source::default());
    let LayerKind::Fill { mask: Some(mask), .. } = &layer.kind else { panic!("mask") };
    assert_eq!(mask.get(64, 64), 255);
}

#[test]
fn blur_softens_a_hard_edge_and_flood_fills_ignore_facing() {
    let (mut layer, mut stroke, c, w) = setup();
    let src = Source::Flat(crate::paint::Flat { color: [1.0; 3], ..Default::default() });
    stroke.stamp(&mut layer, &at(c), &round(w * 0.1), &src);
    // Just outside the disc the layer is empty; blurring bleeds coverage in.
    let edge = 64 + (0.1 * SIZE as f32) as i64 + 1;
    assert_eq!(texels(&layer).get(edge, 64).rgba[3], 0);
    let m = plane();
    let mut blur = LayerStroke::new(paint_tris(&m, Mat4::IDENTITY, false), &mut layer, SIZE, 3);
    blur.stamp(&mut layer, &at(c), &Brush { tool: BrushTool::Blur, ..round(w * 0.3) }, &src);
    let a = texels(&layer).get(edge, 64).rgba[3];
    assert!(a > 0 && a < 255, "soft edge: {a}");
    // A flood stamp (no normal) fills every texel the mesh covers.
    let (mut all, mut flood, ..) = setup();
    let n = flood.stamp(&mut all, &Stamp { center: Vec3::ZERO, normal: Vec3::ZERO, tangent: Vec3::X, view_dir: Vec3::ZERO }, &round(1e9), &src);
    assert!(n >= (SIZE * SIZE) as usize, "{n}");
    assert_eq!(texels(&all).allocated(), 4);
}
