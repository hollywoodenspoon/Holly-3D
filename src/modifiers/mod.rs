//! Non-destructive modifier stack (bottom-to-top fold).

pub mod ffd;
pub mod geogen;
pub mod lathe;
pub mod smart_uv;
pub mod spline_extrude;
pub mod unwrap_uvw;
pub mod uvw_map;
pub mod vertex_paint;

pub use ffd::FfdMod;
pub use geogen::{GeoGenMod, GeoGenParams, GeoMode};
pub use lathe::{LatheAlign, LatheMod};
pub use spline_extrude::ExtrudeMod;
pub use unwrap_uvw::UnwrapMod;
pub use uvw_map::{MapType, UvwMapMod};
pub use vertex_paint::VertexPaintMod;

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
    Ffd(FfdMod),
    UvwMap(UvwMapMod),
    Unwrap(UnwrapMod),
    VertexPaint(VertexPaintMod),
    Lathe(LatheMod),
    Extrude(ExtrudeMod),
    GeoGen(GeoGenMod),
}

/// Modifier sub-objects that are not mesh components (gizmos and lattices).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModSub {
    ControlPoints,
    Lattice,
    Gizmo,
    /// Lathe revolution axis.
    Axis,
}

impl ModSub {
    pub fn label(self) -> &'static str {
        match self {
            ModSub::ControlPoints => "Control Points",
            ModSub::Lattice => "Lattice",
            ModSub::Gizmo => "Gizmo",
            ModSub::Axis => "Axis",
        }
    }
}

impl ModifierKind {
    pub fn default_name(&self) -> &'static str {
        match self {
            ModifierKind::EditPoly(_) => "Edit Poly",
            ModifierKind::Ffd(f) => f.name(),
            ModifierKind::UvwMap(_) => "UVW Map",
            ModifierKind::Unwrap(_) => "Unwrap UVW",
            ModifierKind::VertexPaint(_) => "VertexPaint",
            ModifierKind::Lathe(_) => "Lathe",
            ModifierKind::Extrude(_) => "Extrude",
            ModifierKind::GeoGen(_) => "GeoGen",
        }
    }

    /// Mesh sub-object modes 1–5 (Edit Poly only).
    pub fn supports_sub_objects(&self) -> bool {
        matches!(self, ModifierKind::EditPoly(_))
    }

    /// Gizmo-style sub-objects listed under the stack expander (keys 1, 2…).
    pub fn gizmo_subs(&self) -> &'static [ModSub] {
        match self {
            ModifierKind::Ffd(_) => &[ModSub::ControlPoints, ModSub::Lattice],
            ModifierKind::UvwMap(_) => &[ModSub::Gizmo],
            ModifierKind::Lathe(_) => &[ModSub::Axis],
            _ => &[],
        }
    }

    /// Called once with the modifier's input when it is added to a stack
    /// (fits lattices / gizmos, captures UVs — 3ds Max "Set Volume").
    pub fn init(&mut self, input: &EditablePoly) {
        match self {
            ModifierKind::Ffd(f) => f.fit(input),
            ModifierKind::UvwMap(u) => u.fit(input),
            ModifierKind::Unwrap(u) => u.capture(input),
            ModifierKind::EditPoly(_)
            | ModifierKind::VertexPaint(_)
            | ModifierKind::Lathe(_)
            | ModifierKind::Extrude(_)
            | ModifierKind::GeoGen(_) => {}
        }
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
            ModifierKind::Ffd(f) => f.apply(mesh),
            ModifierKind::UvwMap(u) => u.apply(mesh),
            ModifierKind::Unwrap(u) => u.apply(mesh),
            ModifierKind::VertexPaint(v) => v.apply(mesh),
            ModifierKind::GeoGen(g) => g.apply(mesh),
            // shape modifiers pass meshes through untouched
            ModifierKind::Lathe(_) | ModifierKind::Extrude(_) => {}
        }
    }

    /// Shape -> mesh modifiers (Lathe, Extrude) consume a shape flowing up the stack.
    pub fn consumes_shape(&self) -> bool {
        matches!(self.kind, ModifierKind::Lathe(_) | ModifierKind::Extrude(_))
    }

    pub fn build_from_shape(&self, shape: &crate::splines::Shape) -> EditablePoly {
        match &self.kind {
            ModifierKind::Lathe(l) => l.build(shape),
            ModifierKind::Extrude(e) => e.build(shape),
            _ => EditablePoly::new(),
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
    ModifierListItem { name: "FFD 2x2x2", available: true },
    ModifierListItem { name: "FFD 3x3x3", available: true },
    ModifierListItem { name: "FFD 4x4x4", available: true },
    ModifierListItem { name: "UVW Map", available: true },
    ModifierListItem { name: "Unwrap UVW", available: true },
    ModifierListItem { name: "VertexPaint", available: true },
    ModifierListItem { name: "GeoGen", available: true },
    ModifierListItem { name: "TurboSmooth", available: false },
    ModifierListItem { name: "Symmetry", available: false },
    ModifierListItem { name: "Shell", available: false },
    ModifierListItem { name: "Bend", available: false },
    ModifierListItem { name: "Twist", available: false },
    ModifierListItem { name: "Taper", available: false },
    ModifierListItem { name: "Push", available: false },
    ModifierListItem { name: "Chamfer", available: false },
    ModifierListItem { name: "Lathe", available: true },
    ModifierListItem { name: "Extrude", available: true },
    ModifierListItem { name: "Edit Normals", available: false },
    ModifierListItem { name: "Boolean", available: false },
    ModifierListItem { name: "ProOptimizer", available: false },
];

pub fn make_modifier(name: &str) -> Option<ModifierEntry> {
    match name {
        "Edit Poly" => Some(ModifierEntry::new(ModifierKind::EditPoly(EditPolyMod::default()))),
        "FFD 2x2x2" => Some(ModifierEntry::new(ModifierKind::Ffd(FfdMod::new(2)))),
        "FFD 3x3x3" => Some(ModifierEntry::new(ModifierKind::Ffd(FfdMod::new(3)))),
        "FFD 4x4x4" => Some(ModifierEntry::new(ModifierKind::Ffd(FfdMod::new(4)))),
        "UVW Map" => Some(ModifierEntry::new(ModifierKind::UvwMap(UvwMapMod::default()))),
        "Unwrap UVW" => Some(ModifierEntry::new(ModifierKind::Unwrap(UnwrapMod::default()))),
        "VertexPaint" => Some(ModifierEntry::new(ModifierKind::VertexPaint(VertexPaintMod::default()))),
        "Lathe" => Some(ModifierEntry::new(ModifierKind::Lathe(LatheMod::default()))),
        "Extrude" => Some(ModifierEntry::new(ModifierKind::Extrude(ExtrudeMod::default()))),
        "GeoGen" => Some(ModifierEntry::new(ModifierKind::GeoGen(GeoGenMod::default()))),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
