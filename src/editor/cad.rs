//! SolidCraft-style CAD editing: picking Loft cross-sections from shape
//! objects into the active Loft modifier.

use super::*;
use crate::modifiers::ModifierKind;
use crate::modifiers::cad::LoftSection;

impl Editor {
    /// Pick Shape: adds the first spline of `other` (in its own XY plane) as a
    /// Loft section at the modifier's "next at" path %.
    pub fn pick_loft_section(&mut self, other: ObjId) {
        let Some(id) = self.current() else { return };
        if !matches!(self.current_obj().and_then(|o| o.active_modifier()), Some(ModifierKind::Loft(_))) {
            return;
        }
        let src = &self.scene.objects[other];
        let Some(pl) = src.shape.as_ref().and_then(|sh| sh.polylines().into_iter().find(|p| p.pts.len() >= 2)) else {
            self.status = "Loft: pick a shape (spline) object".into();
            return;
        };
        let name = src.name.clone();
        let mut at = 0.0;
        self.edit_active_modifier("Pick Loft Shape", |k, _| {
            if let ModifierKind::Loft(l) = k {
                at = l.next_at;
                l.add_section(LoftSection::from_polyline(name.clone(), at, &pl));
                l.selected = l.sections.iter().position(|s| s.name == name && s.at == at);
            }
        });
        self.scene.select_only(Some(id));
        self.status = format!("Loft: added {name} at {at:.0}% of the path");
    }

    /// Removes the highlighted Loft section.
    pub fn remove_loft_section(&mut self) {
        self.edit_active_modifier("Remove Loft Section", |k, _| {
            if let ModifierKind::Loft(l) = k
                && let Some(i) = l.selected.filter(|i| *i < l.sections.len())
            {
                l.sections.remove(i);
                l.selected = None;
            }
        });
    }
}
