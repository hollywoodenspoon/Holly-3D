//! Boolean modifier editing: picking operand objects into the active Boolean
//! (the picked object is consumed) and extracting them again.

use super::*;
use crate::modifiers::{ModifierKind, boolean};
use crate::scene::BaseObject;

impl Editor {
    fn active_boolean(&self) -> bool {
        matches!(self.current_obj().and_then(|o| o.active_modifier()), Some(ModifierKind::Boolean(_)))
    }

    /// Pick Operand B: bakes `other` into the active Boolean in this object's mesh space.
    pub fn pick_boolean_operand(&mut self, other: ObjId) {
        let Some(id) = self.current() else { return };
        if id == other || !self.active_boolean() {
            return;
        }
        let src = &self.scene.objects[other];
        if src.is_helper() || src.evaluated_mesh.polygons.is_empty() {
            self.status = "Boolean: pick a mesh object".into();
            return;
        }
        let rel = self.scene.objects[id].world_to_mesh() * src.world_matrix();
        let mut mesh = (*src.evaluated_mesh).clone();
        mesh.clear_selection();
        mesh.transform(rel);
        let name = src.name.clone();
        let warn = boolean::to_manifold(&mesh).err();
        self.edit_active_modifier("Pick Boolean Operand", |k, _| {
            if let ModifierKind::Boolean(b) = k {
                b.add_operand(name.clone(), mesh);
            }
        });
        self.scene.delete(other);
        self.scene.select_only(Some(id));
        self.status = match warn {
            Some(_) => format!("{name} is not a closed manifold mesh: its operation is skipped"),
            None => format!("Added operand {name}"),
        };
    }

    /// Extract Operand: turns the highlighted operand back into a scene object.
    pub fn extract_boolean_operand(&mut self) {
        let Some(id) = self.current() else { return };
        let mut taken = None;
        self.edit_active_modifier("Extract Operand", |k, _| {
            if let ModifierKind::Boolean(b) = k {
                taken = b.take_selected();
            }
        });
        let Some(op) = taken else {
            self.history.discard_last();
            return;
        };
        let (xf, off) = (self.scene.objects[id].transform, self.scene.objects[id].offset);
        let nid = self.scene.add(BaseObject::EditablePoly(op.mesh.into()), Vec3::ZERO);
        // keep the operand's original name when it is free (Sphere001 comes back as Sphere001)
        let taken = self.scene.objects.values().any(|o| o.name == op.name);
        let name = if taken { self.scene.unique_name(&op.name) } else { op.name.clone() };
        let o = &mut self.scene.objects[nid];
        o.name = name;
        o.transform = xf;
        o.offset = off;
        o.evaluate();
        self.status = format!("Extracted {} as {}", op.name, self.scene.objects[nid].name);
        self.scene.select_only(Some(id));
    }
}
