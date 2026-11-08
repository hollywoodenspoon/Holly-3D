//! Editor state: scene, modes, tools, dialogs and undo history.

pub mod actions;
pub mod drag;
pub mod select;
pub mod tools;
pub mod undo;

use crate::mesh::SubObject;
use crate::mesh::ops::PolyOp;
use crate::primitives::PrimitiveKind;
use crate::scene::{CloneKind, ObjId, Scene, SceneObject, Xform};
use crate::viewport::layout::Viewports;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::Resource;
use undo::History;

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
    TargetWeld { first: Option<crate::mesh::VertexId> },
    InsertVertex,
    Cut { last: Option<crate::mesh::VertexId> },
    QuickSlice { start: Option<Vec2> },
    Attach,
}

impl Tool {
    pub fn label(&self) -> Option<&'static str> {
        match self {
            Tool::None => None,
            Tool::Create(_) => Some("Click in a viewport to place the object"),
            Tool::TargetWeld { first: None } => Some("Target Weld: click the vertex to weld"),
            Tool::TargetWeld { .. } => Some("Target Weld: click the target vertex"),
            Tool::InsertVertex => Some("Insert Vertex: click on an edge or polygon"),
            Tool::Cut { .. } => Some("Cut: click vertices/edges in sequence, right-click to finish"),
            Tool::QuickSlice { start: None } => Some("QuickSlice: click the first point"),
            Tool::QuickSlice { .. } => Some("QuickSlice: click the second point"),
            Tool::Attach => Some("Attach: click an object to merge it"),
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

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub scene: Scene,
    pub sub: SubObject,
}

#[derive(Resource)]
pub struct Editor {
    pub scene: Scene,
    pub viewports: Viewports,
    pub sub: SubObject,
    pub mode: TransformMode,
    pub coord: CoordSystem,
    pub pivot_mode: PivotMode,
    pub snap: bool,
    pub angle_snap: bool,
    pub tab: CommandTab,
    pub tool: Tool,
    pub history: History<Snapshot>,
    pub caddy: Option<Caddy>,
    pub quad_menu: Option<Vec2>,
    pub clone_dialog: Option<CloneDialog>,
    pub last_op: Option<PolyOp>,
    pub slice_plane: Option<SlicePlane>,
    pub ignore_backfacing: bool,
    pub by_vertex: bool,
    pub soft: SoftSelection,
    pub explorer_visible: bool,
    pub explorer_filter: String,
    pub modifier_filter: String,
    pub status: String,
    pub affect_pivot_only: bool,
    pub typein_offset: bool,
    pub set_id: u16,
    pub select_id: u16,
    pub autosmooth_threshold: f32,
    pub drag: Option<drag::Drag>,
    pub isolate: Option<Vec<ObjId>>,
    pub ring_loop_spin: i32,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            scene: Scene::default(),
            viewports: Viewports::default(),
            sub: SubObject::Object,
            mode: TransformMode::Move,
            coord: CoordSystem::World,
            pivot_mode: PivotMode::SelectionCenter,
            snap: false,
            angle_snap: false,
            tab: CommandTab::Create,
            tool: Tool::None,
            history: History::default(),
            caddy: None,
            quad_menu: None,
            clone_dialog: None,
            last_op: None,
            slice_plane: None,
            ignore_backfacing: false,
            by_vertex: false,
            soft: SoftSelection { enabled: false, falloff: 1.0, pinch: 0.0, bubble: 0.0 },
            explorer_visible: true,
            explorer_filter: String::new(),
            modifier_filter: String::new(),
            status: "Welcome to MiniMax — create a primitive from the Create tab.".into(),
            affect_pivot_only: false,
            typein_offset: false,
            set_id: 1,
            select_id: 1,
            autosmooth_threshold: 45.0,
            drag: None,
            isolate: None,
            ring_loop_spin: 1,
        }
    }
}

