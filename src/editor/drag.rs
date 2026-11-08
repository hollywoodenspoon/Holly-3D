//! Gizmo drag sessions: object transforms, Shift+Drag clone, sub-object
//! transforms (with Shift+Drag edge extrusion), pivot and slice plane.

use super::*;
use crate::mesh::VertexId;
use bevy::math::Mat4;

#[derive(Clone, Debug)]
pub enum DragTarget {
    Objects { starts: Vec<(ObjId, Xform)>, clone: bool },
    SubObject { object: ObjId, before: Box<SceneObject>, verts: Vec<VertexId>, soft: Option<Vec<(VertexId, f32)>> },
    Pivot { object: ObjId, before: Box<SceneObject> },
    SlicePlane { start: Xform },
    ModSub { object: ObjId, before: Box<SceneObject>, sub: crate::modifiers::ModSub },
}

#[derive(Clone, Debug)]
pub struct Drag {
    pub target: DragTarget,
    /// Gizmo frame at drag start (world).
    pub start: Mat4,
    pub last_total: Mat4,
}

impl Editor {
    /// Starts a drag; `frame` is the gizmo's world frame at the start.
    pub fn begin_drag(&mut self, frame: Mat4, shift: bool) {
        if let (Some(sub), Some(id)) = (self.active_mod_sub(), self.current()) {
            self.checkpoint(sub.label());
            let before = Box::new(self.scene.objects[id].clone());
            self.drag = Some(Drag { target: DragTarget::ModSub { object: id, before, sub }, start: frame, last_total: Mat4::IDENTITY });
            return;
        }
        if let (Some(sp), true) = (&self.slice_plane, self.in_sub_object()) {
            let start = sp.xform;
            self.drag = Some(Drag { target: DragTarget::SlicePlane { start }, start: frame, last_total: Mat4::IDENTITY });
            return;
        }
        if self.in_sub_object() {
            let Some(id) = self.current() else { return };
            self.checkpoint("Transform");
            let edge_like = self.sub.is_edge_like();
            if shift && edge_like && self.mode == TransformMode::Move {
                // 3ds Max: Shift+Drag open edges extrudes new polygons from them.
                let o = &mut self.scene.objects[id];
                if o.apply_op(PolyOp::ShiftExtrudeEdges) {
                    self.status = "Shift+Drag: extruding border edges".into();
                }
            }
            let o = &self.scene.objects[id];
            let verts = o.level_mesh.selection_vertices(self.sub);
            let soft = self.soft.enabled.then(|| {
                let s = &self.soft;
                let w = crate::mesh::ops::geometry::soft_weights(&o.level_mesh, &verts, s.falloff, s.pinch, s.bubble);
                w.into_iter().collect::<Vec<_>>()
            });
            self.drag = Some(Drag {
                target: DragTarget::SubObject { object: id, before: Box::new(o.clone()), verts, soft },
                start: frame,
                last_total: Mat4::IDENTITY,
            });
            return;
        }
        let ids = self.scene.selected();
        if ids.is_empty() {
            return;
        }
        if self.affect_pivot_only {
            self.checkpoint("Move Pivot");
            let before = Box::new(self.scene.objects[ids[0]].clone());
            self.drag = Some(Drag { target: DragTarget::Pivot { object: ids[0], before }, start: frame, last_total: Mat4::IDENTITY });
            return;
        }
        if !shift {
            self.checkpoint("Transform");
        }
        let starts = ids.iter().map(|&k| (k, self.scene.objects[k].transform)).collect();
        self.drag = Some(Drag { target: DragTarget::Objects { starts, clone: shift }, start: frame, last_total: Mat4::IDENTITY });
    }

    /// Updates object transforms directly from per-object gizmo results.
    pub fn drag_objects(&mut self, new: &[(ObjId, Xform)]) {
        for &(id, xf) in new {
            if let Some(o) = self.scene.objects.get_mut(id) {
                o.transform = xf;
            }
        }
        if let Some(Drag { target: DragTarget::Objects { starts, .. }, last_total, .. }) = &mut self.drag
            && let (Some(s), Some(n)) = (starts.first(), new.first())
        {
            *last_total = n.1.matrix() * s.1.matrix().inverse();
        }
    }

