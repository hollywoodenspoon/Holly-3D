//! Shapes in the editor: Create -> Shapes (click-placed parametric shapes
//! and the Line tool), Convert to Editable Spline, and Editable Spline
//! sub-object selection (1 Vertex, 2 Segment, 3 Spline) and editing.

use super::select::{EDGE_PICK_PX, SelectOp, VERTEX_PICK_PX};
use super::*;
use crate::scene::BaseObject;
use crate::splines::edit::SplineLevel;
use crate::splines::{Knot, KnotType, Shape, ShapeKind, ShapeObject, ShapeParams, ShapeSource, Spline, bezier};
use crate::viewport::picking::{in_rect, point_segment};
use bevy::math::{Mat3, Mat4, Quat};

impl Editor {
    /// Spline sub-object level in effect (Editable Spline base, keys 1–3).
    pub fn spline_level(&self) -> Option<SplineLevel> {
        if !self.editable_spline_selected() {
            return None;
        }
        match self.sub {
            SubObject::Vertex => Some(SplineLevel::Vertex),
            SubObject::Edge => Some(SplineLevel::Segment),
            SubObject::Border => Some(SplineLevel::Spline),
            _ => None,
        }
    }

    /// Current object is an Editable Spline with the base level active.
    pub fn editable_spline_selected(&self) -> bool {
        self.current_obj().is_some_and(|o| {
            o.active_level.is_none() && matches!(o.shape_object(), Some(ShapeObject { source: ShapeSource::Editable(_), .. }))
        })
    }

    /// Rotation that maps a shape's local XY plane onto the view's construction plane.
    pub fn view_plane_rotation(&self, view: usize) -> Quat {
        let cam = &self.viewports.views[view].camera;
        if !matches!(cam.kind, crate::viewport::camera::ViewKind::Front | crate::viewport::camera::ViewKind::Left) {
            return Quat::IDENTITY;
        }
        let snap = |v: Vec3| {
            let a = v.abs();
            let i = if a.x >= a.y && a.x >= a.z {
                0
            } else if a.y >= a.z {
                1
            } else {
                2
            };
            let mut s = Vec3::ZERO;
            s[i] = v[i].signum();
            s
        };
        let (r, u) = (snap(cam.right()), snap(cam.up()));
        Quat::from_mat3(&Mat3::from_cols(r, u, r.cross(u)))
    }

    fn add_shape(&mut self, so: ShapeObject, at: Vec3, rot: Quat) -> ObjId {
        let id = self.scene.add(BaseObject::Shape(so), at);
        let o = &mut self.scene.objects[id];
        o.transform.rotation = rot;
        o.wire_color = [0.35, 0.72, 0.98];
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {}", self.scene.objects[id].name);
        id
    }

    pub fn create_shape(&mut self, kind: ShapeKind, view: usize, at: Vec3) {
        let Some(p) = ShapeParams::default_of(kind) else { return };
        self.checkpoint("Create Shape");
        let rot = self.view_plane_rotation(view);
        self.add_shape(ShapeObject::param(p), at, rot);
    }

    pub fn snapped_point_pub(&self, view: usize, cursor: Vec2) -> Vec3 {
        let p = self.viewports.views[view].construction_point(cursor);
        if self.snap { (p * 4.0).round() / 4.0 } else { p }
    }

    /// Line tool, mouse down: closes the spline on its first knot, else adds a knot.
    pub fn line_press(&mut self, view: usize, cursor: Vec2) {
        let p = self.snapped_point_pub(view, cursor);
        let v = &self.viewports.views[view];
        let Tool::Line { knots, view: lv, dragging } = &mut self.tool else { return };
        if knots.is_empty() {
            *lv = view;
        }
        let near_first =
            knots.len() >= 2 && knots[0].pos != p && v.to_screen(knots[0].pos).is_some_and(|s| s.distance(cursor) < VERTEX_PICK_PX);
        if near_first {
            self.finish_line(true);
            return;
        }
        knots.push(Knot::corner(p));
        *dragging = true;
    }

