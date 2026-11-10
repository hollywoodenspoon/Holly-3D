//! Architectural kit tests: closed boxes / prisms / cylinders with outward
//! normals, roof and arch shapes, and point-in-polygon.

use super::roofs::arch_curve;
use super::*;

fn closed_ok(m: &EditablePoly) {
    m.validate().unwrap();
    assert!(m.is_closed(), "closed");
    assert!(m.signed_volume() > 0.0, "outward normals");
}

#[test]
fn block_is_a_closed_box() {
    let mut k = Kit::new();
    k.mat(4).block(Vec3::ZERO, Vec3::new(2.0, 1.0, 3.0));
    let m = k.finish();
    assert_eq!(m.counts(), (8, 12, 6));
    closed_ok(&m);
    assert!((m.signed_volume() - 6.0).abs() < 1e-4);
    assert!(m.polygons.values().all(|p| p.material_id == 4));
    assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())));
}

#[test]
fn beam_cuboid_and_frame() {
    let mut k = Kit::new();
    k.beam(Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0), 0.2, 0.2);
    k.xf = frame(Vec3::new(10.0, 0.0, 0.0), 1.0);
    k.cuboid(Vec3::ZERO, Vec3::splat(0.5), Quat::IDENTITY);
    let m = k.finish();
    assert_eq!(m.counts(), (16, 24, 12));
    assert!((m.signed_volume() - (5.0 * 0.04 + 1.0)).abs() < 1e-3);
    let b = m.bounds().unwrap();
    assert!(b.max.x > 10.5, "frame offset applied");
}

#[test]
fn prism_frustum_cone_tube() {
    let sq = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)];
    let mut k = Kit::new();
    k.prism(&sq, 0.0, 2.0, true, true);
    closed_ok(&k.finish());
    let mut k = Kit::new();
    k.frustum(Vec3::ZERO, 1.0, 0.5, 2.0, 12, true);
    let m = k.finish();
    assert_eq!(m.counts(), (24, 36, 14));
    closed_ok(&m);
    let mut k = Kit::new();
    k.frustum(Vec3::ZERO, 1.0, 0.0, 2.0, 8, true);
    let m = k.finish();
    assert_eq!(m.counts(), (9, 16, 9));
    closed_ok(&m);
    let mut k = Kit::new();
    k.tube(Vec3::ZERO, Vec3::new(0.0, 5.0, 0.0), 0.1, 6);
    let m = k.finish();
    closed_ok(&m);
    assert!((m.bounds().unwrap().size().y - 5.0).abs() < 1e-4);
}

#[test]
fn roofs_face_up_and_gable_ends_take_wall_id() {
    let mut k = Kit::new();
    k.mat(5).gable(3.0, 2.0, 2.0, 0.3, 4);
    let m = k.finish();
    assert_eq!(m.polygons.len(), 6);
    let ids: Vec<u16> = m.polygons.values().map(|p| p.material_id).collect();
    assert_eq!(ids.iter().filter(|&&i| i == 4).count(), 2, "two gable ends");
    let up = m.polygons.keys().filter(|&p| m.polygons[p].material_id == 5 && m.poly_normal(p).z > 0.3).count();
    assert_eq!(up, 2, "two upward slopes");
    let mut k = Kit::new();
    k.hipped(2.0, 2.0, 1.5, 0.0);
    let m = k.finish();
    assert!(m.polygons.keys().filter(|&p| m.poly_normal(p).z > 0.3).count() == 4);
}

#[test]
fn arch_curves_span_and_rise() {
    for pointed in [false, true] {
        let c = arch_curve(4.0, 3.0, pointed, 12);
        assert_eq!(c.len(), 13);
        assert!(c[0].abs_diff_eq(Vec2::new(-2.0, 0.0), 1e-4), "{pointed}: {:?}", c[0]);
        assert!(c[12].abs_diff_eq(Vec2::new(2.0, 0.0), 1e-4));
        let top = c.iter().map(|p| p.y).fold(0.0, f32::max);
        assert!((top - 3.0).abs() < 1e-3, "{pointed}: {top}");
        assert!(c[6].x.abs() < 1e-3, "crown centred");
    }
    let mut k = Kit::new();
    k.arched_panel(1.0, 2.0, true);
    let m = k.finish();
    let p = m.polygons.keys().next().unwrap();
    assert!(m.poly_normal(p).y < -0.99, "panel faces -Y");
    let mut k = Kit::new();
    k.arch_wall(4.0, 2.0, 3.0, 0.5, false);
    k.finish().validate().unwrap();
}

#[test]
fn ground_grid_shares_vertices_and_clips() {
    let mut k = Kit::new();
    k.ground(Vec2::ZERO, Vec2::splat(8.0), 2.0, |_| 0.0, |_| true);
    let m = k.finish();
    assert_eq!(m.counts(), (25, 40, 16));
    let tri = [Vec2::ZERO, Vec2::new(8.0, 0.0), Vec2::new(0.0, 8.0)];
    assert!(inside(&tri, Vec2::new(1.0, 1.0)) && !inside(&tri, Vec2::new(6.0, 6.0)));
    let mut k = Kit::new();
    k.ground(Vec2::ZERO, Vec2::splat(8.0), 2.0, |_| 0.0, |p| inside(&tri, p));
    k.strip(Vec2::ZERO, Vec2::new(10.0, 0.0), 2.0, 2.5, |_| 0.1);
    assert!(k.finish().polygons.len() < 16);
}
