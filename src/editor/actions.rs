//! High-level editor actions (each records an undo checkpoint).

use super::*;
use crate::mesh::EditablePoly;
use crate::modifiers::make_modifier;
use crate::primitives::Primitive;
use crate::scene::BaseObject;

impl Editor {
    pub fn create_primitive(&mut self, kind: PrimitiveKind, at: Vec3) {
        self.checkpoint("Create");
        let id = self.scene.add(BaseObject::Primitive(Primitive::default_of(kind)), at);
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {}", self.scene.objects[id].name);
    }

    pub fn create_geogen(&mut self, mode: crate::modifiers::GeoMode, at: Vec3) {
        self.checkpoint("Create GeoGen");
        let id = self.scene.add(BaseObject::GeoGen(crate::modifiers::GeoGenParams::default_of(mode)), at);
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {}", self.scene.objects[id].name);
    }

    pub fn convert_to_editable_poly(&mut self) {
        let ids = self.scene.selected();
        if ids.is_empty() {
            return;
        }
        self.checkpoint("Convert to Editable Poly");
        for id in ids {
            self.scene.make_unique(id);
            self.scene.objects[id].collapse_to_editable_poly();
        }
        self.tab = CommandTab::Modify;
        self.status = "Converted to Editable Poly".into();
    }

    pub fn add_modifier(&mut self, name: &str) {
        let Some(id) = self.current() else { return };
        let Some(md) = make_modifier(name) else {
            self.status = format!("{name} arrives in a later phase");
            return;
        };
        self.checkpoint("Add Modifier");
        self.scene.objects[id].add_modifier(md);
        self.scene.propagate_from(id);
        self.mod_sub = None;
        self.sub = SubObject::Object;
        self.status = format!("Added {name}");
    }

    pub fn delete_modifier(&mut self) {
        let Some(id) = self.current() else { return };
        if self.scene.objects[id].active_level.is_none() {
            return;
        }
        self.checkpoint("Delete Modifier");
        self.scene.objects[id].delete_active_modifier();
        self.scene.propagate_from(id);
        self.sub = SubObject::Object;
        self.mod_sub = None;
    }

    pub fn collapse_stack(&mut self) {
        self.convert_to_editable_poly();
        self.sub = SubObject::Object;
    }

    /// Delete key: objects in Object mode, sub-objects otherwise.
    pub fn delete_selection(&mut self) {
        if self.in_sub_object() {
            self.apply_op(PolyOp::Delete { level: self.sub });
            return;
        }
        if let Some(level) = self.spline_level() {
            self.edit_shape("Delete", |s| s.delete_selected(level));
            return;
        }
        let ids = self.scene.selected();
        if ids.is_empty() {
            return;
        }
        self.checkpoint("Delete");
        for id in ids {
            self.scene.delete(id);
        }
        self.status = "Deleted".into();
    }

    /// Runs an Edit Poly op on the current object (with undo + Repeat Last).
    pub fn apply_op(&mut self, op: PolyOp) -> bool {
        let Some(id) = self.current() else {
            return false;
        };
        if !self.scene.objects[id].sub_object_editable() {
            self.status = "Convert to Editable Poly or add an Edit Poly modifier first".into();
            return false;
        }
        self.checkpoint(op.name());
        let ok = self.scene.objects[id].apply_op(op.clone());
        if ok {
            self.scene.propagate_from(id);
            if op.repeatable() {
                self.last_op = Some(op.clone());
            }
            self.status = format!("{} applied", op.name());
        } else {
            self.history.discard_last();
            self.status = format!("{}: nothing to do for this selection", op.name());
        }
        ok
    }

    pub fn repeat_last(&mut self) {
        if let Some(op) = self.last_op.clone() {
            self.apply_op(op);
        }
    }

