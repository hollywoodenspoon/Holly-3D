//! J tool ("Assemble Joint"): click a mount on part A (the child), then a
//! mount on part B (the parent). A is snapped face to face onto B and a
//! joint of the chosen type links them. Hydraulic Piston joints aim the two
//! parts at each other instead of moving them. Mirror Mech Limb clones a
//! part and everything it carries across a world plane, joints included.

use super::mirror::{MirrorAxis, MirrorSettings};
use super::select::{EDGE_PICK_PX, VERTEX_PICK_PX};
use super::*;
use crate::assembly::mirror::mirror_joint;
use crate::assembly::mount::{Mount, MountAt, joint_at, mate, mount_point, piston_at};
use crate::assembly::{Frame, JointKind};
use crate::viewport::picking::pick_polygon;
use std::collections::HashMap;

impl Editor {
    /// `J` / Assemble Joint: toggles the two-click tool.
    pub fn toggle_joint_tool(&mut self) {
        self.tool = if matches!(self.tool, Tool::Joint { .. }) { Tool::None } else { Tool::Joint { first: None } };
    }

    /// The mount under the cursor: a vertex, an edge (its loop center) or a
    /// face center, on the closest geometry object.
    pub fn pick_mount(&self, view: usize, cursor: Vec2) -> Option<Mount> {
        let id = self.pick_object(view, cursor)?;
        let o = &self.scene.objects[id];
        if o.is_helper() || o.evaluated_mesh.polygons.is_empty() {
            return None;
        }
        let v = &self.viewports.views[view];
        let (m, w, ray) = (&o.evaluated_mesh, o.world_matrix(), v.ray(cursor));
        let hit = pick_polygon(m, w, &ray, false)?;
        let screen = |p: Vec3| v.to_screen(w.transform_point3(p)).map_or(f32::MAX, |s| s.distance(cursor));
        let corners: Vec<_> = m.polygons[hit.poly].corners.iter().map(|c| c.vertex_id).collect();
        let near_v = corners.iter().map(|&c| (c, screen(m.pos(c)))).min_by(|a, b| a.1.total_cmp(&b.1));
        let edge_d = |a, b| {
            let (Some(pa), Some(pb)) = (v.to_screen(w.transform_point3(m.pos(a))), v.to_screen(w.transform_point3(m.pos(b)))) else {
                return f32::MAX;
            };
            let t = ((cursor - pa).dot(pb - pa) / (pb - pa).length_squared().max(1e-6)).clamp(0.0, 1.0);
            (pa + (pb - pa) * t).distance(cursor)
        };
        let n = corners.len();
        let near_e =
            (0..n).map(|i| (corners[i], corners[(i + 1) % n])).map(|(a, b)| (a, b, edge_d(a, b))).min_by(|x, y| x.2.total_cmp(&y.2));
        let at = match (near_v, near_e) {
            (Some((vid, d)), _) if d <= VERTEX_PICK_PX => MountAt::Vertex(vid),
            (_, Some((a, b, d))) if d <= EDGE_PICK_PX => MountAt::Edge(a, b),
            _ => MountAt::Face,
        };
        let (point, normal) = mount_point(m, w, hit.poly, at, -ray.dir);
        Some(Mount { obj: id, point, normal })
    }

    pub(super) fn joint_click(&mut self, view: usize, cursor: Vec2, first: Option<Mount>) {
        let Some(m) = self.pick_mount(view, cursor) else {
            self.status = "Assemble Joint: click a point on a geometry object".into();
            return;
        };
        match first {
            None => {
                self.scene.select_only(Some(m.obj));
                self.tool = Tool::Joint { first: Some(m) };
            }
            Some(a) if a.obj == m.obj => self.status = "Assemble Joint: pick a point on a different part".into(),
            Some(a) => match self.assemble(a, m) {
                Ok(()) => self.tool = Tool::None,
                Err(e) => self.status = e,
            },
        }
    }

