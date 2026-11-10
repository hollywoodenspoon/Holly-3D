//! Edit Poly pick tools: Create (polygon), Hinge From Edge, Extrude Along
//! Spline (pick the hinge / spline), and Create Shape from Selection.

use super::select::EDGE_PICK_PX;
use super::*;
use crate::mesh::ops::sweep::{CreatePoint, selected_edge_chains};
use crate::splines::{Knot, Shape, ShapeObject, Spline};
use crate::viewport::picking::pick_edge;

/// Default ops of the pick-based caddies (no hinge / spline picked yet).
pub fn hinge_default() -> PolyOp {
    PolyOp::Hinge { origin: Vec3::ZERO, axis: Vec3::ZERO, angle: 45.0, segments: 1 }
}

pub fn along_spline_default() -> PolyOp {
    PolyOp::ExtrudeAlongSpline { path: vec![], segments: 6, taper: 0.0, twist: 0.0, align: true }
}

impl Editor {
    /// Create: click existing vertices or empty space; clicking the first
    /// point again (or right-click) builds the polygon.
    pub(super) fn create_poly_click(&mut self, view: usize, cursor: Vec2, mut points: Vec<CreatePoint>) {
        let Some(o) = self.current_obj() else { return };
        let pt = match self.pick_cursor_vertex(view, cursor) {
            Some(id) => CreatePoint::Vertex(id),
            None => CreatePoint::New(o.world_to_mesh().transform_point3(self.snapped_point_pub(view, cursor))),
        };
        if points.len() >= 3 && points[0] == pt {
            self.finish_create_poly(view, points);
            return;
        }
        points.push(pt);
        self.tool = Tool::CreatePoly { points };
    }

    /// Builds the polygon from the clicked points. True if the tool was active.
    pub fn finish_create_poly(&mut self, view: usize, points: Vec<CreatePoint>) -> bool {
        self.tool = Tool::CreatePoly { points: vec![] };
        let Some(o) = self.current_obj() else { return true };
        let view_dir = o.world_to_mesh().transform_vector3(self.viewports.views[view].camera.forward()).normalize_or(Vec3::NEG_Z);
        if points.len() >= 3 {
            self.apply_op(PolyOp::CreatePolygon { points, view_dir });
        }
        true
    }

    /// World positions of the Create tool's points (for the overlay).
    pub fn create_poly_points(&self) -> Vec<Vec3> {
        let (Tool::CreatePoly { points }, Some(o)) = (&self.tool, self.current_obj()) else { return vec![] };
        let m = &o.level_mesh;
        points
            .iter()
            .filter_map(|p| match *p {
                CreatePoint::Vertex(v) => m.vertices.get(v).map(|x| x.position),
                CreatePoint::New(p) => Some(p),
            })
            .map(|p| o.world_matrix().transform_point3(p))
            .collect()
    }

    /// Hinge From Edge: the clicked edge becomes the hinge (caddy or direct).
    pub(super) fn pick_hinge_click(&mut self, view: usize, cursor: Vec2) {
        let Some(o) = self.current_obj() else { return };
        let v = &self.viewports.views[view];
        let project = |p: Vec3| v.to_screen(p);
        let Some((e, _)) = pick_edge(&o.level_mesh, o.world_matrix(), &project, cursor, EDGE_PICK_PX * 1.5, &|_| true) else {
            return;
        };
        let (a, b) = (o.level_mesh.pos(o.level_mesh.edges[e].v0), o.level_mesh.pos(o.level_mesh.edges[e].v1));
        self.tool = Tool::None;
        self.with_picked(hinge_default(), |op| {
            if let PolyOp::Hinge { origin, axis, .. } = op {
                (*origin, *axis) = (a, b - a);
            }
        });
    }

    /// Extrude Along Spline: the clicked shape's first spline becomes the path.
    pub(super) fn pick_spline_click(&mut self, view: usize, cursor: Vec2) {
        let Some(sid) = self.pick_object(view, cursor) else { return };
        let Some(o) = self.current_obj() else { return };
        let s = &self.scene.objects[sid];
        let Some(pl) = s.shape.as_ref().and_then(|sh| sh.polylines().into_iter().next()) else {
            self.status = "Extrude Along Spline: pick a shape".into();
            return;
        };
        let to_mesh = o.world_to_mesh() * s.world_matrix();
        let mut path: Vec<Vec3> = pl.pts.iter().map(|&p| to_mesh.transform_point3(p)).collect();
        if pl.closed {
            path.push(path[0]);
        }
        self.tool = Tool::None;
        self.with_picked(along_spline_default(), |op| {
            if let PolyOp::ExtrudeAlongSpline { path: p, .. } = op {
                p.clone_from(&path);
            }
        });
    }

    /// Feeds a picked hinge / spline into the open caddy, or applies the
    /// remembered settings directly.
    fn with_picked(&mut self, default: PolyOp, set: impl Fn(&mut PolyOp)) {
        let key = op_key(&default);
        if let Some(c) = self.caddy.as_mut().filter(|c| op_key(&c.op) == key) {
            set(&mut c.op);
            self.update_caddy_preview();
            return;
        }
        let mut op = self.remembered(default);
        set(&mut op);
        self.op_memory.insert(key, op.clone());
        self.apply_op(op);
    }

    /// Create Shape from Selection: the selected edges become a new
    /// Editable Spline (Smooth or Linear knots). The poly stays selected.
    pub fn create_shape_from_edges(&mut self, smooth: bool) {
        let Some(o) = self.current_obj() else { return };
        let w = o.world_matrix();
        let chains = selected_edge_chains(&o.level_mesh);
        if chains.is_empty() {
            self.status = "Create Shape: select edges first".into();
            return;
        }
        let pts: Vec<Vec3> = chains.iter().flat_map(|(c, _)| c.iter().map(|&p| w.transform_point3(p))).collect();
        let center = pts.iter().sum::<Vec3>() / pts.len() as f32;
        let knot = |p: Vec3| if smooth { Knot::smooth(p - center) } else { Knot::corner(p - center) };
        let splines = chains
            .into_iter()
            .map(|(c, closed)| Spline { knots: c.into_iter().map(|p| knot(w.transform_point3(p))).collect(), closed, selected: false })
            .collect();
        self.checkpoint("Create Shape");
        let id = self.scene.add(crate::scene::BaseObject::Shape(ShapeObject::editable(Shape { splines, ..Default::default() })), center);
        self.scene.objects[id].wire_color = [0.35, 0.72, 0.98];
        self.status = format!("Created {} from the selected edges", self.scene.objects[id].name);
    }
}
