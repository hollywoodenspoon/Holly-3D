//! `Q` region shapes (cycling, Circle / Fence / Lasso window selection) and Pin Stack.

use super::*;
use crate::primitives::Primitive;
use crate::viewport::region::{Region, RegionShape};
use select::SelectOp;

/// Two boxes (left, right) seen in a maximized Top view.
fn editor() -> (Editor, [crate::scene::ObjId; 2]) {
    let mut ed = Editor::default();
    let b = Primitive::Box { length: 2.0, width: 2.0, height: 2.0, length_segs: 1, width_segs: 1, height_segs: 1 };
    let ids = [-6.0, 6.0].map(|x| {
        let id = ed.scene.add(crate::scene::BaseObject::Primitive(b.clone()), Vec3::new(x, 0.0, 0.0));
        ed.scene.objects[id].evaluate();
        id
    });
    ed.scene.select_only(None);
    (ed.viewports.maximized, ed.viewports.active) = (true, 0);
    ed.viewports.layout(crate::viewport::layout::ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
    (ed, ids)
}

#[test]
fn q_cycles_region_shapes_only_in_select_mode() {
    let mut ed = Editor { mode: TransformMode::Move, ..Default::default() };
    ed.select_mode_or_cycle_region();
    assert_eq!((ed.mode, ed.region_shape), (TransformMode::Select, RegionShape::Rectangle));
    ed.select_mode_or_cycle_region();
    assert_eq!(ed.region_shape, RegionShape::Circle);
    ed.select_mode_or_cycle_region();
    ed.select_mode_or_cycle_region();
    assert_eq!(ed.region_shape, RegionShape::Lasso);
    ed.select_mode_or_cycle_region();
    assert_eq!(ed.region_shape, RegionShape::Rectangle);
}

#[test]
fn circle_fence_and_lasso_select_what_they_enclose() {
    let (mut ed, [left, right]) = editor();
    let s = |ed: &Editor, x: f32, y: f32| ed.viewports.views[0].to_screen(Vec3::new(x, y, 0.0)).unwrap();
    let sel = |ed: &Editor| [left, right].map(|k| ed.scene.objects[k].selected);
    // Circle around the left box only.
    let c = s(&ed, -6.0, 0.0);
    let r = Region::build(RegionShape::Circle, &[c], s(&ed, -6.0, 3.0)).unwrap();
    ed.region_select(0, &r, SelectOp::Replace);
    assert_eq!(sel(&ed), [true, false]);
    // Fence (triangle) around the right box, added to the selection.
    let tri = [s(&ed, 2.0, -3.0), s(&ed, 12.0, -3.0)];
    let r = Region::build(RegionShape::Fence, &tri, s(&ed, 6.0, 6.0)).unwrap();
    ed.region_select(0, &r, SelectOp::Add);
    assert_eq!(sel(&ed), [true, true]);
    // A lasso through the gap between them touches neither.
    let gap: Vec<Vec2> = (0..12).map(|i| s(&ed, (i as f32 * 0.5).sin(), i as f32 - 6.0)).collect();
    let r = Region::build(RegionShape::Lasso, &gap, s(&ed, 0.5, 6.0)).unwrap();
    ed.region_select(0, &r, SelectOp::Replace);
    assert_eq!(sel(&ed), [false, false]);
}

#[test]
fn pin_stack_keeps_the_modify_panel_on_the_pinned_object() {
    let (mut ed, [left, right]) = editor();
    ed.scene.select_only(Some(left));
    ed.pinned = Some(left);
    ed.scene.select_only(Some(right));
    assert_eq!(ed.current(), Some(left), "pinned object stays current");
    assert_eq!(ed.scene.selected(), vec![right]);
    ed.pinned = None;
    assert_eq!(ed.current(), Some(right));
    // A deleted pinned object no longer holds the panel.
    ed.pinned = Some(left);
    ed.scene.objects.remove(left);
    assert_eq!(ed.current(), Some(right));
}

#[test]
fn hdri_survives_save_and_open() {
    use crate::render::hdri::Hdri;
    let dir = std::env::temp_dir().join("minimax_hdri_test");
    std::fs::create_dir_all(&dir).unwrap();
    // A 64x32 sky: blue above the horizon, warm ground, a bright sun at +X.
    let img = image::Rgb32FImage::from_fn(64, 32, |x, y| {
        let sun = (x as i32 - 32).abs() < 3 && (y as i32 - 12).abs() < 3;
        image::Rgb(if sun {
            [20.0, 18.0, 12.0]
        } else if y < 16 {
            [0.25, 0.45, 1.2]
        } else {
            [0.35, 0.25, 0.15]
        })
    });
    let hdr = dir.join("sky.hdr");
    img.save(&hdr).unwrap();
    let mut ed = Editor::default();
    ed.scene.environment.hdri = Some(Hdri::open(&hdr).unwrap());
    ed.scene.environment.hdri.as_mut().unwrap().strength = 1.5;
    let file = dir.join("hdri.mmax");
    ed.save_scene(&file).unwrap();
    let mut ed2 = Editor::default();
    ed2.open_scene(&file).unwrap();
    let h = ed2.scene.environment.hdri.as_ref().unwrap();
    assert!(h.image.is_some(), "the image is reloaded from its path");
    assert_eq!(h.strength, 1.5);
    assert!(h.radiance(bevy::math::Vec3::new(1.0, 0.0, 0.2)).unwrap().x > 10.0, "the sun sits just above +X");
}
