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

#[test]
fn undo_checkpoints_share_untouched_meshes() {
    let (mut ed, [a, b]) = editor();
    for id in [a, b] {
        ed.scene.objects[id].collapse_to_editable_poly();
        ed.scene.objects[id].evaluate();
    }
    let poly = |ed: &Editor, id| match &ed.scene.objects[id].base {
        crate::scene::BaseObject::EditablePoly(m) => m.clone(),
        _ => unreachable!(),
    };
    let snap = ed.snapshot();
    let (sa, sb) = (&snap.scene.objects[a], &snap.scene.objects[b]);
    // A checkpoint copies no mesh data: every mesh is the same allocation.
    assert!(crate::scene::Shared::ptr_eq(&sa.evaluated_mesh, &ed.scene.objects[a].evaluated_mesh));
    assert!(crate::scene::Shared::ptr_eq(&sb.level_mesh, &ed.scene.objects[b].level_mesh));
    // Editing A copies only A's base; B stays shared with the snapshot.
    ed.checkpoint("Move vertex");
    if let crate::scene::BaseObject::EditablePoly(m) = &mut ed.scene.objects[a].base {
        let v = m.vertices.keys().next().unwrap();
        m.vertices[v].position.z += 1.0;
    }
    let (pa, pb) = (poly(&ed, a), poly(&ed, b));
    let (old_a, old_b) = match (&sa.base, &sb.base) {
        (crate::scene::BaseObject::EditablePoly(x), crate::scene::BaseObject::EditablePoly(y)) => (x, y),
        _ => unreachable!(),
    };
    assert!(!crate::scene::Shared::ptr_eq(&pa, old_a) && crate::scene::Shared::ptr_eq(&pb, old_b));
    ed.undo();
    let za = poly(&ed, a).vertices.values().map(|v| v.position.z).fold(f32::MIN, f32::max);
    assert_eq!(za, old_a.vertices.values().map(|v| v.position.z).fold(f32::MIN, f32::max), "undo restored A");
}

#[test]
fn show_end_result_off_shows_the_highlighted_stack_level() {
    let (mut ed, [left, right]) = editor();
    ed.scene.select_only(Some(left));
    let o = &mut ed.scene.objects[left];
    o.add_modifier(crate::modifiers::make_modifier("TurboSmooth").unwrap());
    o.set_active_level(None);
    let (base, smooth) = (o.level_mesh.polygons.len(), o.evaluated_mesh.polygons.len());
    assert_eq!((base, smooth), (6, 24));
    assert_eq!(ed.display_mesh(left).polygons.len(), 24, "end result shown by default");
    ed.show_end_result = false;
    assert_eq!(ed.display_mesh(left).polygons.len(), 6, "base level shown");
    ed.scene.objects[left].set_active_level(Some(0));
    assert_eq!(ed.display_mesh(left).polygons.len(), 24, "TurboSmooth level shown");
    // Objects other than the current one always show their end result.
    ed.scene.objects[right].add_modifier(crate::modifiers::make_modifier("TurboSmooth").unwrap());
    ed.scene.objects[right].set_active_level(None);
    assert_eq!(ed.display_mesh(right).polygons.len(), 24);
}
