//! `PolyOp`: a recordable, replayable Edit Poly command. Used directly on an
//! Editable Poly base object and recorded inside the Edit Poly modifier.

use super::*;
use bevy::math::Mat4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ExtrudeMode {
    #[default]
    Group,
    LocalNormal,
    ByPolygon,
}

impl ExtrudeMode {
    pub const ALL: [ExtrudeMode; 3] = [Self::Group, Self::LocalNormal, Self::ByPolygon];
    pub fn label(self) -> &'static str {
        match self {
            Self::Group => "Group",
            Self::LocalNormal => "Local Normal",
            Self::ByPolygon => "By Polygon",
        }
    }
}

/// A point the Cut tool clicked: an existing vertex or a spot on an edge.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CutPoint {
    Vertex(VertexId),
    Edge(VertexId, VertexId, f32),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum PolyOp {
    ExtrudeVertices {
        height: f32,
        width: f32,
    },
    ExtrudeEdges {
        height: f32,
        width: f32,
    },
    ExtrudePolygons {
        mode: ExtrudeMode,
        height: f32,
    },
    Bevel {
        mode: ExtrudeMode,
        height: f32,
        outline: f32,
    },
    Inset {
        by_polygon: bool,
        amount: f32,
    },
    Outline {
        amount: f32,
    },
    ChamferVertices {
        amount: f32,
    },
    ChamferEdges {
        amount: f32,
        segments: u32,
        tension: f32,
        open: bool,
    },
    /// Panel Line / Seam Groove along the selected edges.
    PanelGroove {
        width: f32,
        depth: f32,
        lip: f32,
        material_id: u16,
    },
    /// Hole / Counterbore / Countersink drilled into each selected polygon.
    Hole { spec: crate::modifiers::cad::HoleSpec },
    ConnectEdges {
        segments: u32,
        pinch: f32,
        slide: f32,
    },
    ConnectVertices,
    Weld {
        level: SubObject,
        threshold: f32,
    },
    TargetWeld {
        from: VertexId,
        to: VertexId,
    },
    Collapse {
        level: SubObject,
    },
    RemoveVertices,
    RemoveEdges {
        remove_verts: bool,
    },
    BreakVertices,
    SplitEdges,
    Delete {
        level: SubObject,
    },
    InsertVertexOnEdge {
        a: VertexId,
        b: VertexId,
        t: f32,
    },
    InsertVertexInPolygon {
        poly: PolyId,
        point: Vec3,
    },
    Cut {
        from: CutPoint,
        to: CutPoint,
        view_dir: Vec3,
    },
    Slice {
        point: Vec3,
        normal: Vec3,
        only_selected: bool,
        split: bool,
    },
    Cap,
    BridgeBorders {
        segments: u32,
        taper: f32,
        twist: i32,
    },
    BridgePolygons {
        segments: u32,
        taper: f32,
        twist: i32,
    },
    MakePlanar {
        level: SubObject,
    },
    Flatten {
        level: SubObject,
        axis: usize,
    },
    Flip,
    SetSmoothing {
        bit: u32,
        on: bool,
    },
    ClearSmoothing,
    AutoSmooth {
        threshold: f32,
    },
    SetMaterialId {
        id: u16,
    },
    Tessellate {
        face_center: bool,
    },
    MSmooth {
        strength: f32,
    },
    Transform {
        verts: Vec<VertexId>,
        matrix: Mat4,
    },
    SoftTransform {
        weights: Vec<(VertexId, f32)>,
        matrix: Mat4,
    },
    ShiftExtrudeEdges,
    Attach {
        mesh: Box<EditablePoly>,
    },
    /// Hinge From Edge: axis through `origin` along `axis` (zero until picked).
    Hinge {
        origin: Vec3,
        axis: Vec3,
        angle: f32,
        segments: u32,
    },
    /// Extrude Along Spline: `path` in mesh space (empty until picked).
    ExtrudeAlongSpline {
        path: Vec<Vec3>,
        segments: u32,
        taper: f32,
        twist: f32,
        align: bool,
    },
    CreatePolygon {
        points: Vec<sweep::CreatePoint>,
        view_dir: Vec3,
    },
}

