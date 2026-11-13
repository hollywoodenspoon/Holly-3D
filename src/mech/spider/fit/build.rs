//! Turns the classified, fitted parts into a live Spider rig: every
//! imported object is re-pivoted in place onto its joint (its stack is
//! collapsed into the new Editable Poly), gets its role's Material ID and
//! name, and the carriage + piston parts are added.

use super::classify::{Part, Roles};
use super::geom::flat;
use super::pins::{LegFit, fit_gun, fit_turret};
use super::strut::{link_ratio, support};
use crate::assembly::JointKind;
use crate::assembly::mount::{Mount, piston_at};
use crate::assembly::{Axis, Limits};
use crate::mech::spider::kit::mat;
use crate::mech::spider::rig::{frame, hinge, join, part};
use crate::mech::spider::{LEG_NAMES, LegIds, SpiderRig, limits};
use crate::mesh::EditablePoly;
use crate::scene::{BaseObject, ObjId, Scene, Shared, Xform};
use bevy::math::{Mat4, Quat, Vec3};

/// `world` relative to a pivot at `at` turned by `rot`.
fn local(world: &EditablePoly, at: Vec3, rot: Quat, id: Option<u16>) -> EditablePoly {
    let mut m = world.clone();
    m.transform(Mat4::from_rotation_translation(rot, at).inverse());
    if let Some(id) = id {
        for p in m.polygons.values_mut() {
            p.material_id = id;
        }
    }
    m
}

/// Moves object `id`'s pivot to `at` / `rot` without moving its geometry.
fn repivot(s: &mut Scene, id: ObjId, world: &EditablePoly, at: Vec3, rot: Quat, mat_id: u16, name: String) {
    let o = &mut s.objects[id];
    o.base = BaseObject::EditablePoly(Shared::new(local(world, at, rot, Some(mat_id))));
    o.modifiers.clear();
    o.active_level = None;
    o.offset = Mat4::IDENTITY;
    o.transform = Xform { translation: at, rotation: rot, scale: Vec3::ONE };
    o.name = name;
    o.evaluate();
}

/// Leg order (FL, FR, RL, RR) from where each foot is, given the front.
fn leg_slots(parts: &[Part], roles: &Roles, hub: Vec3, front: Vec3) -> Vec<usize> {
    let left = Vec3::Z.cross(front);
    let mut slot: Vec<(usize, usize)> = roles
        .legs
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let d = parts[l.foot].bb.center() - hub;
            (usize::from(d.dot(front) < 0.0) * 2 + usize::from(d.dot(left) < 0.0), i)
        })
        .collect();
    slot.sort();
    let distinct = slot.windows(2).all(|w| w[0].0 != w[1].0);
    if distinct { slot.into_iter().map(|(_, i)| i).collect() } else { (0..roles.legs.len()).collect() }
}

pub struct Built {
    pub rig: SpiderRig,
    /// Strut polygons removed per leg.
    pub cut: Vec<usize>,
}

