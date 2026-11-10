//! Headless tests for the modifier stack: Edit Poly, FFD, UVW Map, Unwrap UVW.
use super::*;
use crate::mesh::ops::ExtrudeMode;
use crate::primitives::{Primitive, PrimitiveKind};
use bevy::math::{Mat4, Vec2, Vec3};

#[test]
fn edit_poly_replays_recorded_ops() {
    let base = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    let mut m = base.clone();
    let top: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).dot(Vec3::Z) > 0.9).collect();
    m.polygons[top[0]].selected = true;
    let sel = m.selection_sets();
    let op = PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 1.0 };
    op.apply(&mut m);
    let md = EditPolyMod { ops: vec![(sel, op)], selection: m.selection_sets() };
    let mut out = base.clone();
    md.apply(&mut out);
    out.validate().unwrap();
    assert_eq!(out.counts(), (12, 20, 10));
    assert_eq!(out.selected_polygons().len(), 1);
    let mut disabled = ModifierEntry::new(ModifierKind::EditPoly(md));
    disabled.enabled_in_viewport = false;
    let mut out2 = base.clone();
    disabled.apply(&mut out2);
    assert_eq!(out2.counts(), (8, 12, 6));
}

fn box_mesh() -> EditablePoly {
    Primitive::default_of(PrimitiveKind::Box).to_editable_poly()
}

fn uvs_valid(m: &EditablePoly) -> bool {
    m.polygons.values().flat_map(|p| p.corners.iter()).all(|c| c.uv.is_finite())
}

#[test]
fn ffd_identity_lattice_keeps_mesh() {
    for dim in 2..=4 {
        let base = box_mesh();
        let mut f = FfdMod::new(dim);
        f.fit(&base);
        assert_eq!(f.points.len(), dim * dim * dim);
        assert_eq!(f.cage_edges().len(), 3 * dim * dim * (dim - 1));
        let mut m = base.clone();
        f.apply(&mut m);
        m.validate().unwrap();
        assert_eq!(m.counts(), (8, 12, 6));
        for (a, b) in base.vertices.values().zip(m.vertices.values()) {
            assert!((a.position - b.position).length() < 1e-5);
        }
    }
}

#[test]
fn ffd_moving_top_points_lifts_top_and_keeps_counts() {
    let base = Primitive::Box { length: 1.0, width: 1.0, height: 1.0, length_segs: 2, width_segs: 2, height_segs: 2 }.to_editable_poly();
    let mut f = FfdMod::new(3);
    f.fit(&base);
    for idx in 0..f.points.len() {
        if f.coords(idx).2 == 2 {
            let p = f.point_pos(idx);
            f.set_point_pos(idx, p + Vec3::Z);
        }
    }
    let mut m = base.clone();
    f.apply(&mut m);
    m.validate().unwrap();
    assert_eq!(m.counts(), base.counts());
    let top = m.vertices.values().map(|v| v.position.z).fold(f32::MIN, f32::max);
    let bottom = m.vertices.values().map(|v| v.position.z).fold(f32::MAX, f32::min);
    assert!((top - 2.0).abs() < 1e-4, "top {top}");
    assert!(bottom.abs() < 1e-4);
    assert!(m.polygons.keys().all(|p| m.poly_normal(p).is_normalized()));
}

#[test]
fn ffd_select_along_and_lattice_move() {
    let base = box_mesh();
    let mut f = FfdMod::new(4);
    f.fit(&base);
    f.selected[0] = true;
    f.select_along(0);
    assert_eq!(f.selected.iter().filter(|s| **s).count(), 4);
    // Moving the whole lattice with an untouched grid is still the identity.
    f.lattice = Mat4::from_translation(Vec3::X * 0.25);
    let mut m = base.clone();
    f.only_in_volume = false;
    f.apply(&mut m);
    for (a, b) in base.vertices.values().zip(m.vertices.values()) {
        assert!((a.position - b.position).length() < 1e-4);
    }
}

#[test]
fn ffd_conform_pulls_points_onto_sphere() {
    let sphere = Primitive::default_of(PrimitiveKind::Sphere).to_editable_poly();
    let mut f = FfdMod::new(2);
    f.fit(&sphere);
    f.conform(&sphere);
    for i in 0..f.points.len() {
        let r = f.point_pos(i).length();
        assert!(r < 0.61 && r > 0.5, "corner radius {r}");
    }
}

#[test]
fn uvw_map_planar_fits_top_face_into_unit_square() {
    let mut m = box_mesh();
    let mut u = UvwMapMod::default();
    u.fit(&m);
    u.apply(&mut m);
    assert!(uvs_valid(&m));
    let top = m.polygons.values().find(|p| p.vertex_ids().all(|v| m.pos(v).z > 0.9)).unwrap();
    let mut uvs: Vec<_> = top.corners.iter().map(|c| (c.uv * 100.0).round() / 100.0).collect();
    uvs.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    assert_eq!(uvs, vec![Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)]);
    // tiling and flipping
    u.u_tile = 2.0;
    u.v_flip = true;
    u.apply(&mut m);
    let max_u = m.polygons.values().flat_map(|p| p.corners.iter()).map(|c| c.uv.x).fold(0.0, f32::max);
    assert!((max_u - 2.0).abs() < 1e-4);
    assert_eq!(m.counts(), (8, 12, 6));
}

