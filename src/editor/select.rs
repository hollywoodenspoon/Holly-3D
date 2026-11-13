//! Click and marquee (window) selection for objects and sub-objects.
//! Ctrl adds, Alt subtracts.

use super::*;
use crate::mesh::adjacency::Adjacency;
use crate::viewport::picking::{pick_edge, pick_polygon, pick_vertex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectOp {
    Replace,
    Add,
    Subtract,
}

pub const VERTEX_PICK_PX: f32 = 9.0;
pub const EDGE_PICK_PX: f32 = 7.0;
/// Click radius around a light / target icon.
pub const HELPER_PICK_PX: f32 = 12.0;

impl Editor {
    pub(super) fn selectable(&self) -> impl Iterator<Item = (ObjId, &SceneObject)> {
        let f = self.iface.select_filter;
        self.scene.objects.iter().filter(move |(_, o)| !o.hidden && !o.frozen && f.accepts(o))
    }

    pub fn pick_object(&self, view: usize, cursor: Vec2) -> Option<ObjId> {
        let v = &self.viewports.views[view];
        // Light and target icons win over geometry behind them.
        let helper = self
            .selectable()
            .filter(|(_, o)| o.is_helper())
            .filter_map(|(k, o)| v.to_screen(o.transform.translation).map(|s| (k, s.distance(cursor))))
            .filter(|(_, d)| *d <= HELPER_PICK_PX)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((k, _)) = helper {
            return Some(k);
        }
        // Live splines (shapes not yet lathed / extruded) pick by screen distance.
        let spline = self
            .selectable()
            .filter_map(|(k, o)| self.shape_screen_distance(view, o, cursor).map(|d| (k, d)))
            .filter(|(_, d)| *d <= EDGE_PICK_PX)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let ray = v.ray(cursor);
        let hit = self
            .selectable()
            .filter_map(|(k, o)| pick_polygon(&o.evaluated_mesh, o.world_matrix(), &ray, false).map(|h| (k, h.distance)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|x| x.0);
        hit.or(spline.map(|x| x.0)).or_else(|| {
            // wireframe-friendly fallback: nearest edge on screen
            let project = |p: Vec3| v.to_screen(p);
            self.selectable()
                .find(|(_, o)| pick_edge(&o.evaluated_mesh, o.world_matrix(), &project, cursor, EDGE_PICK_PX, &|_| true).is_some())
                .map(|x| x.0)
        })
    }

    fn vertex_visible(&self, view: usize, o: &SceneObject) -> impl Fn(crate::mesh::VertexId) -> bool + use<> {
        let cam = self.viewports.views[view].camera;
        let normals = if self.ignore_backfacing { Some(o.level_mesh.vertex_normals()) } else { None };
        let w = o.world_matrix();
        let positions: std::collections::HashMap<_, _> =
            o.level_mesh.vertices.iter().map(|(k, v)| (k, w.transform_point3(v.position))).collect();
        let nmat = w.inverse().transpose();
        move |id| {
            let Some(n) = normals.as_ref().and_then(|ns| ns.get(&id)) else {
                return true;
            };
            let p = positions[&id];
            let to_eye = if cam.is_ortho() { -cam.forward() } else { cam.eye() - p };
            nmat.transform_vector3(*n).dot(to_eye) > 0.0
        }
    }

    pub fn click_select(&mut self, view: usize, cursor: Vec2, op: SelectOp) {
        if self.iface.lock_selection {
            self.status = "Selection is locked (Space)".into();
            return;
        }
        if self.mod_click_select(view, cursor, op) || self.spline_click_select(view, cursor, op) {
            return;
        }
        if !self.in_sub_object() {
            let hit = self.pick_object(view, cursor);
            match (op, hit) {
                (SelectOp::Replace, h) => self.scene.select_only(h),
                (SelectOp::Add, Some(h)) => self.scene.objects[h].selected = true,
                (SelectOp::Subtract, Some(h)) => self.scene.objects[h].selected = false,
                _ => {}
            }
            if let Some(o) = self.current_obj_mut()
                && op == SelectOp::Replace
                && o.active_level.is_none()
                && !o.modifiers.is_empty()
            {
                let top = o.modifiers.len() - 1;
                o.set_active_level(Some(top));
            }
            return;
        }
        let Some(id) = self.current() else { return };
        let v = self.viewports.views[view].clone();
        let sub = self.sub;
        let o = &self.scene.objects[id];
        let w = o.world_matrix();
        let project = |p: Vec3| v.to_screen(p);
        let m = &o.level_mesh;
        enum Pick {
            V(crate::mesh::VertexId),
            E(crate::mesh::EdgeId),
            P(crate::mesh::PolyId),
            Nothing,
        }
        let pick = match sub {
            SubObject::Vertex => {
                let vis = self.vertex_visible(view, o);
                pick_vertex(m, w, &project, cursor, VERTEX_PICK_PX, &vis).map(Pick::V)
            }
            SubObject::Edge | SubObject::Border => {
                let adj = Adjacency::build(m);
                let border = sub == SubObject::Border;
                pick_edge(m, w, &project, cursor, EDGE_PICK_PX, &|e| !border || adj.is_open_edge(m.edges[e].v0, m.edges[e].v1))
                    .map(|x| Pick::E(x.0))
            }
            _ => pick_polygon(m, w, &v.ray(cursor), self.ignore_backfacing).map(|h| Pick::P(h.poly)),
        }
        .unwrap_or(Pick::Nothing);
        let by_vertex = self.by_vertex && sub != SubObject::Vertex;
        let vertex_hit = if by_vertex { pick_vertex(m, w, &project, cursor, VERTEX_PICK_PX, &|_| true) } else { None };
        self.edit_selection(|m, sub| {
            let before = m.clone();
            if op == SelectOp::Replace {
                m.clear_selection();
            }
            let on = op != SelectOp::Subtract;
            match pick {
                Pick::V(k) => m.vertices[k].selected = on,
                Pick::E(k) => m.edges[k].selected = on,
                Pick::P(k) => m.polygons[k].selected = on,
                Pick::Nothing => {}
            }
            if let Some(vh) = vertex_hit {
                // By Vertex: select everything at this level touching the vertex
                for e in m.edges.values_mut().filter(|e| e.v0 == vh || e.v1 == vh) {
                    e.selected = on;
                }
                for p in m.polygons.values_mut().filter(|p| p.vertex_ids().any(|x| x == vh)) {
                    p.selected = on;
                }
            }
            expand_level(m, sub, on, &before);
        });
    }

    /// `Q`: Select Object; pressed again in Select mode, the next region shape.
    pub fn select_mode_or_cycle_region(&mut self) {
        if self.mode == TransformMode::Select {
            self.region_shape = self.region_shape.next();
        } else {
            self.mode = TransformMode::Select;
        }
        self.status = self.region_shape.label().into();
    }

    /// Rectangular marquee from corner `a` to `b`.
    pub fn marquee_select(&mut self, view: usize, a: Vec2, b: Vec2, op: SelectOp) {
        self.region_select(view, &crate::viewport::region::Region::rect(a, b), op);
    }

    /// Window selection with any region shape (Rectangle / Circle / Fence / Lasso).
    pub fn region_select(&mut self, view: usize, region: &crate::viewport::region::Region, op: SelectOp) {
        if self.iface.lock_selection {
            self.status = "Selection is locked (Space)".into();
            return;
        }
        let window = self.iface.window_select;
        if self.mod_marquee_select(view, region, op) || self.spline_marquee_select(view, region, op) {
            return;
        }
        let v = self.viewports.views[view].clone();
        if !self.in_sub_object() {
            let inside: Vec<ObjId> = self
                .selectable()
                .filter(|(_, o)| {
                    if o.is_helper() {
                        return v.to_screen(o.transform.translation).is_some_and(|s| region.contains(s));
                    }
                    let w = o.world_matrix();
                    let shape_pts = o.shape.iter().flat_map(|s| s.polylines()).flat_map(|pl| pl.pts);
                    let mut pts = o.evaluated_mesh.vertices.values().map(|p| p.position).chain(shape_pts).peekable();
                    let hit = |p: Vec3| v.to_screen(w.transform_point3(p)).is_some_and(|s| region.contains(s));
                    // Window: the whole object is inside; Crossing: any part touches.
                    if window { pts.peek().is_some() && pts.all(hit) } else { pts.any(hit) }
                })
                .map(|x| x.0)
                .collect();
            if op == SelectOp::Replace {
                self.scene.select_only(None);
            }
            for k in inside {
                self.scene.objects[k].selected = op != SelectOp::Subtract;
            }
            return;
        }
        let Some(id) = self.current() else { return };
        let o = &self.scene.objects[id];
        let w = o.world_matrix();
        let vis = self.vertex_visible(view, o);
        let screen: std::collections::HashMap<_, _> = o
            .level_mesh
            .vertices
            .iter()
            .filter(|(k, _)| vis(*k))
            .filter_map(|(k, p)| v.to_screen(w.transform_point3(p.position)).map(|s| (k, s)))
            .collect();
        let inside = |k: &crate::mesh::VertexId| screen.get(k).is_some_and(|s| region.contains(*s));
        let m = &o.level_mesh;
        let sel_v: Vec<_> = m.vertices.keys().filter(|k| inside(k)).collect();
        let sel_e: Vec<_> = m
            .edges
            .iter()
            .filter(|(_, e)| if window { inside(&e.v0) && inside(&e.v1) } else { inside(&e.v0) || inside(&e.v1) })
            .map(|x| x.0)
            .collect();
        let sel_p: Vec<_> = m
            .polygons
            .iter()
            .filter(|(_, p)| if window { p.vertex_ids().all(|x| inside(&x)) } else { p.vertex_ids().any(|x| inside(&x)) })
            .map(|x| x.0)
            .collect();
        self.edit_selection(|m, sub| {
            let before = m.clone();
            if op == SelectOp::Replace {
                m.clear_selection();
            }
            let on = op != SelectOp::Subtract;
            match sub {
                SubObject::Vertex => sel_v.iter().for_each(|&k| m.vertices[k].selected = on),
                SubObject::Edge | SubObject::Border => sel_e.iter().for_each(|&k| m.edges[k].selected = on),
                _ => sel_p.iter().for_each(|&k| m.polygons[k].selected = on),
            }
            expand_level(m, sub, on, &before);
        });
    }
}

/// Border clicks select the whole loop; Element clicks select the whole island.
fn expand_level(m: &mut crate::mesh::EditablePoly, sub: SubObject, on: bool, before: &crate::mesh::EditablePoly) {
    match sub {
        SubObject::Border if on => m.select_borders(),
        SubObject::Element if on => m.select_elements(),
        SubObject::Element => {
            // subtract whole elements that lost a polygon
            let adj = Adjacency::build(m);
            for el in adj.elements(m) {
                if el.iter().any(|&p| !m.polygons[p].selected && before.polygons.get(p).is_some_and(|q| q.selected)) {
                    el.iter().for_each(|&p| m.polygons[p].selected = false);
                }
            }
        }
        _ => {}
    }
}