    /// Updates a sub-object / pivot / slice-plane drag from the gizmo frame.
    pub fn drag_frame(&mut self, frame: Mat4) {
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let total = frame * drag.start.inverse();
        drag.last_total = total;
        match drag.target.clone() {
            DragTarget::SubObject { object, before, verts, soft } => {
                let world = before.world_matrix();
                let local = world.inverse() * total * world;
                if let Some(o) = self.scene.objects.get_mut(object) {
                    *o = (*before).clone();
                    let op = match soft {
                        Some(weights) => PolyOp::SoftTransform { weights, matrix: local },
                        None => PolyOp::Transform { verts, matrix: local },
                    };
                    o.apply_op(op);
                }
                self.scene.propagate_from(object);
            }
            DragTarget::Pivot { object, before } => {
                if let Some(o) = self.scene.objects.get_mut(object) {
                    *o = (*before).clone();
                    o.move_pivot(total.to_scale_rotation_translation().2);
                }
            }
            DragTarget::SlicePlane { start } => {
                if let Some(sp) = self.slice_plane.as_mut() {
                    sp.xform = Xform::from_matrix(total * start.matrix());
                }
            }
            DragTarget::ModSub { object, before, sub } => self.drag_mod_sub(object, &before, sub, total),
            DragTarget::Objects { .. } => {}
        }
    }

    pub fn end_drag(&mut self) {
        let Some(drag) = self.drag.take() else { return };
        if let DragTarget::Objects { starts, clone: true } = drag.target {
            // Shift+Drag in Object mode: put originals back, ask for clone options.
            for (id, xf) in &starts {
                if let Some(o) = self.scene.objects.get_mut(*id) {
                    o.transform = *xf;
                }
            }
            self.clone_dialog = Some(CloneDialog { sources: starts, delta: drag.last_total, kind: CloneKind::Copy, copies: 1 });
        }
    }

    /// Clone Options [OK]: creates N clones stepping by the drag delta.
    pub fn confirm_clone(&mut self) {
        let Some(d) = self.clone_dialog.take() else {
            return;
        };
        self.checkpoint("Clone");
        let mut created = vec![];
        for (id, start) in &d.sources {
            let mut m = start.matrix();
            for _ in 0..d.copies.max(1) {
                m = d.delta * m;
                if let Some(n) = self.scene.clone_object(*id, d.kind, Xform::from_matrix(m)) {
                    created.push(n);
                }
            }
        }
        self.scene.select_only(None);
        for n in &created {
            self.scene.objects[*n].selected = true;
        }
        self.status = format!("Created {} {} clone(s)", created.len(), d.kind.label());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::test_support::top_selected_box;

    #[test]
    fn shift_drag_object_opens_clone_dialog_and_clones() {
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        ed.begin_drag(Mat4::IDENTITY, true);
        let id = ed.current().unwrap();
        ed.drag_objects(&[(id, Xform { translation: Vec3::X * 2.0, ..Default::default() })]);
        ed.end_drag();
        assert_eq!(ed.scene.objects[id].transform.translation, Vec3::ZERO);
        ed.clone_dialog.as_mut().unwrap().copies = 3;
        ed.confirm_clone();
        assert_eq!(ed.scene.objects.len(), 4);
        let mut xs: Vec<f32> = ed.scene.objects.values().map(|o| o.transform.translation.x).collect();
        xs.sort_by(f32::total_cmp);
        assert_eq!(xs, vec![0.0, 2.0, 4.0, 6.0]);
    }

    #[test]
    fn sub_object_move_and_undo() {
        let mut ed = Editor { scene: top_selected_box(), sub: SubObject::Polygon, ..Default::default() };
        ed.begin_drag(Mat4::IDENTITY, false);
        ed.drag_frame(Mat4::from_translation(Vec3::Z));
        ed.drag_frame(Mat4::from_translation(Vec3::Z * 0.5));
        ed.end_drag();
        let o = ed.current_obj().unwrap();
        let maxz = o.evaluated_mesh.vertices.values().map(|v| v.position.z).fold(0.0, f32::max);
        assert!((maxz - 1.5).abs() < 1e-5);
        ed.undo();
        let o = ed.current_obj().unwrap();
        let maxz = o.evaluated_mesh.vertices.values().map(|v| v.position.z).fold(0.0, f32::max);
        assert!((maxz - 1.0).abs() < 1e-5);
    }

    #[test]
    fn shift_drag_border_edges_extrudes() {
        let mut ed = Editor { scene: top_selected_box(), sub: SubObject::Polygon, ..Default::default() };
        let id = ed.current().unwrap();
        ed.apply_op(PolyOp::Delete { level: SubObject::Polygon });
        ed.sub = SubObject::Border;
        ed.edit_selection(|m, _| {
            m.select_all(SubObject::Edge, true);
            m.select_borders();
        });
        ed.begin_drag(Mat4::IDENTITY, true);
        ed.drag_frame(Mat4::from_translation(Vec3::Z * 0.5));
        ed.end_drag();
        assert_eq!(ed.scene.objects[id].evaluated_mesh.counts(), (12, 20, 9));
    }
}