#[test]
fn uvw_map_bitmap_fit_matches_image_aspect() {
    let mut m = box_mesh();
    let mut u = UvwMapMod::default();
    u.fit(&m);
    let w = u.width;
    u.bitmap_fit(2.0); // a 2:1 bitmap
    assert!((u.width - w).abs() < 1e-6 && (u.length - w * 0.5).abs() < 1e-6);
    u.bitmap_fit(0.0); // ignored
    assert!((u.length - w * 0.5).abs() < 1e-6);
    u.apply(&mut m);
    let top = m.polygons.values().find(|p| p.vertex_ids().all(|v| m.pos(v).z > 0.9)).unwrap();
    let span = |f: fn(Vec2) -> f32| {
        let vs: Vec<f32> = top.corners.iter().map(|c| f(c.uv)).collect();
        vs.iter().cloned().fold(f32::MIN, f32::max) - vs.iter().cloned().fold(f32::MAX, f32::min)
    };
    // The square top now covers the whole bitmap across and twice it down.
    assert!((span(|v| v.x) - 1.0).abs() < 1e-4 && (span(|v| v.y) - 2.0).abs() < 1e-4);
}

#[test]
fn uvw_map_every_type_gives_finite_uvs() {
    let cyl = Primitive::default_of(PrimitiveKind::Cylinder).to_editable_poly();
    for kind in MapType::ALL {
        for axis in 0..3 {
            let mut m = cyl.clone();
            let mut u = UvwMapMod { kind, cap: true, ..Default::default() };
            u.set_axis(axis, &m);
            u.apply(&mut m);
            m.validate().unwrap();
            assert_eq!(m.counts(), cyl.counts());
            assert!(uvs_valid(&m), "{kind:?} axis {axis}");
        }
    }
}

#[test]
fn uvw_map_cylindrical_wraps_without_seam_jump() {
    let mut m = Primitive::default_of(PrimitiveKind::Cylinder).to_editable_poly();
    let mut u = UvwMapMod { kind: MapType::Cylindrical, ..Default::default() };
    u.fit(&m);
    u.apply(&mut m);
    for p in m.polygons.values().filter(|p| p.corners.len() == 4) {
        let (lo, hi) = p.corners.iter().fold((f32::MAX, f32::MIN), |a, c| (a.0.min(c.uv.x), a.1.max(c.uv.x)));
        assert!(hi - lo < 0.5);
    }
}

#[test]
fn unwrap_capture_edit_and_relax() {
    let base = box_mesh();
    let mut u = UnwrapMod::default();
    u.capture(&base);
    assert!(u.matches(&base));
    let cv = unwrap_uvw::corner_vertices(&base);
    u.selected.insert((0, 0));
    u.expand_selection(&cv);
    let before = u.uv((0, 0));
    u.move_selected(Vec2::new(0.25, 0.0));
    assert!((u.uv((0, 0)) - before - Vec2::new(0.25, 0.0)).length() < 1e-6);
    u.rotate_selected(1.0);
    u.scale_selected(Vec2::splat(2.0));
    u.selected.clear();
    u.relax(&cv, 5, 0.5);
    let mut m = base.clone();
    u.apply(&mut m);
    assert!(uvs_valid(&m));
    assert_eq!(m.counts(), (8, 12, 6));
    // topology mismatch: modifier passes the input through untouched
    let sphere = Primitive::default_of(PrimitiveKind::Sphere).to_editable_poly();
    assert!(!u.matches(&sphere));
}

#[test]
fn unwrap_stitch_welds_split_uvs() {
    let base = Primitive::Plane { length: 1.0, width: 1.0, length_segs: 1, width_segs: 2 }.to_editable_poly();
    let mut u = UnwrapMod::default();
    u.capture(&base);
    let cv = unwrap_uvw::corner_vertices(&base);
    // split polygon 1 away, then stitch it back
    for c in 0..u.uvs[1].len() {
        u.uvs[1][c].x += 0.3;
    }
    u.selected = u.all_corners().collect();
    assert_eq!(u.stitch(&cv), 2);
    let shared: Vec<_> = cv[0].iter().filter(|v| cv[1].contains(v)).collect();
    assert_eq!(shared.len(), 2);
}

#[test]
fn smart_uv_unwrap_packs_box_into_unit_square() {
    for mesh in [box_mesh(), Primitive::default_of(PrimitiveKind::Torus).to_editable_poly()] {
        let mut u = UnwrapMod::default();
        u.capture(&mesh);
        let charts = u.smart_unwrap(&mesh).unwrap();
        assert!(charts >= 1);
        let mut m = mesh.clone();
        u.apply(&mut m);
        m.validate().unwrap();
        assert_eq!(m.counts(), mesh.counts());
        for c in m.polygons.values().flat_map(|p| p.corners.iter()) {
            assert!((-1e-4..=1.0001).contains(&c.uv.x) && (-1e-4..=1.0001).contains(&c.uv.y), "{:?}", c.uv);
        }
    }
    // a box has 6 faces 90° apart: with a 66° limit each becomes its own chart
    let mut u = UnwrapMod::default();
    assert_eq!(u.smart_unwrap(&box_mesh()).unwrap(), 6);
}

#[test]
fn stack_add_modifier_fits_gizmos() {
    let mut s = crate::scene::Scene::default();
    let id = s.add(crate::scene::BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::ZERO);
    let o = &mut s.objects[id];
    o.add_modifier(make_modifier("FFD 3x3x3").unwrap());
    o.add_modifier(make_modifier("UVW Map").unwrap());
    o.add_modifier(make_modifier("Unwrap UVW").unwrap());
    assert_eq!(o.modifiers.len(), 3);
    match &o.modifiers[1].kind {
        ModifierKind::UvwMap(u) => assert!((u.width - 1.0).abs() < 1e-4),
        _ => panic!(),
    }
    assert!(o.active_modifier().unwrap().gizmo_subs().is_empty());
    assert_eq!(o.evaluated_mesh.counts(), (8, 12, 6));
}
