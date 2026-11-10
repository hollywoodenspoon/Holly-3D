//! Contact Sets: overlapping Contact Member parts push each other along
//! their hinge or slider axes. The part being dragged (selected) pushes;
//! a part that cannot give way (no hinge or slider, or at a stop) blocks.

use super::Frame;
use super::solve::child_pose;
use crate::mesh::Aabb;
use crate::scene::{ObjId, Scene, SceneObject};
use bevy::math::{Mat4, Vec3};

/// Overlap below this is touching (mated faces), not a collision.
const GAP: f32 = 1e-3;
const STEPS: usize = 90;

/// World bounds of `o` if its pivot had the rigid pose `pose`.
fn world_box(o: &SceneObject, pose: Frame) -> Option<Aabb> {
    let b = o.evaluated_mesh.bounds()?;
    let m = Mat4::from_scale_rotation_translation(o.transform.scale, pose.rot, pose.pos) * o.offset;
    let corners = (0..8).map(|i| Vec3::new([b.min.x, b.max.x][i & 1], [b.min.y, b.max.y][(i >> 1) & 1], [b.min.z, b.max.z][i >> 2]));
    Aabb::from_points(corners.map(|c| m.transform_point3(c)))
}

pub fn overlap(a: Aabb, b: Aabb) -> bool {
    (a.min + GAP).cmplt(b.max).all() && (b.min + GAP).cmplt(a.max).all()
}

impl Scene {
    fn current_box(&self, id: ObjId) -> Option<Aabb> {
        world_box(&self.objects[id], Frame::of(&self.objects[id].transform))
    }

    /// Moves `id` along its own hinge or slider (the smallest change) until
    /// it clears `other`; false when its joint cannot.
    fn push_clear(&mut self, id: ObjId, other: Aabb) -> bool {
        let a = &self.assembly;
        let Some(i) = a.joint_of(id) else { return false };
        let Some(v0) = a.joints[i].kind.value() else { return false };
        let p = Frame::of(&self.objects[a.joints[i].parent].transform);
        let clear = |s: &Self, v: f32| {
            let mut j = s.assembly.joints[i].clone();
            j.kind.set_value(v);
            // Blocked by a stop.
            j.kind.value() == Some(v) && world_box(&s.objects[id], child_pose(&j, p)).is_some_and(|b| !overlap(b, other))
        };
        let step = match a.joints[i].kind {
            super::JointKind::Revolute { .. } => 2.0,
            _ => self.current_box(id).map_or(0.02, |b| (b.size().max_element() + (other.max - other.min).max_element()) / 50.0),
        };
        for n in 1..=STEPS {
            for dir in [1.0, -1.0] {
                let v = v0 + dir * n as f32 * step;
                if !clear(self, v) {
                    continue;
                }
                // Snug: bisect between the last blocked and the first clear value.
                let (mut lo, mut hi) = (v - dir * step, v);
                for _ in 0..12 {
                    let mid = (lo + hi) * 0.5;
                    if clear(self, mid) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                self.assembly.joints[i].kind.set_value(hi);
                return true;
            }
        }
        false
    }

    /// Resolves overlaps between Contact Members; true when a joint moved.
    pub(super) fn push_contacts(&mut self) -> bool {
        let objs = &self.objects;
        self.assembly.contact_members.retain(|&m| objs.contains_key(m));
        let members: Vec<ObjId> = self.assembly.contact_members.iter().copied().filter(|&m| !objs[m].hidden).collect();
        let mut moved = false;
        for x in 0..members.len() {
            for &b in &members[x + 1..] {
                let a = members[x];
                // Parts jointed to each other are mated by design.
                if self.assembly.parent_of(a) == Some(b) || self.assembly.parent_of(b) == Some(a) {
                    continue;
                }
                let (Some(ba), Some(bb)) = (self.current_box(a), self.current_box(b)) else { continue };
                if !overlap(ba, bb) {
                    continue;
                }
                // The unselected part gives way first.
                let (first, second) =
                    if self.objects[b].selected && !self.objects[a].selected { ((a, bb), (b, ba)) } else { ((b, ba), (a, bb)) };
                moved |= self.push_clear(first.0, first.1) || self.push_clear(second.0, second.1);
            }
        }
        moved
    }
}
