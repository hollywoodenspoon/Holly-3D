use super::array::ArraySettings;
use super::mirror::{MirrorAxis, MirrorSettings};
use super::*;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::BaseObject;
use bevy::math::Quat;

fn editor_with_box(at: Vec3) -> (Editor, ObjId) {
    let mut ed = Editor::default();
    let id = ed.scene.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), at);
    ed.scene.objects[id].evaluate();
    ed.scene.select_only(Some(id));
    (ed, id)
}

#[test]
fn array_counts_and_totals() {
    let mut s = ArraySettings { dims: 1, count: [5, 3, 2], ..Default::default() };
    assert_eq!(s.total(), 5);
    assert_eq!(s.transforms(Xform::default(), Vec3::ZERO).len(), 4);
    s.dims = 2;
    assert_eq!(s.transforms(Xform::default(), Vec3::ZERO).len(), 14);
    s.dims = 3;
    assert_eq!(s.total(), 30);
    let (mv, _, _) = s.totals();
    assert!((mv.x - 2.0).abs() < 1e-5, "0.5 x 4 copies");
    s.set_totals(Vec3::new(8.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 360.0), Vec3::splat(200.0));
    assert!((s.move_inc.x - 2.0).abs() < 1e-5 && (s.rotate_inc.z - 90.0).abs() < 1e-4 && (s.scale_inc.x - 125.0).abs() < 1e-4);
}

#[test]
fn radial_array_about_the_world_origin() {
    let s = ArraySettings { move_inc: Vec3::ZERO, rotate_inc: Vec3::new(0.0, 0.0, 45.0), count: [8, 1, 1], ..Default::default() };
    let src = Xform { translation: Vec3::new(2.0, 0.0, 0.0), ..Default::default() };
    let xs = s.transforms(src, Vec3::ZERO);
    assert_eq!(xs.len(), 7);
    for x in &xs {
        assert!((x.translation.length() - 2.0).abs() < 1e-4, "on the circle");
    }
    assert!((xs[1].translation - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-4, "copy 2 at 90 degrees");
    let fixed = ArraySettings { re_orient: false, ..s }.transforms(src, Vec3::ZERO);
    assert_eq!(fixed[1].rotation, Quat::IDENTITY, "no re-orient keeps the orientation");
}

#[test]
fn array_dialog_preview_ok_and_cancel() {
    let (mut ed, _) = editor_with_box(Vec3::ZERO);
    ed.open_array();
    ed.update_array_preview();
    assert_eq!(ed.scene.objects.len(), 5, "live preview");
    ed.array_dialog.as_mut().unwrap().settings.count[0] = 3;
    ed.update_array_preview();
    assert_eq!(ed.scene.objects.len(), 3, "preview rebuilt");
    let mut names: Vec<String> = ed.scene.objects.values().map(|o| o.name.clone()).collect();
    names.sort();
    assert_eq!(names, ["Box001", "Box002", "Box003"], "rebuilds reuse names");
    ed.cancel_array();
    assert_eq!(ed.scene.objects.len(), 1);
    assert!(!ed.history.can_undo(), "cancel leaves no undo step");
    ed.open_array();
    ed.array_dialog.as_mut().unwrap().settings.kind = CloneKind::Instance;
    ed.confirm_array();
    assert_eq!(ed.scene.objects.len(), 3);
    assert!(ed.scene.objects.values().all(|o| o.instance_group.is_some()));
    ed.undo();
    assert_eq!(ed.scene.objects.len(), 1);
}

#[test]
fn mirror_flips_and_offsets() {
    let m = MirrorSettings { axis: MirrorAxis::X, offset: 1.0, clone: None };
    let x = m.apply(Xform { translation: Vec3::new(2.0, 1.0, 0.0), ..Default::default() }, Vec3::ZERO);
    assert!((x.translation - Vec3::new(-1.0, 1.0, 0.0)).length() < 1e-5);
    assert!(x.matrix().determinant() < 0.0, "mirrored = negative scale");
    let yz = MirrorSettings { axis: MirrorAxis::YZ, ..Default::default() }.matrix(Vec3::ZERO);
    assert!((yz.determinant() - 1.0).abs() < 1e-5, "two flips = a rotation");
}

#[test]
fn mirror_dialog_no_clone_and_copy() {
    let (mut ed, id) = editor_with_box(Vec3::new(1.0, 0.0, 0.0));
    ed.pivot_mode = PivotMode::TransformCenter;
    ed.open_mirror();
    ed.update_mirror_preview();
    assert!((ed.scene.objects[id].transform.translation.x + 1.0).abs() < 1e-5, "No Clone mirrors in place");
    ed.mirror_dialog.as_mut().unwrap().settings.clone = Some(CloneKind::Copy);
    ed.update_mirror_preview();
    assert!((ed.scene.objects[id].transform.translation.x - 1.0).abs() < 1e-5, "source restored");
    ed.confirm_mirror();
    assert_eq!(ed.scene.objects.len(), 2);
    let copy = ed.scene.selected()[0];
    assert!(copy != id && ed.scene.objects[copy].world_matrix().determinant() < 0.0);
    ed.open_mirror();
    ed.cancel_mirror();
    assert_eq!(ed.scene.objects.len(), 2);
}