impl PolyOp {
    pub fn name(&self) -> &'static str {
        use PolyOp::*;
        match self {
            ExtrudeVertices { .. } | ExtrudeEdges { .. } | ExtrudePolygons { .. } => "Extrude",
            Bevel { .. } => "Bevel",
            Inset { .. } => "Inset",
            Outline { .. } => "Outline",
            ChamferVertices { .. } | ChamferEdges { .. } => "Chamfer",
            PanelGroove { .. } => "Panel Groove",
            Hole { spec } => spec.kind.label(),
            ConnectEdges { .. } | ConnectVertices => "Connect",
            Weld { .. } => "Weld",
            TargetWeld { .. } => "Target Weld",
            Collapse { .. } => "Collapse",
            RemoveVertices | RemoveEdges { .. } => "Remove",
            BreakVertices => "Break",
            SplitEdges => "Split",
            Delete { .. } => "Delete",
            InsertVertexOnEdge { .. } | InsertVertexInPolygon { .. } => "Insert Vertex",
            Cut { .. } => "Cut",
            Slice { .. } => "Slice",
            Cap => "Cap",
            BridgeBorders { .. } | BridgePolygons { .. } => "Bridge",
            MakePlanar { .. } => "Make Planar",
            Flatten { .. } => "Make Planar Axis",
            Flip => "Flip",
            SetSmoothing { .. } | ClearSmoothing => "Smoothing Groups",
            AutoSmooth { .. } => "Auto Smooth",
            SetMaterialId { .. } => "Set ID",
            Tessellate { .. } => "Tessellate",
            MSmooth { .. } => "MSmooth",
            Transform { .. } | SoftTransform { .. } => "Transform",
            ShiftExtrudeEdges => "Shift Extrude",
            Attach { .. } => "Attach",
            Hinge { .. } => "Hinge From Edge",
            ExtrudeAlongSpline { .. } => "Extrude Along Spline",
            CreatePolygon { .. } => "Create",
        }
    }

    /// Ops that can be re-run on a new selection by "Repeat Last".
    pub fn repeatable(&self) -> bool {
        !matches!(
            self,
            PolyOp::TargetWeld { .. }
                | PolyOp::InsertVertexOnEdge { .. }
                | PolyOp::InsertVertexInPolygon { .. }
                | PolyOp::Cut { .. }
                | PolyOp::Transform { .. }
                | PolyOp::SoftTransform { .. }
                | PolyOp::Attach { .. }
                | PolyOp::CreatePolygon { .. }
        )
    }

    /// Applies the op to `m` (reading selection flags). Returns false when it had no effect.
    pub fn apply(&self, m: &mut EditablePoly) -> bool {
        use PolyOp::*;
        match self {
            ExtrudeVertices { height, width } => extrude::extrude_vertices(m, *height, *width),
            ExtrudeEdges { height, width } => extrude::extrude_edges(m, *height, *width),
            ExtrudePolygons { mode, height } => extrude::extrude_polygons(m, *mode, *height),
            Bevel { mode, height, outline } => inset::bevel(m, *mode, *height, *outline),
            Inset { by_polygon, amount } => inset::inset(m, *by_polygon, *amount),
            Outline { amount } => inset::outline(m, *amount),
            ChamferVertices { amount } => chamfer::chamfer_vertices(m, *amount),
            ChamferEdges { amount, segments, tension, open } => chamfer::chamfer_edges_ex(m, *amount, *segments, *tension, *open),
            PanelGroove { width, depth, lip, material_id } => return groove::panel_groove(m, *width, *depth, *lip, *material_id),
            Hole { spec } => return crate::modifiers::cad::hole::drill(m, &m.selected_polygons(), spec).map(|r| *m = r).is_ok(),
            ConnectEdges { segments, pinch, slide } => connect::connect_edges(m, *segments, *pinch, *slide),
            ConnectVertices => connect::connect_vertices(m),
            Weld { level, threshold } => weld::weld(m, *level, *threshold),
            TargetWeld { from, to } => return weld::target_weld(m, *from, *to),
            Collapse { level } => weld::collapse(m, *level),
            RemoveVertices => remove::remove_vertices(m),
            RemoveEdges { remove_verts } => remove::remove_edges(m, *remove_verts),
            BreakVertices => remove::break_vertices(m),
            SplitEdges => remove::split_edges(m),
            Delete { level } => remove::delete_selected(m, *level),
            InsertVertexOnEdge { a, b, t } => {
                if m.edge_between(*a, *b).is_none() {
                    return false;
                }
                let v = insert::insert_vertex_on_edge(m, *a, *b, *t);
                m.clear_selection();
                m.vertices[v].selected = true;
            }
            InsertVertexInPolygon { poly, point } => {
                if !m.polygons.contains_key(*poly) {
                    return false;
                }
                let v = insert::poke_polygon(m, *poly, *point);
                m.vertices[v].selected = true;
            }
            Cut { from, to, view_dir } => {
                let (Some(a), Some(b)) = (resolve_cut_point(m, *from), resolve_cut_point(m, *to)) else {
                    return false;
                };
                insert::cut(m, a, b, *view_dir);
            }
            Slice { point, normal, only_selected, split } => {
                return slice::slice(m, *point, *normal, *only_selected, *split) > 0;
            }
            Cap => return cap_bridge::cap_borders(m) > 0,
            BridgeBorders { segments, taper, twist } => return cap_bridge::bridge_borders(m, *segments, *taper, *twist),
            BridgePolygons { segments, taper, twist } => return cap_bridge::bridge_polygons(m, *segments, *taper, *twist),
            MakePlanar { level } => geometry::make_planar(m, *level),
            Flatten { level, axis } => geometry::flatten_axis(m, *level, *axis),
            Flip => geometry::flip(m),
            SetSmoothing { bit, on } => geometry::set_smoothing_group(m, *bit, *on),
            ClearSmoothing => geometry::clear_smoothing(m),
            AutoSmooth { threshold } => {
                let sel = m.selected_polygons();
                if sel.is_empty() {
                    return false;
                }
                m.auto_smooth(&sel, *threshold);
            }
            SetMaterialId { id } => geometry::set_material_id(m, *id),
            Tessellate { face_center } => geometry::tessellate(m, *face_center),
            MSmooth { strength } => geometry::msmooth(m, *strength),
            Transform { verts, matrix } => geometry::transform_vertices(m, verts, *matrix),
            SoftTransform { weights, matrix } => geometry::soft_transform(m, weights, *matrix),
            ShiftExtrudeEdges => return !extrude::extrude_border_edges(m, Vec3::ZERO).is_empty(),
            Attach { mesh } => {
                m.append(mesh, Mat4::IDENTITY);
            }
            Hinge { origin, axis, angle, segments } => return sweep::hinge(m, *origin, *axis, *angle, *segments),
            ExtrudeAlongSpline { path, segments, taper, twist, align } => {
                return sweep::extrude_along_path(m, path, *segments, *taper, *twist, *align);
            }
            CreatePolygon { points, view_dir } => return sweep::create_polygon(m, points, *view_dir),
        }
        true
    }
}

/// Turns a cut point into a vertex, splitting the edge if necessary.
pub fn resolve_cut_point(m: &mut EditablePoly, p: CutPoint) -> Option<VertexId> {
    match p {
        CutPoint::Vertex(v) => m.vertices.contains_key(v).then_some(v),
        CutPoint::Edge(a, b, t) => {
            m.edge_between(a, b)?;
            Some(split_edge(m, a, b, t))
        }
    }
}
