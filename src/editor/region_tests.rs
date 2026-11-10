//! `Q` region shapes: cycling and Circle / Fence / Lasso window selection.

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
