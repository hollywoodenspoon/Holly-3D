//! Modal viewport tools: Create, Target Weld, Insert Vertex, Cut, QuickSlice, Attach.

use super::select::{EDGE_PICK_PX, VERTEX_PICK_PX};
use super::*;
use crate::mesh::ops::dispatch::CutPoint;
use crate::viewport::picking::{pick_edge, pick_polygon, pick_vertex};

impl Editor {
    /// Handles a left click while a tool is active. Returns true if consumed.
    pub fn tool_click(&mut self, view: usize, cursor: Vec2) -> bool {
        let tool = self.tool.clone();
        let v = self.viewports.views[view].clone();
        match tool {
            Tool::None => return false,
            Tool::Create(kind) => {
                let p = v.construction_point(cursor);
                let p = if self.snap { (p * 4.0).round() / 4.0 } else { p };
                self.create_primitive(kind, p);
                self.tool = Tool::Create(kind);
            }
            Tool::CreateGeoGen(mode) => {
                let p = v.construction_point(cursor);
                let p = if self.snap { (p * 4.0).round() / 4.0 } else { p };
                self.create_geogen(mode, p);
                self.tool = Tool::CreateGeoGen(mode);
            }
            Tool::NormalAlign => return self.normal_align_click(view, cursor),
            Tool::CreateShape(kind) => {
                let p = self.snapped_point_pub(view, cursor);
                self.create_shape(kind, view, p);
                self.tool = Tool::CreateShape(kind);
            }
            // handled on press / drag / release by the viewport
            Tool::Line { .. } => {}
            Tool::CreateLight { kind, first } => self.create_light_click(view, cursor, kind, first),
            Tool::PickMaterial => {
                if let Some(other) = self.pick_object(view, cursor) {
                    self.pick_material_from(other);
                }
                self.tool = Tool::None;
            }
            Tool::Attach => {
                if let Some(other) = self.pick_object(view, cursor) {
                    self.attach(other);
                }
                self.tool = Tool::None;
            }
            Tool::TargetWeld { first } => {
                let Some(vid) = self.pick_cursor_vertex(view, cursor) else {
                    return true;
                };
                match first {
                    None => self.tool = Tool::TargetWeld { first: Some(vid) },
                    Some(a) => {
                        self.apply_op(PolyOp::TargetWeld { from: a, to: vid });
                        self.tool = Tool::TargetWeld { first: None };
                    }
                }
            }
            Tool::InsertVertex => {
                if self.sub.is_face_like() {
                    let Some(o) = self.current_obj() else {
                        return true;
                    };
                    if let Some(h) = pick_polygon(&o.level_mesh, o.world_matrix(), &v.ray(cursor), true) {
                        let local = o.world_to_mesh().transform_point3(h.point);
                        self.apply_op(PolyOp::InsertVertexInPolygon { poly: h.poly, point: local });
                    }
                } else if let Some(CutPoint::Edge(a, b, t)) = self.pick_cut_point(view, cursor, false) {
                    self.apply_op(PolyOp::InsertVertexOnEdge { a, b, t });
                }
            }
            Tool::Cut { last } => {
                let Some(pt) = self.pick_cut_point(view, cursor, true) else {
                    return true;
                };
                let world_pos = self.cut_point_world(pt);
                match last {
                    None => {
                        let vid = match pt {
                            CutPoint::Vertex(v) => Some(v),
                            CutPoint::Edge(a, b, t) => {
                                self.apply_op(PolyOp::InsertVertexOnEdge { a, b, t });
                                self.nearest_vertex(world_pos)
                            }
                        };
                        self.tool = Tool::Cut { last: vid };
                    }
                    Some(from) => {
                        let o = self.current_obj().unwrap();
                        let view_dir = o.world_to_mesh().transform_vector3(v.ray(cursor).dir).normalize_or_zero();
                        self.apply_op(PolyOp::Cut { from: CutPoint::Vertex(from), to: pt, view_dir });
                        self.tool = Tool::Cut { last: self.nearest_vertex(world_pos) };
                    }
                }
            }
            Tool::QuickSlice { start } => match start {
                None => self.tool = Tool::QuickSlice { start: Some(cursor) },
                Some(s) => {
                    self.quick_slice(view, s, cursor);
                    self.tool = Tool::QuickSlice { start: None };
                }
            },
        }
        true
    }

