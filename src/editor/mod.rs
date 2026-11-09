//! Editor state: scene, modes, tools, dialogs and undo history.

pub mod actions;
pub mod array;
#[cfg(test)]
mod array_tests;
pub mod boolean;
pub mod drag;
pub mod file;
pub mod lights;
pub mod materials;
pub mod mirror;
pub mod modsub;
pub mod normals;
pub mod paint;
pub mod render;
pub mod select;
pub mod splines;
pub mod tools;
pub mod types;
pub mod undo;

use crate::mesh::SubObject;
use crate::mesh::ops::PolyOp;
use crate::primitives::PrimitiveKind;
use crate::scene::{CloneKind, ObjId, Scene, SceneObject, Xform};
use crate::viewport::layout::Viewports;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::Resource;
pub use types::*;
use undo::History;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub scene: Scene,
    pub sub: SubObject,
}

/// Variant name of an op ("ExtrudePolygons", …) used as memory key.
pub fn op_key(op: &PolyOp) -> String {
    let d = format!("{op:?}");
    d.split([' ', '{', '(']).next().unwrap_or("").to_string()
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
    pub array_settings: array::ArraySettings,
    pub array_dialog: Option<array::ArrayDialog>,
    pub mirror_settings: mirror::MirrorSettings,
    pub mirror_dialog: Option<mirror::MirrorDialog>,
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
    /// Last-used parameters per op kind (main buttons reuse them).
    pub op_memory: std::collections::HashMap<String, PolyOp>,
    /// FFD Control Points / Lattice or UVW Map Gizmo sub-object mode.
    pub mod_sub: Option<crate::modifiers::ModSub>,
    pub uv_editor_open: bool,
    pub mat_editor: MatEditor,
    /// Ctrl+L: force the Default Modeling Lights rig even with scene lights.
    pub default_lights: bool,
    /// Environment & Effects dialog (`8`).
    pub env_open: bool,
    /// Viewport Paint / PaintBox settings.
    pub paint: crate::paint::PaintSettings,
    pub paint_stroke: Option<paint::PaintStroke>,
    /// Render Setup (F10) parameters.
    pub render: crate::render::RenderSettings,
    /// Rendered Frame Window (F9) + running job.
    pub rfw: render::RenderUi,
    /// Editable Spline tool spinners (Fillet, Chamfer, Weld, Divide, Outline…).
    pub spline_ui: SplineUi,
    /// Current `.mmax` file (Save writes here; `None` = Save As).
    pub scene_path: Option<std::path::PathBuf>,
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
            array_settings: Default::default(),
            array_dialog: None,
            mirror_settings: Default::default(),
            mirror_dialog: None,
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
            op_memory: Default::default(),
            mod_sub: None,
            uv_editor_open: false,
            mat_editor: MatEditor::default(),
            default_lights: false,
            env_open: false,
            paint: Default::default(),
            paint_stroke: None,
            render: Default::default(),
            rfw: Default::default(),
            spline_ui: Default::default(),
            scene_path: None,
        }
    }
}

impl Editor {
    /// Op with remembered parameters (or `default`).
    pub fn remembered(&self, default: PolyOp) -> PolyOp {
        self.op_memory.get(&op_key(&default)).cloned().unwrap_or(default)
    }

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
            o.version = crate::scene::next_version();
            o.selection_version = crate::scene::next_version();
        }
    }

    /// World-space center of the current selection (objects or sub-objects).
    pub fn selection_center(&self) -> Option<Vec3> {
        if self.spline_level().is_some() {
            let pts = self.spline_selection_world();
            return (!pts.is_empty()).then(|| pts.iter().sum::<Vec3>() / pts.len() as f32);
        }
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
            if o.is_helper() {
                pts.push(o.transform.translation);
                continue;
            }
            let w = o.world_matrix();
            pts.extend(o.evaluated_mesh.vertices.values().map(|v| w.transform_point3(v.position)));
            if let Some(sh) = &o.shape {
                pts.extend(sh.polylines().into_iter().flat_map(|pl| pl.pts).map(|p| w.transform_point3(p)));
            }
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