impl Editor {
    /// The object the Modify panel works on (first selected).
    pub fn current(&self) -> Option<ObjId> {
        self.scene.objects.iter().find(|(_, o)| o.selected).map(|(k, _)| k)
    }

    pub fn current_obj(&self) -> Option<&SceneObject> {
        self.current().map(|k| &self.scene.objects[k])
    }

    pub fn current_obj_mut(&mut self) -> Option<&mut SceneObject> {
        let k = self.current()?;
        self.scene.objects.get_mut(k)
    }

    pub fn in_sub_object(&self) -> bool {
        self.sub != SubObject::Object && self.current_obj().is_some_and(|o| o.sub_object_editable())
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot { scene: self.scene.clone(), sub: self.sub }
    }

    /// Records an undo checkpoint before an edit.
    pub fn checkpoint(&mut self, label: &str) {
        let s = self.snapshot();
        self.history.push(label, s);
    }

    pub fn undo(&mut self) {
        let cur = self.snapshot();
        match self.history.undo(cur) {
            Ok((label, s)) => {
                self.restore(s);
                self.status = format!("Undo {label}");
            }
            Err(_) => self.status = "Nothing to undo".into(),
        }
    }

    pub fn redo(&mut self) {
        let cur = self.snapshot();
        match self.history.redo(cur) {
            Ok((label, s)) => {
                self.restore(s);
                self.status = format!("Redo {label}");
            }
            Err(_) => self.status = "Nothing to redo".into(),
        }
    }

    fn restore(&mut self, s: Snapshot) {
        self.scene = s.scene;
        self.sub = s.sub;
        self.caddy = None;
        self.drag = None;
        for o in self.scene.objects.values_mut() {
            o.version += 1;
            o.selection_version += 1;
        }
    }

    /// World-space center of the current selection (objects or sub-objects).
    pub fn selection_center(&self) -> Option<Vec3> {
        if self.in_sub_object() {
            let o = self.current_obj()?;
            let vs = o.level_mesh.selection_vertices(self.sub);
            if vs.is_empty() {
                return None;
            }
            let w = o.world_matrix();
            let sum: Vec3 = vs.iter().map(|&v| w.transform_point3(o.level_mesh.pos(v))).sum();
            return Some(sum / vs.len() as f32);
        }
        let b = self.selection_bounds()?;
        Some(b.center())
    }

    /// World bounds of selected objects (or sub-objects); whole scene when empty.
    pub fn selection_bounds(&self) -> Option<crate::mesh::Aabb> {
        let world_pts = |o: &SceneObject, verts: Vec<crate::mesh::VertexId>| -> Vec<Vec3> {
            let w = o.world_matrix();
            verts.iter().map(|&v| w.transform_point3(o.level_mesh.pos(v))).collect()
        };
        if self.in_sub_object() {
            let o = self.current_obj()?;
            let vs = o.level_mesh.selection_vertices(self.sub);
            if !vs.is_empty() {
                return crate::mesh::Aabb::from_points(world_pts(o, vs));
            }
        }
        let sel: Vec<&SceneObject> = self.scene.objects.values().filter(|o| o.selected && !o.hidden).collect();
        let objs: Vec<&SceneObject> = if sel.is_empty() { self.scene.objects.values().filter(|o| !o.hidden).collect() } else { sel };
        let mut pts = vec![];
        for o in objs {
            let w = o.world_matrix();
            pts.extend(o.evaluated_mesh.vertices.values().map(|v| w.transform_point3(v.position)));
        }
        crate::mesh::Aabb::from_points(pts)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::primitives::Primitive;
    use crate::scene::BaseObject;

    /// Editable Poly box, selected, with its top polygon selected.
    pub fn top_selected_box() -> Scene {
        let mut s = Scene::default();
        let id = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::ZERO);
        let o = &mut s.objects[id];
        o.collapse_to_editable_poly();
        o.selected = true;
        let m = &mut o.level_mesh;
        let top: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).z > 0.9).collect();
        m.polygons[top[0]].selected = true;
        o.commit_selection();
        s
    }
}