    /// Line tool, drag: pulls Bezier handles on the knot just placed.
    pub fn line_drag(&mut self, view: usize, cursor: Vec2) {
        let p = self.snapped_point_pub(view, cursor);
        if let Tool::Line { knots, dragging: true, .. } = &mut self.tool
            && let Some(k) = knots.last_mut()
        {
            let d = p - k.pos;
            if d.length() > 1e-3 {
                *k = Knot::bezier(k.pos, k.pos - d, k.pos + d);
            }
        }
    }

    pub fn line_release(&mut self) {
        if let Tool::Line { dragging, .. } = &mut self.tool {
            *dragging = false;
        }
    }

    /// Ends the current Line (right-click or click on the first knot).
    /// Returns false when there was nothing to finish.
    pub fn finish_line(&mut self, close: bool) -> bool {
        let Tool::Line { knots, view, .. } = &mut self.tool else { return false };
        let (knots, view) = (std::mem::take(knots), *view);
        if knots.is_empty() {
            return false;
        }
        if knots.len() < 2 {
            return true;
        }
        let rot = self.view_plane_rotation(view);
        let center = crate::mesh::Aabb::from_points(knots.iter().map(|k| k.pos)).map_or(Vec3::ZERO, |b| b.center());
        let to_local = (Mat4::from_translation(center) * Mat4::from_quat(rot)).inverse();
        let mut spline = Spline { knots, closed: close, selected: false };
        spline.knots.iter_mut().for_each(|k| k.transform(to_local));
        self.checkpoint("Create Line");
        let tool = self.tool.clone();
        let id = self.add_shape(ShapeObject::editable(Shape { splines: vec![spline], steps: 6 }), center, rot);
        let name = self.scene.unique_name("Line");
        self.status = format!("Created {name}");
        self.scene.objects[id].name = name;
        self.tool = tool;
        if close {
            self.status = "Spline closed".into();
        }
        true
    }

    pub fn convert_to_editable_spline(&mut self) {
        let ids: Vec<ObjId> = self.scene.selected().into_iter().filter(|&k| self.scene.objects[k].shape_object().is_some()).collect();
        if ids.is_empty() {
            self.status = "Convert to Editable Spline needs a shape".into();
            return;
        }
        self.checkpoint("Convert to Editable Spline");
        for id in ids {
            let o = &mut self.scene.objects[id];
            if let Some(so) = o.shape_object_mut() {
                so.make_editable();
            }
            o.evaluate();
        }
    }

    /// Edits the current Editable Spline's shape with undo.
    pub fn edit_shape(&mut self, label: &str, f: impl FnOnce(&mut Shape)) {
        let Some(id) = self.current() else { return };
        if !self.editable_spline_selected() {
            return;
        }
        self.checkpoint(label);
        let o = &mut self.scene.objects[id];
        if let Some(s) = o.shape_object_mut().and_then(|so| so.editable_mut()) {
            f(s);
        }
        o.evaluate();
        self.scene.propagate_from(id);
    }

    /// World positions of the knots selected at the current spline level.
    pub fn spline_selection_world(&self) -> Vec<Vec3> {
        let Some(level) = self.spline_level() else { return vec![] };
        let Some((sh, w)) = self.base_shape() else { return vec![] };
        sh.selected_knots(level).into_iter().map(|(s, k)| w.transform_point3(sh.splines[s].knots[k].pos)).collect()
    }

    fn base_shape(&self) -> Option<(Shape, Mat4)> {
        let o = self.current_obj()?;
        Some((o.shape_object()?.to_shape(), o.world_matrix()))
    }

