//! Clone Options: Copy / Instance / Reference (index based, no shared pointers).

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CloneKind {
    #[default]
    Copy,
    Instance,
    Reference,
}

impl CloneKind {
    pub const ALL: [CloneKind; 3] = [Self::Copy, Self::Instance, Self::Reference];
    pub fn label(self) -> &'static str {
        match self {
            Self::Copy => "Copy",
            Self::Instance => "Instance",
            Self::Reference => "Reference",
        }
    }
}

impl Scene {
    /// Clones `id` with the given node transform. Instances share an
    /// instance group; references track their parent's base + lower stack.
    pub fn clone_object(&mut self, id: ObjId, kind: CloneKind, transform: Xform) -> Option<ObjId> {
        let src = self.objects.get(id)?.clone();
        if matches!(src.base, BaseObject::LightTarget) {
            return None; // targets are cloned with their light
        }
        // Lights are always independent copies (each owns its own target).
        let kind = if src.is_helper() { CloneKind::Copy } else { kind };
        let mut o = src.clone();
        o.name = self.unique_name(&src.name);
        o.transform = transform;
        o.selected = false;
        match kind {
            CloneKind::Copy => {
                o.instance_group = None;
                o.reference_parent = None;
                o.reference_shared = 0;
            }
            CloneKind::Instance => {
                let g = match src.instance_group {
                    Some(g) => g,
                    None => {
                        let g = self.new_instance_group();
                        self.objects[id].instance_group = Some(g);
                        g
                    }
                };
                o.instance_group = Some(g);
            }
            CloneKind::Reference => {
                o.instance_group = None;
                o.reference_parent = Some(id);
                o.reference_shared = src.modifiers.len();
            }
        }
        let shift = transform.translation - src.transform.translation;
        if let Some(t) = o.light().and_then(|l| l.target).and_then(|t| self.objects.get(t)).cloned() {
            let mut t = t;
            t.name = format!("{}.Target", o.name);
            t.transform.translation += shift;
            t.selected = false;
            let tid = self.objects.insert(t);
            if let Some(l) = o.light_mut() {
                l.target = Some(tid);
            }
        }
        Some(self.objects.insert(o))
    }

    /// After editing `id`: pushes base/stack changes to instances and references.
    pub fn propagate_from(&mut self, id: ObjId) {
        let Some(src) = self.objects.get(id).cloned() else {
            return;
        };
        let targets: Vec<ObjId> = self
            .objects
            .iter()
            .filter(|(k, o)| {
                *k != id && ((src.instance_group.is_some() && o.instance_group == src.instance_group) || o.reference_parent == Some(id))
            })
            .map(|(k, _)| k)
            .collect();
        for t in targets {
            let o = &mut self.objects[t];
            if o.reference_parent == Some(id) {
                let own: Vec<_> = o.modifiers.split_off(o.reference_shared.min(o.modifiers.len()));
                o.base = src.base.clone();
                o.modifiers = src.modifiers.clone();
                o.reference_shared = o.modifiers.len();
                o.modifiers.extend(own);
            } else {
                o.base = src.base.clone();
                o.modifiers = src.modifiers.clone();
            }
            o.active_level = o.active_level.filter(|&i| i < o.modifiers.len());
            o.evaluate();
        }
    }

    /// Make Unique: detaches an object from its instance group / reference parent.
    pub fn make_unique(&mut self, id: ObjId) {
        if let Some(o) = self.objects.get_mut(id) {
            o.instance_group = None;
            o.reference_parent = None;
            o.reference_shared = 0;
        }
    }
}
