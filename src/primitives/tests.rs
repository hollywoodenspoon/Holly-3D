use super::*;

fn check(m: &EditablePoly, v: usize, e: usize, f: usize) {
    m.validate().unwrap();
    assert_eq!(m.counts(), (v, e, f));
}

#[test]
fn box_1x1x1() {
    let m = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    check(&m, 8, 12, 6);
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
    assert!((m.signed_volume() - 1.0).abs() < 1e-4, "outward winding");
}

#[test]
fn box_segmented() {
    let m = Primitive::Box { length: 2.0, width: 3.0, height: 1.0, length_segs: 2, width_segs: 3, height_segs: 4 }.to_editable_poly();
    // faces: 2*(3*2) + 2*(3*4) + 2*(2*4) = 12 + 24 + 16
    assert_eq!(m.polygons.len(), 52);
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
    assert!((m.signed_volume() - 6.0).abs() < 1e-3);
    m.validate().unwrap();
}

#[test]
fn plane_grid() {
    let m = Primitive::Plane { length: 1.0, width: 1.0, length_segs: 4, width_segs: 3 }.to_editable_poly();
    check(&m, 20, 31, 12);
    assert!(!m.is_closed());
}

#[test]
fn sphere_closed() {
    let m = Primitive::Sphere { radius: 1.0, segments: 16, hemisphere: 0.0 }.to_editable_poly();
    // 8 latitude bands: 2 polar fans of 16 tris + 6 bands of 16 quads
    assert_eq!(m.polygons.len(), 16 * 8);
    assert_eq!(m.vertices.len(), 16 * 7 + 2);
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
    assert!(m.signed_volume() > 3.5 && m.signed_volume() < 4.19);
    m.validate().unwrap();
}

#[test]
fn hemisphere_capped() {
    let m = Primitive::Sphere { radius: 1.0, segments: 16, hemisphere: 0.5 }.to_editable_poly();
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
    assert!(m.signed_volume() > 0.0);
    m.validate().unwrap();
}

#[test]
fn cylinder_counts() {
    let m = Primitive::Cylinder { radius: 1.0, height: 2.0, height_segs: 1, cap_segs: 1, sides: 12 }.to_editable_poly();
    check(&m, 24, 36, 14);
    assert!(m.is_closed());
    assert!(m.signed_volume() > 0.0);
}

#[test]
fn cylinder_cap_segments() {
    let m = Primitive::Cylinder { radius: 1.0, height: 2.0, height_segs: 3, cap_segs: 2, sides: 8 }.to_editable_poly();
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
    m.validate().unwrap();
}

#[test]
fn cone_apex() {
    let m = Primitive::default_of(PrimitiveKind::Cone).to_editable_poly();
    assert!(m.is_closed());
    assert_eq!(m.euler(), 2);
    assert_eq!(m.vertices.len(), 24 + 1);
    assert!(m.signed_volume() > 0.0);
    m.validate().unwrap();
}

#[test]
fn torus_genus_one() {
    let m = Primitive::Torus { radius1: 1.0, radius2: 0.25, segments: 12, sides: 8 }.to_editable_poly();
    check(&m, 96, 192, 96);
    assert_eq!(m.euler(), 0);
    assert!(m.signed_volume() > 0.0);
}

#[test]
fn sphere_normals_point_outward() {
    let m = Primitive::Sphere { radius: 1.0, segments: 12, hemisphere: 0.0 }.to_editable_poly();
    let t = m.to_tri_mesh();
    for (p, n) in t.positions.iter().zip(&t.normals) {
        assert!(Vec3::from_array(*p).dot(Vec3::from_array(*n)) > 0.5);
    }
}
