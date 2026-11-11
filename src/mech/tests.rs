use super::*;
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};

fn check(m: &EditablePoly, v: usize, e: usize, f: usize, euler: i64) {
    m.validate().unwrap();
    assert_eq!(m.counts(), (v, e, f));
    assert!(m.is_closed(), "watertight");
    assert_eq!(m.euler(), euler);
    assert!(m.signed_volume() > 0.0, "outward winding");
}

fn radial(m: &EditablePoly) -> (f32, f32) {
    let r = m.vertices.values().map(|v| v.position.truncate().length());
    r.fold((f32::MAX, 0.0), |(lo, hi), x| (lo.min(x), hi.max(x)))
}

#[test]
fn spur_gear_counts_and_radii() {
    let g = Gear { bore: 0.0, ..Gear::default() };
    // 20 teeth × 16 outline points; 7 layers (6 for the helix); a center
    // point on each cap fans to the outline.
    let n = 320;
    let m = g.build();
    check(&m, n * 7 + 2, n * 7 + n * 6 + 2 * n, 2 * n + 6 * n, 2);
    let (r, md) = (g.pitch_radius(), g.module);
    assert!((r - 1.0).abs() < 1e-6, "module 0.1 × 20 teeth / 2");
    let tips = m.vertices.values().map(|v| v.position.truncate().length()).filter(|&x| x > 0.01);
    let (lo, hi) = tips.fold((f32::MAX, 0.0f32), |(a, b), x| (a.min(x), b.max(x)));
    assert!((hi - (r + md)).abs() < 1e-4, "addendum {hi}");
    assert!((lo - (r - 1.25 * md)).abs() < 1e-4, "dedendum {lo}");
    // Smoothing groups: flat caps (1) and flanks (2).
    assert!(m.polygons.values().all(|p| p.smoothing_group == 1 || p.smoothing_group == 2));
    // With a bore the gear has a hole (genus 1).
    let b = Gear::default().build();
    check(&b, 2 * n * 7, 2 * n * 7 + 2 * n * 6 + 2 * n, 2 * n + 2 * n * 6, 0);
    assert!((radial(&b).0 - 0.2).abs() < 1e-4, "bore radius");
}

#[test]
fn ring_gear_teeth_point_inward() {
    let g = Gear { kind: GearType::Ring, teeth: 40, ..Gear::default() };
    let m = g.build();
    m.validate().unwrap();
    assert!(m.is_closed() && m.euler() == 0 && m.signed_volume() > 0.0);
    let (lo, hi) = radial(&m);
    let r = g.pitch_radius();
    assert!((lo - (r - g.module)).abs() < 1e-4, "internal tooth tips at r - m: {lo}");
    assert!((hi - (r + 3.75 * g.module)).abs() < 1e-4, "rim");
}

#[test]
fn rack_and_helix() {
    let g = Gear { kind: GearType::Rack, teeth: 10, ..Gear::default() };
    let m = g.build();
    m.validate().unwrap();
    assert!(m.is_closed() && m.euler() == 2 && m.signed_volume() > 0.0);
    let b = m.bounds().unwrap();
    assert!((b.size().x - 10.0 * std::f32::consts::PI * g.module).abs() < 1e-4, "length = teeth × circular pitch");
    assert!((b.max.y - g.module).abs() < 1e-5 && (b.size().z - g.thickness).abs() < 1e-5);
    // Helix: the top face is turned by thickness × tan(helix) / pitch radius.
    let h = Gear { helix: 20.0, bore: 0.0, ..Gear::default() };
    let hm = h.build();
    hm.validate().unwrap();
    assert!(hm.signed_volume() > 0.0);
    let ang = |z: f32| {
        let p = hm.vertices.values().map(|v| v.position).find(|p| (p.z - z).abs() < 1e-5 && p.truncate().length() > 1.09).unwrap();
        p.y.atan2(p.x)
    };
    let want = h.thickness * 20f32.to_radians().tan() / h.pitch_radius();
    let got = ang(h.thickness) - ang(0.0);
    // The first tip found may be any tooth: compare modulo a tooth pitch.
    let pitch = std::f32::consts::TAU / 20.0;
    let d = (got - want).rem_euclid(pitch);
    assert!(d.min(pitch - d) < 1e-3, "twist {got} vs {want}");
}

