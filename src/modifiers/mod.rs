//! Non-destructive modifier stack (bottom-to-top fold).

pub mod battlefield;
pub mod boolean;
pub mod cad;
pub mod catalog;
pub mod chamfer;
pub mod deformers;
pub mod edit_normals;
pub mod ffd;
pub mod geogen;
pub mod greeble;
pub mod lathe;
pub mod medieval;
pub mod panel_groove;
pub mod pro_optimizer;
pub mod shell;
pub mod smart_uv;
pub mod spline_extrude;
pub mod symmetry;
pub mod turbosmooth;
pub mod unwrap_uvw;
pub mod uvw_map;
pub mod vertex_paint;

pub use boolean::{BoolOp, BooleanMod};
pub use chamfer::{ChamferFilter, ChamferMod};
pub use deformers::{BendMod, Limits, PushMod, TaperEffect, TaperMod, TwistMod};
pub use edit_normals::{EditNormalsMod, NormalKind, NormalSelBy};
pub use ffd::FfdMod;
pub use geogen::{GeoGenMod, GeoGenParams, GeoMode};
pub use greeble::GreebleMod;
pub use lathe::{LatheAlign, LatheMod};
pub use panel_groove::PanelGrooveMod;
pub use pro_optimizer::ProOptimizerMod;
pub use shell::ShellMod;
pub use spline_extrude::ExtrudeMod;
pub use symmetry::SymmetryMod;
pub use turbosmooth::TurboSmoothMod;
pub use unwrap_uvw::UnwrapMod;
pub use uvw_map::{MapType, UvwMapMod};
pub use vertex_paint::VertexPaintMod;

use crate::mesh::ops::PolyOp;
use crate::mesh::{EditablePoly, SelectionSets};

/// The Edit Poly modifier: records every Edit Poly op together with the
/// selection it acted on, and replays them on its input mesh.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ModifierKind {
    EditPoly(EditPolyMod),
    Ffd(FfdMod),
    UvwMap(UvwMapMod),
    Unwrap(UnwrapMod),
    VertexPaint(VertexPaintMod),
    Lathe(LatheMod),
    Extrude(ExtrudeMod),
    GeoGen(GeoGenMod),
    Greeble(GreebleMod),
    Chamfer(ChamferMod),
    PanelGroove(PanelGrooveMod),
    Symmetry(SymmetryMod),
    Shell(ShellMod),
    TurboSmooth(TurboSmoothMod),
    Bend(BendMod),
    Twist(TwistMod),
    Taper(TaperMod),
    Push(PushMod),
    EditNormals(EditNormalsMod),
    Boolean(BooleanMod),
    ProOptimizer(ProOptimizerMod),
    Terrain(crate::generators::TerrainMod),
    Ocean(crate::generators::OceanMod),
    Settlement(crate::generators::SettlementMod),
    Architect(medieval::ArchitectMod),
    Battlefield(battlefield::BattlefieldMod),
    Loft(cad::LoftMod),
}

/// Modifier sub-objects that are not mesh components (gizmos and lattices).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModSub {
    ControlPoints,
    Lattice,
    Gizmo,
    /// Lathe revolution axis.
    Axis,
    /// Edit Normals face-corner normals.
    Normal,
    /// Bend / Twist / Taper gizmo center.
    Center,
    /// Symmetry mirror plane.
    Mirror,
    /// Battlefield blast-zone spheres.
    BlastZones,
}

