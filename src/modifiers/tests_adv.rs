//! Phase 5e: Boolean, ProOptimizer and Edit Normals.

use super::edit_normals::{corners, keys_at_vertex, keys_of_face};
use super::*;
use crate::mesh::EditablePoly;
use crate::primitives::{Primitive, PrimitiveKind};
use bevy::math::{Mat4, Quat, Vec3};

/// Unit cube spanning [0,1]^3.
fn unit_cube() -> EditablePoly {
    let mut m = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    let b = m.bounds().unwrap();
    let s = b.max - b.min;
    m.transform(Mat4::from_scale(Vec3::ONE / s) * Mat4::from_translation(-b.min));
    m.polygons.values_mut().for_each(|p| p.material_id = 1);
    m
}

/// Box over x in [0.5, 1.5], y/z in [-0.1, 1.1] (no faces coplanar with the cube).
fn cutter() -> EditablePoly {
    let mut m = unit_cube();
    m.transform(Mat4::from_translation(Vec3::new(0.5, -0.1, -0.1)) * Mat4::from_scale(Vec3::new(1.0, 1.2, 1.2)));
    for p in m.polygons.values_mut() {
        p.material_id = 3;
    }
    m
}

fn solid(m: &EditablePoly, what: &str) {
    assert!(m.validate().is_ok(), "{what}: {:?}", m.validate());
    assert!(m.is_closed(), "{what}: closed");
    assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())), "{what}: UVs");
}

fn boolean(op: BoolOp) -> EditablePoly {
    let mut b = BooleanMod { op, ..Default::default() };
    b.add_operand("Box002".into(), cutter());
    let mut m = unit_cube();
    b.apply(&mut m);
    m
}

#[test]
fn boolean_subtract_and_intersect_give_clean_boxes() {
    for (op, vol) in [(BoolOp::SubtractAB, 0.5), (BoolOp::Intersect, 0.5)] {
        let m = boolean(op);
        solid(&m, op.label());
        assert_eq!(m.counts(), (8, 12, 6), "{}: coplanar triangles merged back to quads", op.label());
        assert!((m.signed_volume() - vol).abs() < 1e-4, "{}: volume {}", op.label(), m.signed_volume());
    }
    // the cut face of A-B comes from the cutter and keeps its material ID
    let m = boolean(BoolOp::SubtractAB);
    assert_eq!(m.polygons.values().filter(|p| p.material_id == 3).count(), 1);
}

#[test]
fn boolean_union_and_reverse_subtract_volumes() {
    let u = boolean(BoolOp::Union);
    solid(&u, "union");
    assert!((u.signed_volume() - 1.94).abs() < 1e-3, "union volume {}", u.signed_volume());
    let r = boolean(BoolOp::SubtractBA);
    solid(&r, "B-A");
    assert!((r.signed_volume() - 0.94).abs() < 1e-3, "B-A volume {}", r.signed_volume());
    // without merging the result stays triangulated
    let mut b = BooleanMod { merge_coplanar: false, op: BoolOp::SubtractAB, ..Default::default() };
    b.add_operand("B".into(), cutter());
    let mut m = unit_cube();
    b.apply(&mut m);
    solid(&m, "unmerged");
    assert!(m.polygons.values().all(|p| p.corners.len() == 3));
}

#[test]
fn boolean_skips_open_operands_and_disabled_ones() {
    let mut open = unit_cube();
    let top = open.polygons.keys().next().unwrap();
    open.polygons.remove(top);
    open.finalize();
    let mut b = BooleanMod { op: BoolOp::SubtractAB, ..Default::default() };
    b.add_operand("Open".into(), open);
    let mut m = unit_cube();
    b.apply(&mut m);
    assert_eq!(m.counts(), (8, 12, 6));
    let mut b = BooleanMod { op: BoolOp::SubtractAB, ..Default::default() };
    b.add_operand("B".into(), cutter());
    b.operands[0].enabled = false;
    let mut m = unit_cube();
    b.apply(&mut m);
    assert!((m.signed_volume() - 1.0).abs() < 1e-5);
}

fn sphere() -> EditablePoly {
    Primitive::default_of(PrimitiveKind::Sphere).to_editable_poly()
}

