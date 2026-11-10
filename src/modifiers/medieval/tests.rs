use super::*;
use crate::generators::kit::ids;
use crate::primitives::{Primitive, PrimitiveKind};

fn fp(x: f32, y: f32, z: f32, round: bool) -> Footprint {
    Footprint { min: Vec3::new(-x * 0.5, -y * 0.5, 0.0), max: Vec3::new(x * 0.5, y * 0.5, z), round, path: None }
}

fn has(m: &EditablePoly, id: u16) -> bool {
    m.polygons.values().any(|p| p.material_id == id)
}

fn base_for(p: ArchPreset) -> Footprint {
    match p {
        ArchPreset::Watchtower => fp(8.0, 8.0, 16.0, true),
        ArchPreset::CurtainWall => fp(30.0, 3.0, 8.0, false),
        ArchPreset::TimberHouse => fp(10.0, 7.0, 10.0, false),
        ArchPreset::Bridge => fp(36.0, 5.0, 9.0, false),
        ArchPreset::Cathedral => fp(40.0, 16.0, 24.0, false),
    }
}

#[test]
fn every_preset_builds_valid_detailed_geometry() {
    for preset in ArchPreset::ALL {
        let a = ArchitectMod { preset, ..Default::default() };
        let f = base_for(preset);
        let m = a.build(&f);
        m.validate().unwrap_or_else(|e| panic!("{preset:?}: {e}"));
        let (v, e, p) = m.counts();
        assert!(p > 200 && v > p / 2 && e > p, "{preset:?}: {v} {e} {p}");
        assert!(m.polygons.values().all(|q| q.corners.len() >= 3));
        // Stays within (roughly) its base; roofs and spires may rise above.
        let b = m.bounds().unwrap();
        assert!(b.min.z >= -0.01, "{preset:?} sits on the base");
        assert!((b.max.x - b.min.x) < f.size().x * 1.6 + 6.0, "{preset:?} width");
        assert!(has(&m, ids::FACADE), "{preset:?} stone");
        assert_eq!(a.build(&f).counts(), (v, e, p), "{preset:?} deterministic");
    }
}

#[test]
fn preset_specific_details() {
    let build = |p: ArchPreset| ArchitectMod { preset: p, ..Default::default() }.build(&base_for(p));
    assert!(has(&build(ArchPreset::Watchtower), ids::ROOF), "tower spire");
    assert!(has(&build(ArchPreset::Watchtower), ids::GLASS), "tower windows / slits");
    assert!(has(&build(ArchPreset::CurtainWall), ids::IRON), "portcullis");
    let house = build(ArchPreset::TimberHouse);
    assert!(has(&house, ids::TRIM) && has(&house, ids::PLASTER) && has(&house, ids::ROOF), "timber, plaster, roof");
    assert!(has(&build(ArchPreset::Bridge), ids::ROAD), "bridge deck");
    let cath = build(ArchPreset::Cathedral);
    assert!(has(&cath, ids::GLASS) && has(&cath, ids::ROOF) && has(&cath, ids::PLASTER), "lancets, roof, vault web");
}

#[test]
fn toggles_remove_details() {
    for preset in ArchPreset::ALL {
        let f = base_for(preset);
        let full = ArchitectMod { preset, ..Default::default() }.build(&f).polygons.len();
        let bare = ArchitectMod { preset, bricks: false, crenellations: false, machicolations: false, windows: false, door: false, buttresses: false, roof: false, ..Default::default() };
        let m = bare.build(&f);
        assert!(m.polygons.len() < full, "{preset:?}: {} < {full}", m.polygons.len());
        assert!(!has(&m, ids::GLASS), "{preset:?} no windows");
    }
    let no_roof = ArchitectMod { roof: false, ..Default::default() }.build(&base_for(ArchPreset::Watchtower));
    assert!(!has(&no_roof, ids::ROOF));
}

