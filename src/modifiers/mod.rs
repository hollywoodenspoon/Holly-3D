//! Non-destructive modifier stack (bottom-to-top fold).

use crate::mesh::ops::PolyOp;
use crate::mesh::{EditablePoly, SelectionSets};

/// The Edit Poly modifier: records every Edit Poly op together with the
/// selection it acted on, and replays them on its input mesh.
#[derive(Clone, Debug, Default)]
pub struct EditPolyMod {
    pub ops: Vec<(SelectionSets, PolyOp)>,
    /// Current sub-object selection held by the modifier.
    pub selection: SelectionSets,
}

impl EditPolyMod {
    pub fn apply(&self, mesh: &mut EditablePoly) {
        for (sel, op) in &self.ops {
            mesh.apply_selection_sets(sel);
            op.apply(mesh);
        }
        mesh.apply_selection_sets(&self.selection);
    }
}

#[derive(Clone, Debug)]
pub enum ModifierKind {
    EditPoly(EditPolyMod),
}

impl ModifierKind {
    pub fn default_name(&self) -> &'static str {
        match self {
            ModifierKind::EditPoly(_) => "Edit Poly",
        }
    }

    pub fn supports_sub_objects(&self) -> bool {
        matches!(self, ModifierKind::EditPoly(_))
    }
}

#[derive(Clone, Debug)]
pub struct ModifierEntry {
    pub enabled_in_viewport: bool,
    pub name: String,
    pub kind: ModifierKind,
}

impl ModifierEntry {
    pub fn new(kind: ModifierKind) -> Self {
        Self { enabled_in_viewport: true, name: kind.default_name().to_string(), kind }
    }

    pub fn apply(&self, mesh: &mut EditablePoly) {
        if !self.enabled_in_viewport {
            return;
        }
        match &self.kind {
            ModifierKind::EditPoly(e) => e.apply(mesh),
        }
    }
}

/// Entries offered by the searchable Modifier List. Later phases append here.
pub struct ModifierListItem {
    pub name: &'static str,
    pub available: bool,
}

pub const MODIFIER_LIST: &[ModifierListItem] = &[
    ModifierListItem { name: "Edit Poly", available: true },
    ModifierListItem { name: "FFD 2x2x2", available: false },
    ModifierListItem { name: "FFD 3x3x3", available: false },
    ModifierListItem { name: "FFD 4x4x4", available: false },
    ModifierListItem { name: "UVW Map", available: false },
    ModifierListItem { name: "Unwrap UVW", available: false },
    ModifierListItem { name: "TurboSmooth", available: false },
    ModifierListItem { name: "Symmetry", available: false },
    ModifierListItem { name: "Shell", available: false },
    ModifierListItem { name: "Bend", available: false },
    ModifierListItem { name: "Twist", available: false },
    ModifierListItem { name: "Taper", available: false },
    ModifierListItem { name: "Push", available: false },
    ModifierListItem { name: "Chamfer", available: false },
    ModifierListItem { name: "Lathe", available: false },
    ModifierListItem { name: "Extrude", available: false },
    ModifierListItem { name: "Edit Normals", available: false },
    ModifierListItem { name: "Boolean", available: false },
    ModifierListItem { name: "ProOptimizer", available: false },
    ModifierListItem { name: "VertexPaint", available: false },
];

pub fn make_modifier(name: &str) -> Option<ModifierEntry> {
    match name {
        "Edit Poly" => Some(ModifierEntry::new(ModifierKind::EditPoly(EditPolyMod::default()))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::ops::ExtrudeMode;
    use crate::primitives::{Primitive, PrimitiveKind};
    use bevy::math::Vec3;

    #[test]
    fn edit_poly_replays_recorded_ops() {
        let base = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
        let mut m = base.clone();
        let top: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).dot(Vec3::Z) > 0.9).collect();
        m.polygons[top[0]].selected = true;
        let sel = m.selection_sets();
        let op = PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 1.0 };
        op.apply(&mut m);
        let md = EditPolyMod { ops: vec![(sel, op)], selection: m.selection_sets() };
        let mut out = base.clone();
        md.apply(&mut out);
        out.validate().unwrap();
        assert_eq!(out.counts(), (12, 20, 10));
        assert_eq!(out.selected_polygons().len(), 1);
        let mut disabled = ModifierEntry::new(ModifierKind::EditPoly(md));
        disabled.enabled_in_viewport = false;
        let mut out2 = base.clone();
        disabled.apply(&mut out2);
        assert_eq!(out2.counts(), (8, 12, 6));
    }
}