    /// Snaps part A onto part B and links them with the current joint type.
    pub fn assemble(&mut self, a: Mount, b: Mount) -> Result<(), String> {
        let kind = self.joint_type.rest();
        if kind.is_kinematic() && self.scene.assembly.is_ancestor(a.obj, b.obj) {
            return Err(format!(
                "{} already carries {}: pick another parent",
                self.scene.objects[a.obj].name, self.scene.objects[b.obj].name
            ));
        }
        self.checkpoint("Assemble Joint");
        let f = |ed: &Self, id| Frame::of(&ed.scene.objects[id].transform);
        let joint = if kind.is_kinematic() {
            let c = mate(f(self, a.obj), &a, &b);
            c.write(&mut self.scene.objects[a.obj].transform);
            joint_at(kind, (a.obj, c), (b.obj, f(self, b.obj)), &b)
        } else {
            piston_at((a.obj, f(self, a.obj)), (b.obj, f(self, b.obj)), &a, &b)
        };
        if let Err(e) = self.scene.add_joint(joint) {
            self.history.discard_last();
            return Err(e);
        }
        self.scene.select_only(Some(a.obj));
        let (ca, cb) = (&self.scene.objects[a.obj].name, &self.scene.objects[b.obj].name);
        self.status = format!("{} joint: {ca} -> {cb}", self.joint_type.label());
        Ok(())
    }

    /// Edits the joint placing the current object (one undo step per gesture).
    pub fn edit_joint(&mut self, label: &str, started: bool, f: impl FnOnce(&mut JointKind)) {
        let Some(i) = self.current().and_then(|id| self.scene.assembly.joint_of(id)) else { return };
        if started {
            self.checkpoint(label);
        }
        f(&mut self.scene.assembly.joints[i].kind);
        self.scene.solve_assembly();
    }

    /// Switches the current part's joint to another type; the part keeps
    /// as much of its pose as the new joint allows.
    pub fn change_joint_type(&mut self, ty: crate::assembly::JointType) {
        let Some(i) = self.current().and_then(|id| self.scene.assembly.joint_of(id)) else { return };
        self.checkpoint("Joint Type");
        let j = &mut self.scene.assembly.joints[i];
        j.kind = ty.rest();
        j.posed = None;
        self.scene.solve_assembly();
    }

    /// Removes every joint the current object takes part in.
    pub fn break_joints(&mut self) {
        let Some(id) = self.current() else { return };
        if !self.scene.assembly.joints.iter().any(|j| j.child == id || j.parent == id) {
            return;
        }
        self.checkpoint("Break Joint");
        self.scene.assembly.joints.retain(|j| j.child != id && j.parent != id);
        self.status = format!("{}: joints removed", self.scene.objects[id].name);
    }

    pub fn set_ik_drag(&mut self, on: bool) {
        self.checkpoint("IK Drag");
        self.scene.assembly.no_ik = !on;
    }

    /// Grounded parts end IK chains: they never move for an IK drag.
    pub fn set_grounded(&mut self, id: ObjId, on: bool) {
        self.checkpoint("Grounded");
        let g = &mut self.scene.assembly.grounded;
        g.retain(|&x| x != id);
        if on {
            g.push(id);
        }
    }

    /// Mirror Mech Limb: copies the current part and every part it carries
    /// across the world `axis` plane through its parent's pivot (or the
    /// world origin for a root part), with mirrored joints.
    pub fn mirror_limb(&mut self, axis: MirrorAxis) {
        let Some(root) = self.current() else { return };
        let a = &self.scene.assembly;
        let center = a.parent_of(root).map_or(Vec3::ZERO, |p| self.scene.objects[p].transform.translation);
        let parts: Vec<ObjId> = std::iter::once(root).chain(a.descendants(root)).collect();
        self.checkpoint("Mirror Mech Limb");
        let refl = MirrorSettings { axis, offset: 0.0, clone: None }.matrix(center);
        let mut map = HashMap::new();
        for &id in &parts {
            let xf = Xform::from_matrix(refl * self.scene.objects[id].transform.matrix());
            if let Some(n) = self.scene.clone_object(id, CloneKind::Copy, xf) {
                map.insert(id, n);
            }
        }
        let f = |s: &Scene, id| Frame::of(&s.objects[id].transform);
        let joints: Vec<_> = (self.scene.assembly.joints.iter())
            .filter(|j| map.contains_key(&j.child) && (j.kind.is_kinematic() || map.contains_key(&j.parent)))
            .cloned()
            .collect();
        for j in joints {
            let (c, p) = (map[&j.child], map.get(&j.parent).copied().unwrap_or(j.parent));
            let (old, new) = ((f(&self.scene, j.child), f(&self.scene, j.parent)), (f(&self.scene, c), f(&self.scene, p)));
            let mut m = mirror_joint(&j, refl, old, new);
            (m.child, m.parent) = (c, p);
            // Cannot loop: the copies are new parts.
            let _ = self.scene.add_joint(m);
        }
        let Some(&copy) = map.get(&root) else { return };
        self.scene.select_only(Some(copy));
        self.status = format!("Mirror Mech Limb: {} part(s) mirrored across {}", map.len(), axis.label());
    }
}
