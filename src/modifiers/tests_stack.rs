//! Phase 5d stack modifiers: Greeble, Chamfer, Symmetry, Shell,
//! TurboSmooth, Bend, Twist, Taper and Push.

use super::*;
use crate::mesh::EditablePoly;
use crate::primitives::{Primitive, PrimitiveKind};
use bevy::math::{Vec2, Vec3};

fn prim(kind: PrimitiveKind) -> EditablePoly {
    Primitive::default_of(kind).to_editable_poly()
}

fn tall_box() -> EditablePoly {
    Primitive::Box { length: 0.4, width: 0.4, height: 2.0, length_segs: 1, width_segs: 1, height_segs: 8 }.to_editable_poly()
}

fn quad() -> EditablePoly {
    let mut m = EditablePoly::new();
    let v: Vec<_> = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].iter().map(|&(x, y)| m.add_vertex(Vec3::new(x, y, 0.))).collect();
    m.add_polygon_uv(&v, &[Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]);
    m.finalize();
    m
}

fn check(m: &EditablePoly, what: &str) {
    assert!(m.validate().is_ok(), "{what}: {:?}", m.validate());
    assert!(m.vertices.values().all(|v| v.position.is_finite()), "{what}: finite");
    assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())), "{what}: UVs");
}

fn closed(m: &EditablePoly, what: &str) {
    check(m, what);
    assert!(m.is_closed(), "{what}: closed");
    assert!(m.signed_volume() > 0.0, "{what}: outward normals");
}

#[test]
fn turbosmooth_cube_counts() {
    let mut m = prim(PrimitiveKind::Box);
    TurboSmoothMod::default().apply(&mut m);
    assert_eq!(m.counts(), (26, 48, 24));
    closed(&m, "1 iteration");
    let mut m2 = prim(PrimitiveKind::Box);
    TurboSmoothMod { iterations: 2, ..Default::default() }.apply(&mut m2);
    assert_eq!(m2.counts(), (98, 192, 96));
    closed(&m2, "2 iterations");
    assert!(m2.signed_volume() < 1.0, "smoothing shrinks the cube");
    assert_eq!(m2.edges.values().filter(|e| !e.hidden).count(), 12 * 4, "isoline: only the 12 cage edges (4 segments each) show");
    let mut all = prim(PrimitiveKind::Box);
    TurboSmoothMod { iterations: 2, isoline: false, ..Default::default() }.apply(&mut all);
    assert!(all.edges.values().all(|e| !e.hidden));
}

#[test]
fn turbosmooth_borders_and_separation() {
    let mut p = quad();
    TurboSmoothMod::default().apply(&mut p);
    assert_eq!(p.counts(), (9, 12, 4));
    let b = p.bounds().unwrap();
    assert!(b.min.abs_diff_eq(Vec3::ZERO, 1e-5) && b.max.abs_diff_eq(Vec3::new(1.0, 1.0, 0.0), 1e-5), "plane corners stay put");
    // box faces each have their own smoothing group: separated, the cube keeps its shape
    let mut c = prim(PrimitiveKind::Box);
    TurboSmoothMod { iterations: 2, separate_smoothing: true, ..Default::default() }.apply(&mut c);
    assert!((c.signed_volume() - 1.0).abs() < 1e-4, "{}", c.signed_volume());
    closed(&c, "separated");
}

#[test]
fn symmetry_slices_mirrors_and_welds() {
    let mut m = prim(PrimitiveKind::Box);
    SymmetryMod::default().apply(&mut m);
    assert_eq!(m.counts(), (12, 20, 10), "box halves welded along x = 0");
    closed(&m, "symmetry");
    let mut open = prim(PrimitiveKind::Box);
    SymmetryMod { weld: false, ..Default::default() }.apply(&mut open);
    assert_eq!(open.counts(), (16, 24, 10));
    assert!(!open.is_closed(), "unwelded halves stay separate");
    let mut no_slice = prim(PrimitiveKind::Box);
    SymmetryMod { slice: false, weld: false, axis: 2, ..Default::default() }.apply(&mut no_slice);
    assert_eq!(no_slice.counts(), (16, 24, 12), "two whole boxes, mirrored across z = 0");
    assert!(no_slice.bounds().unwrap().min.z < -0.99);
}

#[test]
fn shell_plane_and_box() {
    let mut m = quad();
    ShellMod { inner: 0.1, ..Default::default() }.apply(&mut m);
    assert_eq!(m.counts(), (8, 12, 6));
    closed(&m, "shell");
    let mut s2 = quad();
    ShellMod { inner: 0.1, segments: 2, select_edges: true, override_edge_mat: Some(3), ..Default::default() }.apply(&mut s2);
    assert_eq!(s2.counts(), (12, 20, 10));
    closed(&s2, "shell 2 segments");
    assert_eq!(s2.polygons.values().filter(|p| p.selected && p.material_id == 3).count(), 8);
    let mut b = prim(PrimitiveKind::Box);
    ShellMod { inner: 0.1, ..Default::default() }.apply(&mut b);
    assert_eq!(b.counts(), (16, 24, 12), "closed input: outer + inner layer");
    check(&b, "box shell");
    assert!((b.signed_volume() - (1.0 - 0.8f32.powi(3))).abs() < 1e-3, "wall volume {}", b.signed_volume());
}