/// Re-pivots `ids` (in `parts` order) and joins them into a Spider rig.
pub fn build(s: &mut Scene, ids: &[ObjId], mut parts: Vec<Part>, roles: &Roles, fits: &[LegFit]) -> Built {
    let hub = parts[roles.hull].bb.center();
    let (trunnion, lift_axis, e0) = fit_gun(&parts[roles.gun], &parts[roles.turret]);
    let ring_at = fit_turret(&parts, roles.turret, roles.gun);
    let front = flat(parts[roles.gun].bb.center() - ring_at);
    let hull = ids[roles.hull];
    repivot(s, hull, &parts[roles.hull].mesh, hub, Quat::IDENTITY, mat::HULL, "Spider_Hull".into());
    s.assembly.grounded.push(hull);
    let mut legs = vec![];
    let mut cut = vec![];
    for (slot, li) in leg_slots(&parts, roles, hub, front).into_iter().enumerate() {
        let (l, f) = (roles.legs[li], fits[li]);
        let n = LEG_NAMES.get(slot).copied().unwrap_or("Leg");
        let sup = support(&mut parts[l.shin].mesh, &f);
        cut.push(sup.cut);
        let r = f.r;
        let named = |what: &str| format!("Spider_{n}_{what}");
        repivot(s, ids[l.ring], &parts[l.ring].mesh, f.swing_at, r, mat::JOINT, named("HipRing"));
        repivot(s, ids[l.thigh], &parts[l.thigh].mesh, f.lift_at, r, mat::LEG, named("Thigh"));
        repivot(s, ids[l.shin], &parts[l.shin].mesh, f.knee_at, r, mat::LEG, named("Shin"));
        repivot(s, ids[l.foot], &parts[l.foot].mesh, f.ankle_at, r, mat::FOOT, named("Foot"));
        let carriage = part(s, named("KneeCarriage"), local(&sup.carriage, sup.carriage_at, r, None), sup.carriage_at, r);
        let housing = part(s, named("KneePiston"), local(&sup.housing, sup.housing_eye, sup.housing_rot, None), sup.housing_eye, sup.housing_rot);
        let rod = part(s, named("KneeRod"), local(&sup.rod, sup.rod_eye, sup.rod_rot, None), sup.rod_eye, sup.rod_rot);
        let (ring, thigh, shin, foot) = (ids[l.ring], ids[l.thigh], ids[l.shin], ids[l.foot]);
        let sw = limits::HIP_SWING;
        join(s, hinge(-sw, sw, true), ring, hull, f.swing_at, Vec3::Z);
        join(s, hinge(limits::HIP_LIFT.0, limits::HIP_LIFT.1, true), thigh, ring, f.lift_at, f.lift_axis);
        join(s, hinge(limits::KNEE.0, limits::KNEE.1, true), shin, thigh, f.knee_at, f.knee_axis);
        let ball = JointKind::Ball { rot: Quat::IDENTITY, cone: limits::ANKLE_CONE, twist: limits::ANKLE_TWIST };
        join(s, ball, foot, shin, f.ankle_at, Vec3::Z);
        let len = f.shin_len();
        let (lo, hi) = ((-0.15 * len).max(0.1 * len - sup.along), (0.15 * len).min(0.92 * len - sup.along));
        let slide = JointKind::Slider { axis: Axis::Z, travel: 0.0, limits: Limits { on: true, min: lo.min(0.0), max: hi.max(0.0) } };
        join(s, slide, carriage, shin, sup.carriage_at, f.shin_dir());
        join(s, hinge(-180.0, 180.0, false), housing, thigh, sup.housing_eye, f.knee_axis);
        join(s, hinge(-180.0, 180.0, false), rod, carriage, sup.rod_eye, f.knee_axis);
        let dir = sup.rod_eye - sup.housing_eye;
        let a = Mount { obj: housing, point: sup.housing_eye, normal: dir };
        let b = Mount { obj: rod, point: sup.rod_eye, normal: -dir };
        let pj = piston_at((housing, frame(s, housing)), (rod, frame(s, rod)), &a, &b);
        s.add_joint(pj).expect("a piston never loops");
        s.link_motion(shin, carriage, link_ratio(&f, &sup)).expect("knee and carriage both have 1-DOF joints");
        legs.push(LegIds { drum: ring, thigh, shin, foot, carriage, housing, rod });
    }
    let (turret, gun) = (ids[roles.turret], ids[roles.gun]);
    repivot(s, turret, &parts[roles.turret].mesh, ring_at, Quat::IDENTITY, mat::ARMOUR, "Spider_RailgunTurret".into());
    repivot(s, gun, &parts[roles.gun].mesh, trunnion, Quat::IDENTITY, mat::WEAPON, "Spider_Railgun".into());
    join(s, hinge(-180.0, 180.0, false), turret, hull, ring_at, Vec3::Z);
    let (gl, gh) = limits::GUN_LIFT;
    join(s, hinge(gl - e0, gh - e0, true), gun, turret, trunnion, lift_axis);
    for &(i, p) in &roles.welded {
        let c = parts[i].bb.center();
        let id = if p == roles.turret { mat::ARMOUR } else { mat::HULL };
        let name = s.objects[ids[i]].name.clone();
        repivot(s, ids[i], &parts[i].mesh, c, Quat::IDENTITY, id, name);
        join(s, JointKind::Rigid, ids[i], ids[p], c, Vec3::Z);
    }
    s.solve_assembly();
    Built { rig: SpiderRig { hull, legs, turret, gun }, cut }
}