    /// Number keys 1–5; pressing the active one again returns to Object level.
    pub fn set_sub_object(&mut self, s: SubObject) {
        let gizmo_subs = self.current_obj().and_then(|o| o.active_modifier()).map(|k| k.gizmo_subs()).unwrap_or(&[]);
        if !gizmo_subs.is_empty() {
            // FFD: 1 = Control Points, 2 = Lattice; UVW Map: 1 = Gizmo
            match (s.number() as usize).checked_sub(1).and_then(|i| gizmo_subs.get(i)) {
                Some(&m) => self.set_mod_sub(Some(m)),
                None if s == SubObject::Object => self.mod_sub = None,
                None => {}
            }
            return;
        }
        self.mod_sub = None;
        if self.editable_spline_selected() && matches!(s, SubObject::Vertex | SubObject::Edge | SubObject::Border | SubObject::Object) {
            // Editable Spline: 1 Vertex, 2 Segment, 3 Spline
            self.sub = if self.sub == s { SubObject::Object } else { s };
            self.tool = Tool::None;
            self.tab = CommandTab::Modify;
            return;
        }
        if !self.current_obj().is_some_and(|o| o.sub_object_editable()) {
            self.sub = SubObject::Object;
            if s != SubObject::Object {
                self.status = "Sub-object modes need an Editable Poly or Edit Poly modifier".into();
            }
            return;
        }
        self.sub = if self.sub == s { SubObject::Object } else { s };
        self.tool = Tool::None;
        self.tab = CommandTab::Modify;
    }

    /// Edits the current level's selection flags and commits them.
    pub fn edit_selection(&mut self, f: impl FnOnce(&mut EditablePoly, SubObject)) {
        let sub = self.sub;
        if let Some(o) = self.current_obj_mut() {
            f(&mut o.level_mesh, sub);
            o.commit_selection();
        }
    }

    pub fn select_all(&mut self, on: bool) {
        if self.in_sub_object() {
            self.edit_selection(|m, s| m.select_all(s, on));
        } else {
            let filter_hidden: Vec<_> = self.scene.objects.iter().map(|(k, o)| (k, !o.hidden && !o.frozen)).collect();
            for (k, ok) in filter_hidden {
                self.scene.objects[k].selected = on && ok;
            }
        }
    }

    pub fn invert_selection(&mut self) {
        if self.in_sub_object() {
            self.edit_selection(|m, s| m.invert_selection(s));
        } else {
            for o in self.scene.objects.values_mut() {
                o.selected = !o.selected && !o.hidden;
            }
        }
    }

    /// Detach selected polygons to a new object (or as clone).
    pub fn detach(&mut self, as_clone: bool) {
        let Some(id) = self.current() else { return };
        if self.scene.objects[id].level_mesh.selected_polygons().is_empty() {
            self.status = "Detach: select polygons first".into();
            return;
        }
        self.checkpoint("Detach");
        let mut lvl = (*self.scene.objects[id].level_mesh).clone();
        let part = crate::mesh::ops::geometry::detach(&mut lvl, true);
        if !as_clone {
            self.scene.objects[id].apply_op(PolyOp::Delete { level: SubObject::Polygon });
        }
        let src = &self.scene.objects[id];
        let (xf, off) = (src.transform, src.offset);
        let nid = self.scene.add(BaseObject::EditablePoly(part.into()), Vec3::ZERO);
        let o = &mut self.scene.objects[nid];
        o.transform = xf;
        o.offset = off;
        o.evaluate();
        self.status = format!("Detached to {}", self.scene.objects[nid].name);
    }

    /// Attach: merges `other` into the current object.
    pub fn attach(&mut self, other: ObjId) {
        let Some(id) = self.current() else { return };
        if id == other || !self.scene.objects[id].sub_object_editable() {
            return;
        }
        let rel = self.scene.objects[id].world_to_mesh() * self.scene.objects[other].world_matrix();
        let mut mesh = (*self.scene.objects[other].evaluated_mesh).clone();
        mesh.clear_selection();
        mesh.transform(rel);
        self.checkpoint("Attach");
        if self.scene.objects[id].apply_op(PolyOp::Attach { mesh: Box::new(mesh) }) {
            let name = self.scene.objects[other].name.clone();
            self.scene.delete(other);
            self.status = format!("Attached {name}");
        } else {
            self.history.discard_last();
        }
    }

