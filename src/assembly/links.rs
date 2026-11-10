//! Motion Links: a driven part's hinge or slider follows a driver's
//! (driven value = offset + ratio × driver value), for gear trains, rack
//! and pinion or linked levers. The offset is taken when linking, so
//! nothing jumps.

use super::Axis;
use crate::mech::{GearType, MechPart};
use crate::scene::{ObjId, Scene, SceneObject};

/// Pitch radius of a round gear part (mesh units).
fn pitch_radius(o: &SceneObject) -> Option<f32> {
    match o.mech() {
        Some(MechPart::Gear(g)) if g.kind != GearType::Rack => Some(g.pitch_radius()),
        _ => None,
    }
}

fn is_ring(o: &SceneObject) -> bool {
    matches!(o.mech(), Some(MechPart::Gear(g)) if g.kind == GearType::Ring)
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MotionLink {
    pub driver: ObjId,
    pub driven: ObjId,
    pub ratio: f32,
    pub offset: f32,
}

impl Scene {
    /// The 1-DOF value (hinge angle or slider travel) of the joint placing `id`.
    pub fn joint_value(&self, id: ObjId) -> Option<f32> {
        let a = &self.assembly;
        a.joint_of(id).and_then(|i| a.joints[i].kind.value())
    }

    /// Links `driven`'s joint to `driver`'s. Refuses parts without a hinge
    /// or slider and loops of links; replaces `driven`'s earlier link.
    pub fn link_motion(&mut self, driver: ObjId, driven: ObjId, ratio: f32) -> Result<(), String> {
        if driver == driven {
            return Err("A part cannot drive itself".into());
        }
        let (Some(a), Some(b)) = (self.joint_value(driver), self.joint_value(driven)) else {
            return Err("Motion Link: both parts need a hinge or slider joint".into());
        };
        let links = &mut self.assembly.links;
        let mut at = driver;
        for _ in 0..=links.len() {
            let Some(l) = links.iter().find(|l| l.driven == at) else { break };
            if l.driver == driven {
                return Err("Motion Link: that would make a loop of links".into());
            }
            at = l.driver;
        }
        links.retain(|l| l.driven != driven);
        links.push(MotionLink { driver, driven, ratio, offset: b - ratio * a });
        Ok(())
    }

    /// Changes a link's ratio, keeping the driven part where it is.
    pub fn set_link_ratio(&mut self, driven: ObjId, ratio: f32) {
        let Some(i) = self.assembly.links.iter().position(|l| l.driven == driven) else { return };
        let (Some(a), Some(b)) = (self.joint_value(self.assembly.links[i].driver), self.joint_value(driven)) else { return };
        let l = &mut self.assembly.links[i];
        (l.ratio, l.offset) = (ratio, b - ratio * a);
    }

    /// Sets every driven joint from its driver; true when a value changed.
    pub(super) fn apply_links(&mut self) -> bool {
        let objs = &self.objects;
        self.assembly.links.retain(|l| objs.contains_key(l.driver) && objs.contains_key(l.driven));
        let mut changed = false;
        // Repeat so chains of links settle whatever their list order.
        for _ in 0..self.assembly.links.len() {
            let mut any = false;
            for k in 0..self.assembly.links.len() {
                let l = self.assembly.links[k].clone();
                let (Some(v), Some(i)) = (self.joint_value(l.driver), self.assembly.joint_of(l.driven)) else { continue };
                let kind = &mut self.assembly.joints[i].kind;
                let before = kind.value();
                kind.set_value(l.offset + l.ratio * v);
                any |= kind.value() != before;
            }
            changed |= any;
            if !any {
                break;
            }
        }
        changed
    }

    /// Radius of part `id` about its joint axis: a Gear's pitch radius, or
    /// the farthest vertex from the axis line. None for a slider part.
    pub fn joint_radius(&self, id: ObjId) -> Option<f32> {
        let a = &self.assembly;
        let j = &a.joints[a.joint_of(id)?];
        let super::JointKind::Revolute { axis, .. } = j.kind else { return None };
        let o = &self.objects[id];
        if let Some(r) = pitch_radius(o) {
            return Some(r * o.transform.scale.max_element());
        }
        let w = super::Frame::of(&self.objects[j.parent].transform) * j.parent_frame;
        let dir = w.rot * Axis::vec(axis);
        let m = o.world_matrix();
        let far = o.evaluated_mesh.vertices.values().map(|v| {
            let d = m.transform_point3(v.position) - w.pos;
            (d - dir * d.dot(dir)).length()
        });
        Some(far.fold(0.0, f32::max)).filter(|&r| r > 1e-6)
    }

    /// Auto-Ratio from Gear Teeth / Radius: meshing gears turn opposite
    /// ways at the inverse ratio of their radii (the same way inside an
    /// internal ring); a pinion drives a rack by its arc length (and the
    /// reverse). Gear parts use their pitch radius (module × teeth / 2).
    pub fn auto_ratio(&self, driver: ObjId, driven: ObjId) -> Option<f32> {
        let deg = std::f32::consts::PI / 180.0;
        let (ra, rb) = (self.joint_radius(driver), self.joint_radius(driven));
        self.joint_value(driver)?;
        self.joint_value(driven)?;
        Some(match (ra, rb) {
            (Some(a), Some(b)) if is_ring(&self.objects[driver]) || is_ring(&self.objects[driven]) => a / b,
            (Some(a), Some(b)) => -a / b,
            (Some(a), None) => a * deg,
            (None, Some(b)) => 1.0 / (b * deg),
            (None, None) => 1.0,
        })
    }
}