#[test]
fn hose_hangs_between_its_ends() {
    let h = Hose::default();
    let m = h.build();
    check(&m, 33 * 12, 32 * 12 * 2 + 12, 32 * 12 + 2, 2);
    let first: Vec3 = m.vertices.values().take(12).map(|v| v.position).sum::<Vec3>() / 12.0;
    assert!(first.distance(h.start) < 1e-4, "starts at its start point");
    assert!((h.point(0.5).z - (h.start.z - h.sag)).abs() < 1e-4, "gravity sag at the middle");
    let slack = Hose { slack: 0.5, ..h.clone() };
    assert!(slack.point(0.5).z < h.point(0.5).z, "slack hangs lower");
    let cr = Hose { curve: HoseCurve::CatmullRom, ..h.clone() };
    assert!(cr.point(0.0).distance(h.start) < 1e-4 && cr.point(1.0).distance(h.end) < 1e-4);
    assert!((cr.point(0.5).z - (h.start.z - h.sag)).abs() < 1e-3, "passes through the sagging middle");
    // Corrugated: round rings dip by Ring Depth; square rings make steps.
    for square in [false, true] {
        let c = Hose { rings: 8, ring_depth: 0.02, square_rings: square, ..h.clone() };
        let m = c.build();
        m.validate().unwrap();
        assert!(m.is_closed() && m.signed_volume() > 0.0);
        assert!(m.vertices.len() > 33 * 12);
    }
}

#[test]
fn piston_parts_have_an_eyelet_and_a_body() {
    for p in [PistonPart::housing(), PistonPart::rod()] {
        let m = p.build();
        m.validate().unwrap();
        assert!(m.is_closed());
        // Torus (χ 0) + barrel (χ 2) + gland on the housing (χ 2).
        assert_eq!(m.euler(), if p.rod { 2 } else { 4 });
        let b = m.bounds().unwrap();
        assert!((b.max.z - (1.2 * p.eyelet + p.length)).abs() < 1e-4);
        assert!(b.min.z < -p.eyelet, "eyelet around the pivot");
        let _ = Vec2::ZERO;
    }
}

mod fasteners {
    use crate::mech::{Fastener, FastenerKind, ThreadStd};

    fn closed(m: &crate::mesh::EditablePoly) -> bool {
        m.euler() == 2 || m.euler() == 0
    }

    #[test]
    fn bolt_is_a_closed_solid_with_a_thread() {
        let f = Fastener::new(FastenerKind::Bolt);
        let m = f.build();
        let (v, e, p) = m.counts();
        assert!(v > 1000 && e > v && p > 1000, "{v} {e} {p}");
        assert_eq!(m.euler(), 2, "closed, genus 0");
        assert!(m.signed_volume() > 0.0, "outward normals");
        assert!(m.polygons.values().all(|p| p.corners.len() >= 3));
        // The thread makes the shank radius vary between root and crest.
        let k = 0.001 * f.scale;
        let radii: Vec<f32> = m.vertices.values().filter(|v| v.position.z > (f.spec().head + 20.0) * k).map(|v| v.position.truncate().length() / k).filter(|r| *r > 1.0).collect();
        let (lo, hi) = radii.iter().fold((f32::MAX, 0.0f32), |(a, b), &r| (a.min(r), b.max(r)));
        assert!((hi - 5.0).abs() < 0.05, "crest at the M10 major radius ({hi})");
        assert!((5.0 - lo - f.depth()).abs() < 0.05, "root one thread depth in ({lo})");
    }

    #[test]
    fn nut_and_tapped_block_are_rings() {
        for kind in [FastenerKind::Nut, FastenerKind::TappedHole] {
            let m = Fastener::new(kind).build();
            assert_eq!(m.euler(), 0, "{kind:?}: a solid with one hole through it");
            assert!(m.signed_volume() > 0.0, "{kind:?}: outward normals");
            assert!(closed(&m));
            assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())));
        }
    }

    #[test]
    fn nut_fits_the_bolt_and_sizes_follow_the_table() {
        for (std, i) in [(ThreadStd::Metric, 0), (ThreadStd::Metric, 9), (ThreadStd::Imperial, 3)] {
            let mut nut = Fastener::new(FastenerKind::Nut);
            (nut.standard, nut.size) = (std, i);
            let s = nut.spec();
            let m = nut.build();
            let k = 0.001 * nut.scale;
            let inner = m.vertices.values().map(|v| v.position.truncate().length() / k).fold(f32::MAX, f32::min);
            assert!(inner < s.d * 0.5 && inner > s.d * 0.5 - nut.depth() - 0.01, "{}: hole clears the bolt root", s.name);
            let b = m.bounds().unwrap();
            assert!(((b.max.z - b.min.z) / k - s.nut).abs() < 1e-3, "{}: nut height", s.name);
        }
    }

    #[test]
    fn plain_option_is_light_and_still_closed() {
        let f = Fastener { threads: false, ..Fastener::new(FastenerKind::Bolt) };
        let m = f.build();
        assert!(m.counts().0 < 600, "no thread rows");
        assert_eq!(m.euler(), 2);
    }
}