#[test]
fn pro_optimizer_reduces_after_calculate() {
    let src = sphere();
    let mut p = ProOptimizerMod { percent: 50.0, ..Default::default() };
    let mut m = src.clone();
    p.apply(&mut m);
    assert_eq!(m.counts(), src.counts(), "nothing happens before Calculate");
    p.calculate(&src);
    assert_eq!(p.target_verts(), (src.vertices.len() as f32 * 0.5).round() as usize);
    p.apply(&mut m);
    let (n0, n) = (src.vertices.len() as f32, m.vertices.len() as f32);
    assert!(n < n0 * 0.6 && n > n0 * 0.3, "{n} of {n0}");
    assert!(m.validate().is_ok(), "{:?}", m.validate());
    assert!(m.is_closed());
    assert!(m.signed_volume() > 0.0);
    p.set_target_verts(src.vertices.len() / 4);
    assert!((p.percent - 25.0).abs() < 1.0);
}

#[test]
fn pro_optimizer_keeps_material_boundaries() {
    let mut src = sphere();
    let ids: Vec<_> = src.polygons.keys().collect();
    for id in ids {
        if src.poly_centroid(id).z > 0.0 {
            src.polygons[id].material_id = 2;
        }
    }
    let b = src.bounds().unwrap();
    let mid = (b.min.z + b.max.z) * 0.5;
    let mut p = ProOptimizerMod { percent: 30.0, ..Default::default() };
    p.calculate(&src);
    let mut m = src.clone();
    p.apply(&mut m);
    assert!(m.validate().is_ok());
    assert!(m.vertices.len() < src.vertices.len() / 2);
    for (id, poly) in &m.polygons {
        let zs: Vec<f32> = poly.vertex_ids().map(|v| m.pos(v).z - mid).collect();
        if poly.material_id == 2 {
            assert!(zs.iter().all(|&z| z > -1e-4), "material 2 crossed the seam: {id:?} {zs:?}");
        } else {
            assert!(zs.iter().all(|&z| z < 1e-4), "material 1 crossed the seam: {id:?} {zs:?}");
        }
    }
}

#[test]
fn edit_normals_unify_break_reset_rotate() {
    let input = unit_cube();
    let mut e = EditNormalsMod::default();
    let corner = input.vertices.iter().find(|(_, v)| v.position.distance(Vec3::ONE) < 1e-5).unwrap().0;
    let keys = keys_at_vertex(&input, corner);
    assert_eq!(keys.len(), 3);
    e.selected.extend(keys.iter().copied());
    e.unify_selected(&input);
    let mut m = input.clone();
    e.apply(&mut m);
    let normals = m.corner_normals();
    for &(p, i) in &keys {
        assert!((normals[&p][i] - Vec3::ONE.normalize()).length() < 1e-5);
    }
    assert!(m.validate().is_ok());
    assert_eq!(m.counts(), input.counts());
    // Reset returns them to smoothing-group (faceted) control
    e.reset_selected();
    assert!(e.explicit.is_empty());
    // Rotate a top face's normals 90 degrees about X: +Z -> -Y
    let top = input.polygons.keys().find(|&p| input.poly_normal(p).z > 0.9).unwrap();
    e.selected = keys_of_face(&input, top).into_iter().collect();
    e.rotate_selected(&input, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
    let cur = e.current(&input);
    for k in keys_of_face(&input, top) {
        assert!((cur[&k].0 - Vec3::NEG_Y).length() < 1e-5);
        assert_eq!(cur[&k].1, NormalKind::Explicit);
    }
    assert_eq!(cur.len(), corners(&input).count());
}

#[test]
fn edit_normals_break_gives_face_normals_on_a_smooth_sphere() {
    let input = sphere();
    let mut e = EditNormalsMod::default();
    let v = input.polygons.values().next().unwrap().corners[0].vertex_id;
    let keys = keys_at_vertex(&input, v);
    let before = e.current(&input);
    assert!(keys.iter().all(|k| before[k].1 == NormalKind::Unified));
    e.selected.extend(keys.iter().copied());
    e.break_selected(&input);
    let after = e.current(&input);
    for &(p, i) in &keys {
        assert!((after[&(p, i)].0 - input.poly_normal(p)).length() < 1e-5);
    }
    let mut m = input.clone();
    e.apply(&mut m);
    assert!(m.validate().is_ok());
    let t = m.to_tri_mesh();
    assert!(t.normals.iter().all(|n| Vec3::from_array(*n).is_finite()));
}