    /// Right-click / Esc ends the current tool.
    pub fn cancel_tool(&mut self) -> bool {
        if self.paint.active {
            self.toggle_paint();
            return true;
        }
        if self.tool == Tool::None {
            return false;
        }
        if matches!(self.tool, Tool::Line { .. }) && self.finish_line(false) {
            return true;
        }
        self.tool = Tool::None;
        true
    }

    fn pick_cursor_vertex(&self, view: usize, cursor: Vec2) -> Option<crate::mesh::VertexId> {
        let o = self.current_obj()?;
        let v = &self.viewports.views[view];
        let project = |p: Vec3| v.to_screen(p);
        pick_vertex(&o.level_mesh, o.world_matrix(), &project, cursor, VERTEX_PICK_PX, &|_| true)
    }

    fn pick_cut_point(&self, view: usize, cursor: Vec2, vertices: bool) -> Option<CutPoint> {
        if vertices && let Some(v) = self.pick_cursor_vertex(view, cursor) {
            return Some(CutPoint::Vertex(v));
        }
        let o = self.current_obj()?;
        let v = &self.viewports.views[view];
        let project = |p: Vec3| v.to_screen(p);
        let (e, t) = pick_edge(&o.level_mesh, o.world_matrix(), &project, cursor, EDGE_PICK_PX * 1.5, &|_| true)?;
        let ed = &o.level_mesh.edges[e];
        Some(CutPoint::Edge(ed.v0, ed.v1, t.clamp(0.02, 0.98)))
    }

    fn cut_point_world(&self, p: CutPoint) -> Vec3 {
        let Some(o) = self.current_obj() else {
            return Vec3::ZERO;
        };
        let m = &o.level_mesh;
        let local = match p {
            CutPoint::Vertex(v) => m.pos(v),
            CutPoint::Edge(a, b, t) => m.pos(a).lerp(m.pos(b), t),
        };
        o.world_matrix().transform_point3(local)
    }

    fn nearest_vertex(&self, world: Vec3) -> Option<crate::mesh::VertexId> {
        let o = self.current_obj()?;
        let local = o.world_to_mesh().transform_point3(world);
        o.level_mesh.vertices.iter().min_by(|a, b| a.1.position.distance(local).total_cmp(&b.1.position.distance(local))).map(|x| x.0)
    }

    /// QuickSlice: plane through the two screen points along the view direction.
    pub fn quick_slice(&mut self, view: usize, a: Vec2, b: Vec2) {
        let v = &self.viewports.views[view];
        let (ra, rb) = (v.ray(a), v.ray(b));
        let normal = if v.camera.is_ortho() { (rb.origin - ra.origin).cross(ra.dir) } else { ra.dir.cross(rb.dir) };
        if normal.length_squared() < 1e-12 {
            return;
        }
        let Some(o) = self.current_obj() else { return };
        let inv = o.world_to_mesh();
        let point = inv.transform_point3(ra.origin);
        let n_local = o.world_matrix().transpose().transform_vector3(normal).normalize();
        let only = self.sub.is_face_like();
        self.apply_op(PolyOp::Slice { point, normal: n_local, only_selected: only, split: false });
    }

    /// `[ Slice ]` with the interactive Slice Plane.
    pub fn slice_with_plane(&mut self) {
        let Some(sp) = self.slice_plane.clone() else {
            return;
        };
        let Some(o) = self.current_obj() else { return };
        let m = sp.xform.matrix();
        let (pw, nw) = (m.transform_point3(Vec3::ZERO), m.transform_vector3(Vec3::Z));
        let point = o.world_to_mesh().transform_point3(pw);
        let normal = o.world_matrix().transpose().transform_vector3(nw).normalize();
        let only = self.sub.is_face_like();
        self.apply_op(PolyOp::Slice { point, normal, only_selected: only, split: sp.split });
    }

    /// Toggles the Slice Plane gizmo, centered on the current object.
    pub fn toggle_slice_plane(&mut self) {
        if self.slice_plane.take().is_some() {
            return;
        }
        let center = self.current_obj().and_then(|o| {
            let b = o.evaluated_mesh.bounds()?;
            Some(o.world_matrix().transform_point3(b.center()))
        });
        self.slice_plane =
            Some(SlicePlane { xform: Xform { translation: center.unwrap_or(Vec3::ZERO), ..Default::default() }, split: false });
    }

    pub fn reset_slice_plane(&mut self) {
        if self.slice_plane.is_some() {
            let split = self.slice_plane.as_ref().unwrap().split;
            self.slice_plane = None;
            self.toggle_slice_plane();
            if let Some(sp) = self.slice_plane.as_mut() {
                sp.split = split;
            }
        }
    }
}