    /// Edit > Clone (in place, as a copy).
    pub fn clone_selected(&mut self, kind: CloneKind) {
        let ids = self.scene.selected();
        if ids.is_empty() {
            return;
        }
        self.checkpoint("Clone");
        let mut new = vec![];
        for id in ids {
            let xf = self.scene.objects[id].transform;
            if let Some(n) = self.scene.clone_object(id, kind, xf) {
                new.push(n);
            }
        }
        self.scene.select_only(None);
        for n in new {
            self.scene.objects[n].selected = true;
        }
    }

    /// Z: frames the selection in the active viewport (all views with `all`).
    pub fn zoom_extents(&mut self, all: bool) {
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let active = self.viewports.active;
        for i in 0..self.viewports.views.len() {
            if all || i == active {
                self.viewports.views[i].camera.frame(b.min, b.max);
                self.push_view_to_camera(i);
            }
        }
    }

    /// Opens the floating caddy for `op` on the current object (live preview).
    pub fn open_caddy(&mut self, op: PolyOp) {
        self.commit_caddy();
        let Some(id) = self.current() else { return };
        if !self.scene.objects[id].sub_object_editable() {
            self.status = "Convert to Editable Poly first".into();
            return;
        }
        self.checkpoint(op.name());
        let before = self.scene.objects[id].clone();
        self.caddy = Some(Caddy { op, object: id, before, previewed: None });
        self.update_caddy_preview();
    }

    /// Re-applies the caddy op from the pre-op snapshot whenever its params change.
    pub fn update_caddy_preview(&mut self) {
        let Some(c) = self.caddy.as_mut() else { return };
        let key = format!("{:?}", c.op);
        if c.previewed.as_deref() == Some(key.as_str()) {
            return;
        }
        c.previewed = Some(key);
        let (id, before, op) = (c.object, c.before.clone(), c.op.clone());
        if let Some(o) = self.scene.objects.get_mut(id) {
            *o = before;
            o.apply_op(op);
            self.scene.propagate_from(id);
        }
    }

    pub fn commit_caddy(&mut self) {
        if let Some(c) = self.caddy.take() {
            self.op_memory.insert(op_key(&c.op), c.op.clone());
            if c.op.repeatable() {
                self.last_op = Some(c.op.clone());
            }
            self.status = format!("{} applied", c.op.name());
        }
    }

    /// `[ + Apply & Continue ]`: keep the result and start a new preview from it.
    pub fn apply_and_continue(&mut self) {
        if let Some(c) = self.caddy.clone() {
            self.commit_caddy();
            self.open_caddy(c.op);
        }
    }

    pub fn cancel_caddy(&mut self) {
        if let Some(c) = self.caddy.take() {
            if let Some(o) = self.scene.objects.get_mut(c.object) {
                *o = c.before;
                o.version = crate::scene::next_version();
            }
            self.scene.propagate_from(c.object);
            self.history.discard_last();
            self.status = "Cancelled".into();
        }
    }

    pub fn toggle_isolate(&mut self) {
        if let Some(prev) = self.isolate.take() {
            for id in prev {
                if let Some(o) = self.scene.objects.get_mut(id) {
                    o.hidden = false;
                }
            }
            return;
        }
        let hidden: Vec<ObjId> = self.scene.objects.iter().filter(|(_, o)| !o.selected && !o.hidden).map(|(k, _)| k).collect();
        for &id in &hidden {
            self.scene.objects[id].hidden = true;
        }
        self.isolate = Some(hidden);
        self.zoom_extents(false);
    }
}
