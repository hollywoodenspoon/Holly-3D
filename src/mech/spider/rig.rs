//! Spawns a Spider Platform into a scene: every part is an Editable Poly
//! object posed in world space and joined by the assembly system.

use super::leg::{self, ANKLE, HIP, KNEE};
use super::{DRUM_AT, HULL_Z, LEG_NAMES, PEDESTAL_TOP, PLATE_H, hull, limits, railgun};
use crate::assembly::mount::{Mount, joint_at, piston_at};
use crate::assembly::{Axis, Frame, JointKind, Limits};
use crate::mesh::EditablePoly;
use crate::scene::{BaseObject, ObjId, Scene, Shared};
use bevy::math::{Quat, Vec3};

/// The parts of one leg.
#[derive(Clone, Copy, Debug)]
pub struct LegIds {
    pub drum: ObjId,
    pub thigh: ObjId,
    pub shin: ObjId,
    pub foot: ObjId,
    pub carriage: ObjId,
    pub housing: ObjId,
    pub rod: ObjId,
}

/// A spawned mech: the hull, its four legs (FL, FR, RL, RR) and turret.
#[derive(Clone, Debug)]
pub struct SpiderRig {
    pub hull: ObjId,
    pub legs: Vec<LegIds>,
    pub turret: ObjId,
    pub gun: ObjId,
}

impl SpiderRig {
    /// Every part of the mech.
    pub fn parts(&self) -> Vec<ObjId> {
        let mut v = vec![self.hull, self.turret, self.gun];
        for l in &self.legs {
            v.extend([l.drum, l.thigh, l.shin, l.foot, l.carriage, l.housing, l.rod]);
        }
        v
    }
}

fn part(s: &mut Scene, name: String, mesh: EditablePoly, at: Vec3, rot: Quat) -> ObjId {
    let id = s.add(BaseObject::EditablePoly(Shared::new(mesh)), at);
    let o = &mut s.objects[id];
    o.name = name;
    o.transform.rotation = rot;
    o.evaluate();
    id
}

fn frame(s: &Scene, id: ObjId) -> Frame {
    Frame::of(&s.objects[id].transform)
}

/// Hinge (axis = `normal`) or other joint placing `child` on `parent` at `point`.
fn join(s: &mut Scene, kind: JointKind, child: ObjId, parent: ObjId, point: Vec3, normal: Vec3) {
    let m = Mount { obj: parent, point, normal };
    let j = joint_at(kind, (child, frame(s, child)), (parent, frame(s, parent)), &m);
    s.add_joint(j).expect("fresh parts cannot form a loop");
}

fn hinge(min: f32, max: f32, on: bool) -> JointKind {
    JointKind::Revolute { axis: Axis::Z, angle: 0.0, limits: Limits { on, min, max } }
}

/// Builds one leg at drum `i` on `hull` (hull pose `h`).
fn spawn_leg(s: &mut Scene, hull: ObjId, h: Frame, i: usize) -> LegIds {
    let d = DRUM_AT[i];
    let r = h.rot * Quat::from_rotation_z(d.y.signum().atan2(d.x.signum()));
    let n = LEG_NAMES[i];
    let drum_at = h.point(Vec3::new(d.x, d.y, PLATE_H));
    let hip_at = drum_at + r * HIP;
    let knee_at = hip_at + r * KNEE;
    let ankle_at = knee_at + r * ANKLE;
    let car_at = knee_at + r * leg::carriage_pivot();
    let (h_eye, r_eye) = (hip_at + r * leg::housing_eye(), car_at + r * leg::rod_eye());
    let drum = part(s, format!("Spider_{n}_HipDrum"), leg::hip_drum(), drum_at, r);
    let thigh = part(s, format!("Spider_{n}_Thigh"), leg::thigh(), hip_at, r);
    let shin = part(s, format!("Spider_{n}_Shin"), leg::shin(), knee_at, r);
    let foot = part(s, format!("Spider_{n}_Foot"), leg::foot(), ankle_at, r);
    let carriage = part(s, format!("Spider_{n}_KneeCarriage"), leg::carriage(), car_at, r);
    // Piston parts: eyelet pins along the leg's Y, bodies aimed at each other.
    let dir = (r_eye - h_eye).normalize();
    let local = r.inverse() * dir;
    let aim = |v: Vec3| r * Quat::from_rotation_y(v.x.atan2(v.z));
    let housing = part(s, format!("Spider_{n}_KneePiston"), leg::piston(false), h_eye, aim(local));
    let rod = part(s, format!("Spider_{n}_KneeRod"), leg::piston(true), r_eye, aim(-local));
    let (pin, up) = (r * Vec3::NEG_Y, h.rot * Vec3::Z);
    let sw = limits::HIP_SWING;
    join(s, hinge(-sw, sw, true), drum, hull, drum_at, up);
    join(s, hinge(limits::HIP_LIFT.0, limits::HIP_LIFT.1, true), thigh, drum, hip_at, pin);
    join(s, hinge(limits::KNEE.0, limits::KNEE.1, true), shin, thigh, knee_at, pin);
    let ball = JointKind::Ball { rot: Quat::IDENTITY, cone: limits::ANKLE_CONE, twist: limits::ANKLE_TWIST };
    join(s, ball, foot, shin, ankle_at, up);
    let c = limits::CARRIAGE;
    let slide = JointKind::Slider { axis: Axis::Z, travel: 0.0, limits: Limits { on: true, min: -c, max: c } };
    join(s, slide, carriage, shin, car_at, r * leg::shin_dir());
    join(s, hinge(-180.0, 180.0, false), housing, thigh, h_eye, pin);
    join(s, hinge(-180.0, 180.0, false), rod, carriage, r_eye, pin);
    let a = Mount { obj: housing, point: h_eye, normal: dir };
    let b = Mount { obj: rod, point: r_eye, normal: -dir };
    let pj = piston_at((housing, frame(s, housing)), (rod, frame(s, rod)), &a, &b);
    s.add_joint(pj).expect("a piston never loops");
    s.link_motion(shin, carriage, limits::CARRIAGE_RATIO).expect("knee and carriage both have 1-DOF joints");
    LegIds { drum, thigh, shin, foot, carriage, housing, rod }
}

/// Spawns the Railgun variant standing on the ground at `at`, facing -Y.
pub fn spawn_railgun(s: &mut Scene, at: Vec3) -> SpiderRig {
    let hull_at = at + Vec3::Z * HULL_Z;
    let hull = part(s, "Spider_Hull".into(), hull::hull(), hull_at, Quat::IDENTITY);
    s.assembly.grounded.push(hull);
    let h = frame(s, hull);
    let legs = (0..4).map(|i| spawn_leg(s, hull, h, i)).collect();
    let ring = hull_at + Vec3::Z * PEDESTAL_TOP;
    let turret = part(s, "Spider_RailgunTurret".into(), railgun::turret(), ring, Quat::IDENTITY);
    let gun_at = ring + railgun::TRUNNION;
    let gun = part(s, "Spider_Railgun".into(), railgun::railgun(), gun_at, Quat::IDENTITY);
    join(s, hinge(-180.0, 180.0, false), turret, hull, ring, Vec3::Z);
    join(s, hinge(limits::GUN_LIFT.0, limits::GUN_LIFT.1, true), gun, turret, gun_at, Vec3::NEG_X);
    s.solve_assembly();
    SpiderRig { hull, legs, turret, gun }
}