#[test]
fn bend_twist_taper_math() {
    let base = tall_box();
    let top = |m: &EditablePoly| m.vertices.values().map(|v| v.position).fold(Vec3::splat(f32::MIN), Vec3::max);
    let mut b = base.clone();
    BendMod { angle: 90.0, ..Default::default() }.apply(&mut b);
    assert_eq!(b.counts(), base.counts());
    check(&b, "bend");
    let r = 2.0 / std::f32::consts::FRAC_PI_2;
    // the column's axis (x = 0, z = 2) ends at (r, 0, r)
    let tip = b.vertices.values().map(|v| v.position).filter(|p| (p.z - r).abs() < 0.25).count();
    assert!(tip > 0 && top(&b).x > r - 0.25, "bent over towards +X: {:?}", top(&b));
    let mut lim = base.clone();
    BendMod { angle: 90.0, limits: Limits { on: true, upper: 1.0, lower: 0.0 }, ..Default::default() }.apply(&mut lim);
    assert!(lim.vertices.values().all(|v| v.position.z < 1.0 + r * 0.5 + 0.25), "past the limit it runs straight");

    let mut t = base.clone();
    TwistMod { angle: 90.0, ..Default::default() }.apply(&mut t);
    for (a, b) in base.vertices.values().zip(t.vertices.values()) {
        if a.position.z > 1.99 {
            assert!(b.position.abs_diff_eq(Vec3::new(-a.position.y, a.position.x, a.position.z), 1e-4));
        }
        if a.position.z < 1e-4 {
            assert_eq!(a.position, b.position);
        }
    }
    let mut tp = base.clone();
    TaperMod { amount: -1.0, ..Default::default() }.apply(&mut tp);
    assert!(tp.vertices.values().filter(|v| v.position.z > 1.99).all(|v| v.position.truncate().length() < 1e-5), "tapers to a point");
    let mut first = base.clone();
    TaperMod { amount: 1.0, effect: TaperEffect::First, ..Default::default() }.apply(&mut first);
    let b = first.bounds().unwrap();
    assert!((b.max.x - 0.4).abs() < 1e-4 && (b.max.y - 0.2).abs() < 1e-4, "only X doubles: {b:?}");
}

#[test]
fn push_inflates_along_normals() {
    let mut s = prim(PrimitiveKind::Sphere);
    PushMod { value: 0.1 }.apply(&mut s);
    assert!(s.vertices.values().all(|v| (v.position.length() - 0.7).abs() < 0.01));
}

#[test]
fn chamfer_modifier_filters() {
    let mut all = prim(PrimitiveKind::Box);
    ChamferMod::default().apply(&mut all);
    assert_eq!(all.counts(), (24, 48, 26), "6 faces + 12 edge strips + 8 corner triangles");
    closed(&all, "chamfer all");
    let mut none = prim(PrimitiveKind::Box);
    ChamferMod { filter: ChamferFilter::Selected, ..Default::default() }.apply(&mut none);
    assert_eq!(none.counts(), (8, 12, 6), "nothing selected");
    let mut steep = prim(PrimitiveKind::Box);
    ChamferMod { filter: ChamferFilter::Angle { min: 100.0, max: 180.0 }, ..Default::default() }.apply(&mut steep);
    assert_eq!(steep.counts(), (8, 12, 6), "box edges are 90°");
    let mut sg = prim(PrimitiveKind::Box);
    ChamferMod { filter: ChamferFilter::SmoothingBoundaries, ..Default::default() }.apply(&mut sg);
    assert_eq!(sg.counts(), all.counts());
}

#[test]
fn greeble_panels_and_widgets() {
    let mut m = quad();
    GreebleMod { levels: 2, widget_density: 0.0, select_tops: true, ..Default::default() }.apply(&mut m);
    assert_eq!(m.counts(), (4 + 4 * 8, 4 + 4 * 12, 1 + 4 * 6), "4 closed panels on the quad");
    check(&m, "greeble");
    assert_eq!(m.selected_polygons().len(), 4, "panel tops selected");
    assert!(m.polygons.values().all(|p| p.material_id == 1));
    let mut w = quad();
    GreebleMod { levels: 3, widget_density: 1.0, ..Default::default() }.apply(&mut w);
    assert!(w.polygons.len() > 1 + 8 * 6, "widgets added");
    assert!(w.polygons.values().any(|p| p.material_id == 2), "widget material");
    let mut again = quad();
    GreebleMod { levels: 3, widget_density: 1.0, ..Default::default() }.apply(&mut again);
    assert_eq!(w.counts(), again.counts(), "deterministic per seed");
    let mut boxes = prim(PrimitiveKind::Box);
    GreebleMod::default().apply(&mut boxes);
    check(&boxes, "greeble box");
}