impl ModSub {
    pub fn label(self) -> &'static str {
        match self {
            ModSub::ControlPoints => "Control Points",
            ModSub::Lattice => "Lattice",
            ModSub::Gizmo => "Gizmo",
            ModSub::Axis => "Axis",
            ModSub::Normal => "Normal",
            ModSub::Center => "Center",
            ModSub::Mirror => "Mirror",
            ModSub::BlastZones => "Blast Zones",
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
            ModifierKind::Terrain(_) => "Demiurge Terrain",
            ModifierKind::Ocean(_) => "Demiurge Ocean",
            ModifierKind::Greeble(_) => "Greeble",
            ModifierKind::Chamfer(_) => "Chamfer",
            ModifierKind::PanelGroove(_) => "Panel Groove",
            ModifierKind::Symmetry(_) => "Symmetry",
            ModifierKind::Shell(_) => "Shell",
            ModifierKind::TurboSmooth(_) => "TurboSmooth",
            ModifierKind::Bend(_) => "Bend",
            ModifierKind::Twist(_) => "Twist",
            ModifierKind::Taper(_) => "Taper",
            ModifierKind::Push(_) => "Push",
            ModifierKind::EditNormals(_) => "Edit Normals",
            ModifierKind::Boolean(_) => "Boolean",
            ModifierKind::ProOptimizer(_) => "ProOptimizer",
            ModifierKind::Settlement(_) => "Demiurge Settlement",
            ModifierKind::Architect(_) => "Demiurge Medieval Architect",
            ModifierKind::Battlefield(_) => "Demiurge Battlefield & Ruin",
            ModifierKind::Loft(_) => "Loft",
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
            ModifierKind::EditNormals(_) => &[ModSub::Normal],
            ModifierKind::Bend(_) | ModifierKind::Twist(_) | ModifierKind::Taper(_) => &[ModSub::Gizmo, ModSub::Center],
            ModifierKind::Symmetry(_) => &[ModSub::Mirror],
            ModifierKind::Battlefield(_) => &[ModSub::BlastZones],
            _ => &[],
        }
    }

    /// Called once with the modifier's input when it is added to a stack
    /// (fits lattices / gizmos, captures UVs — "Set Volume").
    pub fn init(&mut self, input: &EditablePoly) {
        match self {
            ModifierKind::Ffd(f) => f.fit(input),
            ModifierKind::UvwMap(u) => u.fit(input),
            ModifierKind::Unwrap(u) => u.capture(input),
            ModifierKind::ProOptimizer(p) => p.calculate(input),
            _ => {}
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
            ModifierKind::Greeble(g) => g.apply(mesh),
            ModifierKind::Chamfer(c) => c.apply(mesh),
            ModifierKind::PanelGroove(g) => g.apply(mesh),
            ModifierKind::Symmetry(s) => s.apply(mesh),
            ModifierKind::Shell(s) => s.apply(mesh),
            ModifierKind::TurboSmooth(t) => t.apply(mesh),
            ModifierKind::Bend(b) => b.apply(mesh),
            ModifierKind::Twist(t) => t.apply(mesh),
            ModifierKind::Taper(t) => t.apply(mesh),
            ModifierKind::Push(p) => p.apply(mesh),
            ModifierKind::EditNormals(e) => e.apply(mesh),
            ModifierKind::Boolean(b) => b.apply(mesh),
            ModifierKind::ProOptimizer(p) => p.apply(mesh),
            ModifierKind::Terrain(t) => t.apply(mesh),
            ModifierKind::Ocean(o) => o.apply(mesh),
            ModifierKind::Settlement(g) => g.apply(mesh),
            ModifierKind::Architect(a) => a.apply(mesh),
            ModifierKind::Battlefield(b) => b.apply(mesh),
            // shape modifiers pass meshes through untouched
            ModifierKind::Lathe(_) | ModifierKind::Extrude(_) | ModifierKind::Loft(_) => {}
        }
    }

    /// Shape -> mesh modifiers (Lathe, Extrude) consume a shape flowing up the stack.
    pub fn consumes_shape(&self) -> bool {
        matches!(self.kind, ModifierKind::Lathe(_) | ModifierKind::Extrude(_) | ModifierKind::Loft(_) | ModifierKind::Settlement(_) | ModifierKind::Architect(_))
    }

    pub fn build_from_shape(&self, shape: &crate::splines::Shape) -> EditablePoly {
        match &self.kind {
            ModifierKind::Lathe(l) => l.build(shape),
            ModifierKind::Extrude(e) => e.build(shape),
            ModifierKind::Loft(l) => l.build(shape),
            ModifierKind::Settlement(g) => g.build_from_shape(shape),
            ModifierKind::Architect(a) => a.build_from_shape(shape),
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
    ModifierListItem { name: "Demiurge Terrain", available: true },
    ModifierListItem { name: "Demiurge Ocean", available: true },
    ModifierListItem { name: "Greeble", available: true },
    ModifierListItem { name: "TurboSmooth", available: true },
    ModifierListItem { name: "Symmetry", available: true },
    ModifierListItem { name: "Shell", available: true },
    ModifierListItem { name: "Bend", available: true },
    ModifierListItem { name: "Twist", available: true },
    ModifierListItem { name: "Taper", available: true },
    ModifierListItem { name: "Push", available: true },
    ModifierListItem { name: "Chamfer", available: true },
    ModifierListItem { name: "Panel Groove", available: true },
    ModifierListItem { name: "Lathe", available: true },
    ModifierListItem { name: "Extrude", available: true },
    ModifierListItem { name: "Loft", available: true },
    ModifierListItem { name: "Edit Normals", available: true },
    ModifierListItem { name: "Boolean", available: true },
    ModifierListItem { name: "ProOptimizer", available: true },
    ModifierListItem { name: "Demiurge Settlement", available: true },
    ModifierListItem { name: "Demiurge Medieval Architect", available: true },
    ModifierListItem { name: "Demiurge Battlefield & Ruin", available: true },
];

pub fn make_modifier(name: &str) -> Option<ModifierEntry> {
    match catalog::current_name(name) {
        "Edit Poly" => Some(ModifierEntry::new(ModifierKind::EditPoly(EditPolyMod::default()))),
        "FFD 2x2x2" => Some(ModifierEntry::new(ModifierKind::Ffd(FfdMod::new(2)))),
        "FFD 3x3x3" => Some(ModifierEntry::new(ModifierKind::Ffd(FfdMod::new(3)))),
        "FFD 4x4x4" => Some(ModifierEntry::new(ModifierKind::Ffd(FfdMod::new(4)))),
        "UVW Map" => Some(ModifierEntry::new(ModifierKind::UvwMap(UvwMapMod::default()))),
        "Unwrap UVW" => Some(ModifierEntry::new(ModifierKind::Unwrap(UnwrapMod::default()))),
        "VertexPaint" => Some(ModifierEntry::new(ModifierKind::VertexPaint(VertexPaintMod::default()))),
        "Lathe" => Some(ModifierEntry::new(ModifierKind::Lathe(LatheMod::default()))),
        "Extrude" => Some(ModifierEntry::new(ModifierKind::Extrude(ExtrudeMod::default()))),
        "Loft" => Some(ModifierEntry::new(ModifierKind::Loft(Default::default()))),
        "GeoGen" => Some(ModifierEntry::new(ModifierKind::GeoGen(GeoGenMod::default()))),
        "Demiurge Terrain" => Some(ModifierEntry::new(ModifierKind::Terrain(Default::default()))),
        "Demiurge Ocean" => Some(ModifierEntry::new(ModifierKind::Ocean(Default::default()))),
        "Greeble" => Some(ModifierEntry::new(ModifierKind::Greeble(GreebleMod::default()))),
        "Chamfer" => Some(ModifierEntry::new(ModifierKind::Chamfer(ChamferMod::default()))),
        "Panel Groove" => Some(ModifierEntry::new(ModifierKind::PanelGroove(PanelGrooveMod::default()))),
        "Symmetry" => Some(ModifierEntry::new(ModifierKind::Symmetry(SymmetryMod::default()))),
        "Shell" => Some(ModifierEntry::new(ModifierKind::Shell(ShellMod::default()))),
        "TurboSmooth" => Some(ModifierEntry::new(ModifierKind::TurboSmooth(TurboSmoothMod::default()))),
        "Bend" => Some(ModifierEntry::new(ModifierKind::Bend(BendMod::default()))),
        "Twist" => Some(ModifierEntry::new(ModifierKind::Twist(TwistMod::default()))),
        "Taper" => Some(ModifierEntry::new(ModifierKind::Taper(TaperMod::default()))),
        "Push" => Some(ModifierEntry::new(ModifierKind::Push(PushMod::default()))),
        "Edit Normals" => Some(ModifierEntry::new(ModifierKind::EditNormals(EditNormalsMod::default()))),
        "Boolean" => Some(ModifierEntry::new(ModifierKind::Boolean(BooleanMod::default()))),
        "ProOptimizer" => Some(ModifierEntry::new(ModifierKind::ProOptimizer(ProOptimizerMod::default()))),
        "Demiurge Settlement" => Some(ModifierEntry::new(ModifierKind::Settlement(Default::default()))),
        "Demiurge Medieval Architect" => Some(ModifierEntry::new(ModifierKind::Architect(Default::default()))),
        "Demiurge Battlefield & Ruin" => Some(ModifierEntry::new(ModifierKind::Battlefield(Default::default()))),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_adv;
#[cfg(test)]
mod tests_gizmo;
#[cfg(test)]
mod tests_stack;
