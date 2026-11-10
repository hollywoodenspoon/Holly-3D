//! Editor mode enums, modal tools and dialog state.

use crate::mesh::ops::PolyOp;
use crate::primitives::PrimitiveKind;
use crate::scene::{CloneKind, ObjId, SceneObject, Xform};
use bevy::math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransformMode {
    #[default]
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CoordSystem {
    View,
    #[default]
    World,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PivotMode {
    PivotPoint,
    #[default]
    SelectionCenter,
    TransformCenter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CommandTab {
    #[default]
    Create,
    Modify,
    Pivot,
}

/// Interactive viewport tools (modal, like 3ds Max pick modes).
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Tool {
    #[default]
    None,
    Create(PrimitiveKind),
    /// Click-placed GeoGen procedural object.
    CreateGeoGen(crate::modifiers::GeoMode),
    /// Click-placed procedural world (Terrain, Ocean, Planet).
    CreateWorld(crate::generators::WorldKind),
    /// Click-placed Mech primitive (Gear, Piston, Hose).
    CreateMech(crate::mech::MechKind),
    /// Hose: click the object its top (start) or bottom (end) binds to.
    PickHoseEnd {
        top: bool,
    },
    TargetWeld {
        first: Option<crate::mesh::VertexId>,
    },
    InsertVertex,
    Cut {
        last: Option<crate::mesh::VertexId>,
    },
    QuickSlice {
        start: Option<Vec2>,
    },
    Attach,
    /// Boolean: pick operand objects (stays active until cancelled).
    PickBoolean,
    NormalAlign,
    /// Pick Material from Object (eyedropper) into the active slot.
    PickMaterial,
    /// Render Setup Post-FX: `[ Pick Focus Object ]` for depth of field.
    PickFocus,
    /// Edit Poly Create: points clicked so far.
    CreatePoly {
        points: Vec<crate::mesh::ops::sweep::CreatePoint>,
    },
    /// Hinge From Edge: pick the hinge edge.
    PickHinge,
    /// Extrude Along Spline: pick the path shape.
    PickSpline,
    /// Tools -> Align: pick the target object.
    PickAlign,
    /// Assemble Joint (`J`): mount on the child part, then on the parent.
    Joint {
        first: Option<crate::assembly::mount::Mount>,
    },
    /// Editable Spline Refine: click on segments to add vertices.
    RefineSpline,
    /// Editable Spline Boolean: click the second spline.
    PickSplineBoolean,
    /// Create a light; target lights take a second click for the target.
    CreateLight {
        kind: crate::render::LightKind,
        first: Option<bevy::math::Vec3>,
    },
    /// Create a camera; Target Cameras take a second click for the target.
    CreateCamera {
        kind: crate::scene::camera::CameraKind,
        first: Option<bevy::math::Vec3>,
    },
    /// Click-placed parametric shape (Circle, Rectangle, Star…).
    CreateShape(crate::splines::ShapeKind),
    /// Line: click = Corner knot, click-drag = Bezier knot, right-click ends.
    Line {
        view: usize,
        knots: Vec<crate::splines::Knot>,
        dragging: bool,
    },
}

impl Tool {
    pub fn label(&self) -> Option<&'static str> {
        match self {
            Tool::None => None,
            Tool::Create(_) => Some("Click in a viewport to place the object"),
            Tool::CreateGeoGen(_) => Some("Click in a viewport to place the GeoGen object"),
            Tool::CreateWorld(_) => Some("Click in a viewport to place the world object"),
            Tool::CreateMech(_) => Some("Click in a viewport to place the Mech part"),
            Tool::PickHoseEnd { top: true } => Some("Hose: click the Top End Object (Esc to cancel)"),
            Tool::PickHoseEnd { .. } => Some("Hose: click the Bottom End Object (Esc to cancel)"),
            Tool::TargetWeld { first: None } => Some("Target Weld: click the vertex to weld"),
            Tool::TargetWeld { .. } => Some("Target Weld: click the target vertex"),
            Tool::InsertVertex => Some("Insert Vertex: click on an edge or polygon"),
            Tool::Cut { .. } => Some("Cut: click vertices/edges in sequence, right-click to finish"),
            Tool::QuickSlice { start: None } => Some("QuickSlice: click the first point"),
            Tool::QuickSlice { .. } => Some("QuickSlice: click the second point"),
            Tool::Attach => Some("Attach: click an object to merge it"),
            Tool::PickBoolean => Some("Boolean: click objects to add them as operands (Esc to stop)"),
            Tool::NormalAlign => Some("Normal Align: click a face to align the UVW gizmo"),
            Tool::CreatePoly { points } if points.len() < 3 => Some("Create: click vertices or empty space"),
            Tool::CreatePoly { .. } => Some("Create: click the first point or right-click to finish"),
            Tool::PickHinge => Some("Hinge From Edge: click the hinge edge"),
            Tool::PickSpline => Some("Extrude Along Spline: click a shape"),
            Tool::PickAlign => Some("Align: click the target object (Esc to cancel)"),
            Tool::Joint { first: None } => Some("Assemble Joint: click a mount point on the part to attach (child)"),
            Tool::Joint { .. } => Some("Assemble Joint: click the mount point on the parent part"),
            Tool::RefineSpline => Some("Refine: click on a segment to add a vertex (Esc to stop)"),
            Tool::PickSplineBoolean => Some("Boolean: click the other closed spline"),
            Tool::PickMaterial => Some("Pick Material: click an object to load its material into the active slot"),
            Tool::PickFocus => Some("Depth of Field: click the object to keep in focus"),
            Tool::CreateLight { kind, first: None } if kind.has_target() => Some("Click to place the light"),
            Tool::CreateLight { first: Some(_), .. } => Some("Click to place the light's target"),
            Tool::CreateLight { .. } => Some("Click in a viewport to place the light"),
            Tool::CreateCamera { first: None, .. } => Some("Click to place the camera"),
            Tool::CreateCamera { .. } => Some("Click to place the camera's target"),
            Tool::CreateShape(_) => Some("Click in a viewport to place the shape"),
            Tool::Line { knots, .. } if knots.is_empty() => Some("Line: click for corners, drag for Bezier handles"),
            Tool::Line { .. } => Some("Line: right-click to finish, click the first vertex to close"),
        }
    }
}

/// Floating `[□]` caddy with a live-previewed op.
#[derive(Clone, Debug)]
pub struct Caddy {
    pub op: PolyOp,
    pub object: ObjId,
    pub before: SceneObject,
    /// Last op actually applied in preview (to avoid redundant re-evaluation).
    pub previewed: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CloneDialog {
    pub sources: Vec<(ObjId, Xform)>,
    /// World-space delta of the shift-drag (applied once per copy).
    pub delta: bevy::math::Mat4,
    pub kind: CloneKind,
    pub copies: u32,
}

#[derive(Clone, Debug)]
pub struct SlicePlane {
    pub xform: Xform,
    pub split: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SoftSelection {
    pub enabled: bool,
    pub falloff: f32,
    pub pinch: f32,
    pub bubble: f32,
}

/// Compact Material Editor (M) window state.
#[derive(Clone, Debug, Default)]
pub struct MatEditor {
    pub open: bool,
    /// Active sample slot (0..24).
    pub slot: usize,
    /// `[ Mode: Compact | Slate Node Graph ]`.
    pub slate: bool,
    /// Folder typed for Import PBR Texture Folder.
    pub pbr_folder: String,
    /// Sub-material the rollouts edit when the slot is a Multi/Sub-Object.
    pub sub: usize,
}

/// Spinner values of the Editable Spline Geometry rollout.
#[derive(Clone, Debug)]
pub struct SplineUi {
    pub fillet: f32,
    pub chamfer: f32,
    pub weld: f32,
    pub divide: u32,
    pub outline: f32,
    /// 0 = horizontal (X), 1 = vertical (Y), 2 = both.
    pub mirror_axis: usize,
    pub mirror_copy: bool,
    pub bool_op: crate::splines::boolean::SplineBoolOp,
}

impl Default for SplineUi {
    fn default() -> Self {
        Self {
            fillet: 0.1,
            chamfer: 0.1,
            weld: 0.05,
            divide: 1,
            outline: 0.05,
            mirror_axis: 0,
            mirror_copy: false,
            bool_op: Default::default(),
        }
    }
}
