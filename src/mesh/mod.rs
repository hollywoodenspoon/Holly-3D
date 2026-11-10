//! Editable Poly mesh core: index/slotmap based face-corner topology.
//!
//! 3D vertex positions are decoupled from per-face-corner attributes (UV,
//! explicit normals) exactly like 3ds Max, so smoothing groups, hard edges and
//! UV seams never require physically duplicating vertices.

pub mod adjacency;
pub mod build;
pub mod merge;
pub mod normals;
pub mod ops;
pub mod selection;
pub mod selection_shift;
pub mod triangulate;
pub mod validate;

use bevy::math::{Vec2, Vec3, Vec4};
use slotmap::{SlotMap, new_key_type};
use std::collections::HashMap;

new_key_type! {
    pub struct VertexId;
    pub struct EdgeId;
    pub struct PolyId;
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vertex {
    pub position: Vec3,
    /// RGBA for VertexPaint / direct model painting without UVs.
    pub color: Vec4,
    pub selected: bool,
}

impl Vertex {
    pub fn new(position: Vec3) -> Self {
        Self { position, color: Vec4::ONE, selected: false }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FaceCorner {
    pub vertex_id: VertexId,
    pub uv: Vec2,
    /// Set by Edit Normals; if `None`, computed from `smoothing_group`.
    pub explicit_normal: Option<Vec3>,
}

impl FaceCorner {
    pub fn new(vertex_id: VertexId, uv: Vec2) -> Self {
        Self { vertex_id, uv, explicit_normal: None }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Polygon {
    /// Triangles, quads and N-gons (>= 3 corners), counter-clockwise seen from outside.
    pub corners: Vec<FaceCorner>,
    /// 32-bit bitmask of 3ds Max Smoothing Groups 1..=32.
    pub smoothing_group: u32,
    pub material_id: u16,
    pub selected: bool,
}

impl Polygon {
    pub fn new(corners: Vec<FaceCorner>) -> Self {
        Self { corners, smoothing_group: 0, material_id: 1, selected: false }
    }

    pub fn vertex_ids(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.corners.iter().map(|c| c.vertex_id)
    }

    /// Directed edges (a -> b) of this polygon in winding order.
    pub fn half_edges(&self) -> impl Iterator<Item = (VertexId, VertexId)> + '_ {
        let n = self.corners.len();
        (0..n).map(move |i| (self.corners[i].vertex_id, self.corners[(i + 1) % n].vertex_id))
    }

    pub fn position_of(&self, v: VertexId) -> Option<usize> {
        self.corners.iter().position(|c| c.vertex_id == v)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    pub v0: VertexId,
    pub v1: VertexId,
    pub creasing: f32,
    pub selected: bool,
    /// Invisible in wireframe (TurboSmooth isoline display).
    pub hidden: bool,
}

impl Edge {
    pub fn other(&self, v: VertexId) -> VertexId {
        if self.v0 == v { self.v1 } else { self.v0 }
    }
}

/// Canonical (order independent) key for an undirected edge.
pub fn edge_key(a: VertexId, b: VertexId) -> (VertexId, VertexId) {
    if a < b { (a, b) } else { (b, a) }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EditablePoly {
    pub vertices: SlotMap<VertexId, Vertex>,
    pub edges: SlotMap<EdgeId, Edge>,
    pub polygons: SlotMap<PolyId, Polygon>,
    /// Lookup from canonical vertex pair to edge; maintained by `rebuild_edges`.
    edge_map: HashMap<(VertexId, VertexId), EdgeId>,
}

/// Which sub-object level an operation or selection refers to (3ds Max `1`..`5`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum SubObject {
    #[default]
    Object,
    Vertex,
    Edge,
    Border,
    Polygon,
    Element,
}

impl SubObject {
    pub fn from_key(n: u8) -> Self {
        match n {
            1 => Self::Vertex,
            2 => Self::Edge,
            3 => Self::Border,
            4 => Self::Polygon,
            5 => Self::Element,
            _ => Self::Object,
        }
    }
    pub fn number(self) -> u8 {
        match self {
            Self::Object => 0,
            Self::Vertex => 1,
            Self::Edge => 2,
            Self::Border => 3,
            Self::Polygon => 4,
            Self::Element => 5,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Object => "Object",
            Self::Vertex => "Vertex",
            Self::Edge => "Edge",
            Self::Border => "Border",
            Self::Polygon => "Polygon",
            Self::Element => "Element",
        }
    }
    pub fn is_edge_like(self) -> bool {
        matches!(self, Self::Edge | Self::Border)
    }
    pub fn is_face_like(self) -> bool {
        matches!(self, Self::Polygon | Self::Element)
    }
}

/// Snapshot of the selection flags (used by the Edit Poly modifier).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SelectionSets {
    pub vertices: Vec<VertexId>,
    pub edges: Vec<(VertexId, VertexId)>,
    pub polygons: Vec<PolyId>,
}

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        let mut it = points.into_iter();
        let first = it.next()?;
        let mut b = Aabb { min: first, max: first };
        for p in it {
            b.min = b.min.min(p);
            b.max = b.max.max(p);
        }
        Some(b)
    }
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
    pub fn union(&self, o: &Aabb) -> Aabb {
        Aabb { min: self.min.min(o.min), max: self.max.max(o.max) }
    }
}