    /// Click at a spline sub-object level. Returns true when handled.
    pub fn spline_click_select(&mut self, view: usize, cursor: Vec2, op: SelectOp) -> bool {
        let Some(level) = self.spline_level() else { return false };
        let Some((sh, w)) = self.base_shape() else { return true };
        let v = &self.viewports.views[view];
        let scr = |p: Vec3| v.to_screen(w.transform_point3(p));
        let mut best: Option<(f32, usize, usize)> = None;
        let mut consider = |d: f32, s: usize, k: usize| {
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, s, k));
            }
        };
        for (si, s) in sh.splines.iter().enumerate() {
            if level == SplineLevel::Vertex {
                for (ki, k) in s.knots.iter().enumerate() {
                    if let Some(p) = scr(k.pos) {
                        consider(p.distance(cursor), si, ki);
                    }
                }
                continue;
            }
            for i in 0..s.segment_count() {
                let seg = s.segment(i);
                let pts: Vec<Option<Vec2>> = (0..=sh.steps + 1).map(|t| scr(bezier(seg, t as f32 / (sh.steps + 1) as f32))).collect();
                for pair in pts.windows(2) {
                    if let [Some(a), Some(b)] = pair {
                        consider(point_segment(cursor, *a, *b).0, si, i);
                    }
                }
            }
        }
        let radius = if level == SplineLevel::Vertex { VERTEX_PICK_PX } else { EDGE_PICK_PX };
        let hit = best.filter(|b| b.0 <= radius);
        self.set_spline_selection(level, op, hit.map(|h| (h.1, h.2)).into_iter().collect());
        true
    }

    /// Marquee at a spline sub-object level. Returns true when handled.
    pub fn spline_marquee_select(&mut self, view: usize, a: Vec2, b: Vec2, op: SelectOp) -> bool {
        let Some(level) = self.spline_level() else { return false };
        let Some((sh, w)) = self.base_shape() else { return true };
        let (min, max) = (a.min(b), a.max(b));
        let v = &self.viewports.views[view];
        let inside = |p: Vec3| v.to_screen(w.transform_point3(p)).is_some_and(|s| in_rect(s, min, max));
        let mut hits = vec![];
        for (si, s) in sh.splines.iter().enumerate() {
            let n = s.knots.len();
            match level {
                SplineLevel::Vertex => hits.extend((0..n).filter(|&k| inside(s.knots[k].pos)).map(|k| (si, k))),
                SplineLevel::Segment => hits.extend(
                    (0..s.segment_count()).filter(|&i| inside(s.knots[i].pos) && inside(s.knots[(i + 1) % n].pos)).map(|i| (si, i)),
                ),
                SplineLevel::Spline if n > 0 && s.knots.iter().all(|k| inside(k.pos)) => hits.push((si, 0)),
                SplineLevel::Spline => {}
            }
        }
        self.set_spline_selection(level, op, hits);
        true
    }

    fn set_spline_selection(&mut self, level: SplineLevel, op: SelectOp, hits: Vec<(usize, usize)>) {
        let Some(id) = self.current() else { return };
        let o = &mut self.scene.objects[id];
        let Some(sh) = o.shape_object_mut().and_then(|so| so.editable_mut()) else { return };
        if op == SelectOp::Replace {
            sh.clear_selection();
        }
        let on = op != SelectOp::Subtract;
        for (s, k) in hits {
            let sp = &mut sh.splines[s];
            match level {
                SplineLevel::Vertex => sp.knots[k].selected = on,
                SplineLevel::Segment => sp.knots[k].seg_selected = on,
                SplineLevel::Spline => sp.selected = on,
            }
        }
        o.evaluate();
    }

    /// Screen distance from `cursor` to an object's live splines.
    pub fn shape_screen_distance(&self, view: usize, o: &SceneObject, cursor: Vec2) -> Option<f32> {
        let sh = o.shape.as_ref()?;
        let v = &self.viewports.views[view];
        let w = o.world_matrix();
        sh.polylines()
            .iter()
            .flat_map(|pl| pl.segments().collect::<Vec<_>>())
            .filter_map(|(a, b)| Some(point_segment(cursor, v.to_screen(w.transform_point3(a))?, v.to_screen(w.transform_point3(b))?).0))
            .min_by(f32::total_cmp)
    }

    /// Sets the type of the selected vertices (right-click menu in 3ds Max).
    pub fn set_knot_type(&mut self, kind: KnotType) {
        self.edit_shape(kind.label(), |s| s.set_type_selected(kind));
    }
}