#[test]
fn scale_aware_details_recount_instead_of_stretching() {
    let a = ArchitectMod { preset: ArchPreset::CurtainWall, ..Default::default() };
    let (small, big) = (fp(20.0, 3.0, 8.0, false), fp(40.0, 3.0, 16.0, false));
    let (ls, lb) = (a.layout(&small), a.layout(&big));
    assert_eq!(ls.brick, lb.brick, "same block size");
    assert!(lb.bricks_per_course >= ls.bricks_per_course * 2 - 1);
    assert!(lb.courses >= ls.courses * 2 - 1);
    assert!(lb.merlons > ls.merlons && lb.floors > ls.floors && lb.buttresses > ls.buttresses);
    assert!(a.build(&big).polygons.len() > a.build(&small).polygons.len() * 2);
    // Brick scale changes the block size and so the counts.
    let coarse = ArchitectMod { brick_scale: 2.0, ..a.clone() }.layout(&small);
    assert!(coarse.bricks_per_course < ls.bricks_per_course && coarse.brick.x > ls.brick.x);
    // Huge bases grow the blocks to stay inside the brick budget.
    let huge = a.layout(&fp(400.0, 3.0, 60.0, false));
    assert!(huge.brick.x > ls.brick.x);
    assert!((huge.bricks_per_course * huge.courses) as f32 <= BRICK_BUDGET * 1.2);
    // Bridges gain arches with length.
    let br = ArchitectMod { preset: ArchPreset::Bridge, ..Default::default() };
    assert!(br.layout(&fp(80.0, 5.0, 9.0, false)).arches > br.layout(&fp(30.0, 5.0, 9.0, false)).arches);
}

#[test]
fn reads_round_and_square_bases_from_the_input() {
    let cyl = Primitive::default_of(PrimitiveKind::Cylinder).to_editable_poly();
    let bx = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    assert!(Footprint::of_mesh(&cyl, PlanShape::Auto).unwrap().round);
    assert!(!Footprint::of_mesh(&bx, PlanShape::Auto).unwrap().round);
    assert!(Footprint::of_mesh(&bx, PlanShape::Round).unwrap().round);
    let mut m = cyl.clone();
    ArchitectMod::default().apply(&mut m);
    assert!(m.polygons.len() > cyl.polygons.len() * 5);
    m.validate().unwrap();
}

#[test]
fn curtain_wall_follows_a_spline_path() {
    use crate::splines::{Knot, Shape, Spline};
    let pts = [Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0), Vec3::new(30.0, 20.0, 0.0), Vec3::new(0.0, 20.0, 0.0)];
    let shape = Shape { splines: vec![Spline { knots: pts.iter().map(|&p| Knot::corner(p)).collect(), closed: true, selected: false }], ..Default::default() };
    let a = ArchitectMod { preset: ArchPreset::CurtainWall, ..Default::default() };
    let m = a.build_from_shape(&shape);
    m.validate().unwrap();
    let b = m.bounds().unwrap();
    assert!(b.max.x > 29.0 && b.max.y > 19.0 && b.min.x < 1.0, "walls along the path");
    assert!(b.max.z >= a.spline_height, "towers rise above the wall");
    assert!(has(&m, ids::IRON), "gatehouse portcullis on the longest run");
}

#[test]
fn damaged_toggle_uses_period_destruction() {
    let f = base_for(ArchPreset::TimberHouse);
    let clean = ArchitectMod { preset: ArchPreset::TimberHouse, ..Default::default() };
    let mut dmg = ArchitectMod { damaged: true, ..clean.clone() };
    dmg.damage.intensity = 90.0;
    let m = dmg.build(&f);
    m.validate().unwrap();
    assert_ne!(m.counts(), clean.build(&f).counts());
    assert!(has(&m, ids::CHARRED), "burned roof rafters");
    assert!(!has(&m, ids::REBAR), "medieval siege has no rebar");
}
