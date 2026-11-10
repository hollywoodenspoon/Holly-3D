//! 3D Snaps (`S`): Grid Points, Pivot, Vertex, Edge/Segment, Midpoint and
//! Face Center. Point snaps win over edges, edges over the grid; the
//! viewport draws a cyan crosshair at the current snap point.

use super::*;
use crate::scene::ObjId;
use std::collections::HashSet;

/// Which snap found the point (shown next to the crosshair).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    Grid,
    Pivot,
    Vertex,
    Edge,
    Midpoint,
    Face,
}

impl SnapKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid Point",
            Self::Pivot => "Pivot",
            Self::Vertex => "Vertex",
            Self::Edge => "Edge/Segment",
            Self::Midpoint => "Midpoint",
            Self::Face => "Face Center",
        }
    }
}

/// Snaps settings (right-click the `S` button).
#[derive(Clone, Debug, PartialEq)]
pub struct SnapSettings {
    pub grid: bool,
    pub pivot: bool,
    pub vertex: bool,
    pub edge: bool,
    pub midpoint: bool,
    pub face: bool,
    /// Grid point spacing (also the Move gizmo increment).
    pub grid_size: f32,
    /// Snap strength: pick radius in pixels.
    pub radius: f32,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self { grid: true, pivot: false, vertex: true, edge: true, midpoint: true, face: false, grid_size: 0.25, radius: 12.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapHit {
    pub pos: Vec3,
    pub kind: SnapKind,
}

/// What the snap must ignore: the things being moved.
#[derive(Default)]
struct Exclude {
    objects: HashSet<ObjId>,
    /// Moving vertices of the current object (sub-object drags).
    verts: Option<(ObjId, HashSet<crate::mesh::VertexId>)>,
}

impl Editor {
    /// Moving the selection excludes it; creation tools exclude nothing.
    fn snap_exclude(&self) -> Exclude {
        let mut ex = Exclude::default();
        if self.tool != Tool::None {
            return ex;
        }
        if self.in_sub_object() || self.spline_level().is_some() {
            if let Some(id) = self.current() {
                let o = &self.scene.objects[id];
                if self.spline_level().is_some() || o.level_mesh.vertices.is_empty() {
                    ex.objects.insert(id);
                } else {
                    ex.verts = Some((id, o.level_mesh.selection_vertices(self.sub).into_iter().collect()));
                }
            }
        } else {
            ex.objects.extend(self.scene.selected());
        }
        ex
    }

    /// Nearest object snap (pivot / vertex / midpoint / face, then edge)
    /// within the snap radius of `cursor`; `None` without one.
    pub fn snap_at(&self, view: usize, cursor: Vec2) -> Option<SnapHit> {
        let c = &self.snap_cfg;
        let v = &self.viewports.views[view];
        let ex = self.snap_exclude();
        let mut best: Option<(f32, SnapHit)> = None;
        let mut edge: Option<(f32, SnapHit)> = None;
        let offer = |best: &mut Option<(f32, SnapHit)>, pos: Vec3, kind: SnapKind| {
            let Some(s) = v.to_screen(pos) else { return };
            let d = s.distance(cursor);
            if d <= c.radius && best.is_none_or(|b| d < b.0) {
                *best = Some((d, SnapHit { pos, kind }));
            }
        };
        for (id, o) in self.selectable().filter(|(id, _)| !ex.objects.contains(id)) {
            let w = o.world_matrix();
            if c.pivot {
                offer(&mut best, o.transform.translation, SnapKind::Pivot);
            }
            if let Some(sh) = &o.shape {
                for sp in &sh.splines {
                    if c.vertex {
                        sp.knots.iter().for_each(|k| offer(&mut best, w.transform_point3(k.pos), SnapKind::Vertex));
                    }
                }
                if c.edge {
                    for pl in sh.polylines() {
                        let n = pl.pts.len();
                        let segs = if pl.closed { n } else { n.saturating_sub(1) };
                        for i in 0..segs {
                            let (a, b) = (w.transform_point3(pl.pts[i]), w.transform_point3(pl.pts[(i + 1) % n]));
                            offer_edge(&mut edge, v, cursor, c.radius, a, b);
                        }
                    }
                }
            }
            let own = ex.verts.as_ref().filter(|(k, _)| *k == id);
            let m = if own.is_some() { &o.level_mesh } else { &o.evaluated_mesh };
            let moving = |vid| own.is_some_and(|(_, s)| s.contains(&vid));
            if c.vertex {
                for (vid, vx) in &m.vertices {
                    if !moving(vid) {
                        offer(&mut best, w.transform_point3(vx.position), SnapKind::Vertex);
                    }
                }
            }
            if c.edge || c.midpoint {
                for e in m.edges.values().filter(|e| !moving(e.v0) && !moving(e.v1)) {
                    let (a, b) = (w.transform_point3(m.pos(e.v0)), w.transform_point3(m.pos(e.v1)));
                    if c.midpoint {
                        offer(&mut best, (a + b) * 0.5, SnapKind::Midpoint);
                    }
                    if c.edge {
                        offer_edge(&mut edge, v, cursor, c.radius, a, b);
                    }
                }
            }
            if c.face {
                for (pid, p) in &m.polygons {
                    if !p.vertex_ids().any(moving) {
                        offer(&mut best, w.transform_point3(m.poly_centroid(pid)), SnapKind::Face);
                    }
                }
            }
        }
        best.or(edge).map(|b| b.1)
    }

    /// Snap point under the cursor while `S` is on: an object snap, else
    /// the nearest grid point on the construction plane (if Grid is on).
    pub fn snap_point(&self, view: usize, cursor: Vec2) -> Option<SnapHit> {
        if !self.snap {
            return None;
        }
        self.snap_at(view, cursor).or_else(|| {
            let g = self.snap_cfg.grid_size.max(1e-4);
            let p = self.viewports.views[view].construction_point(cursor);
            self.snap_cfg.grid.then(|| SnapHit { pos: (p / g).round() * g, kind: SnapKind::Grid })
        })
    }

    /// Construction-plane point under the cursor, snapped when `S` is on.
    pub fn snapped_point_pub(&self, view: usize, cursor: Vec2) -> Vec3 {
        self.snap_point(view, cursor).map_or_else(|| self.viewports.views[view].construction_point(cursor), |h| h.pos)
    }
}

/// Offers the point of segment `a`-`b` closest to the cursor ray
/// (perspective-correct), if it lies within `radius` pixels on screen.
fn offer_edge(best: &mut Option<(f32, SnapHit)>, v: &crate::viewport::layout::ViewState, cursor: Vec2, radius: f32, a: Vec3, b: Vec3) {
    let ray = v.ray(cursor);
    let (u, w0) = (b - a, a - ray.origin);
    let (uu, ud, dd) = (u.dot(u), u.dot(ray.dir), ray.dir.dot(ray.dir));
    let den = uu * dd - ud * ud;
    let t = if den.abs() < 1e-12 { 0.0 } else { ((ud * ray.dir.dot(w0) - dd * u.dot(w0)) / den).clamp(0.0, 1.0) };
    let pos = a + u * t;
    let Some(s) = v.to_screen(pos) else { return };
    let d = s.distance(cursor);
    if d <= radius && best.is_none_or(|x| d < x.0) {
        *best = Some((d, SnapHit { pos, kind: SnapKind::Edge }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::Primitive;

    /// A 10-unit box in a maximized Perspective view (index 3), unselected.
    fn editor() -> (Editor, ObjId) {
        let mut ed = Editor::default();
        let b = Primitive::Box { length: 10.0, width: 10.0, height: 10.0, length_segs: 1, width_segs: 1, height_segs: 1 };
        let id = ed.scene.add(crate::scene::BaseObject::Primitive(b), Vec3::ZERO);
        ed.scene.objects[id].evaluate();
        (ed.viewports.maximized, ed.viewports.active) = (true, 3);
        ed.scene.select_only(None);
        ed.viewports.layout(crate::viewport::layout::ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
        ed.snap = true;
        (ed, id)
    }

    fn world(ed: &Editor, id: ObjId, f: impl Fn(&crate::mesh::EditablePoly) -> Vec3) -> Vec3 {
        let o = &ed.scene.objects[id];
        o.world_matrix().transform_point3(f(&o.evaluated_mesh))
    }

    fn near(ed: &Editor, p: Vec3) -> Vec2 {
        ed.viewports.views[3].to_screen(p).unwrap() + Vec2::new(3.0, -2.0)
    }

    #[test]
    fn point_edge_face_pivot_and_grid_snaps() {
        let (mut ed, id) = editor();
        let e = ed.scene.objects[id].evaluated_mesh.edges.values().next().unwrap().clone();
        let a = world(&ed, id, |m| m.pos(e.v0));
        let b = world(&ed, id, |m| m.pos(e.v1));
        let hit = ed.snap_at(3, near(&ed, a)).unwrap();
        assert_eq!(hit.kind, SnapKind::Vertex);
        assert!(hit.pos.distance(a) < 1e-4);
        let hit = ed.snap_at(3, near(&ed, (a + b) * 0.5)).unwrap();
        assert_eq!(hit.kind, SnapKind::Midpoint);
        assert!(hit.pos.distance((a + b) * 0.5) < 1e-4);
        let q = a.lerp(b, 0.3);
        let hit = ed.snap_at(3, ed.viewports.views[3].to_screen(q).unwrap()).unwrap();
        assert_eq!(hit.kind, SnapKind::Edge);
        assert!(hit.pos.distance(q) < 0.2, "{:?} vs {q}", hit.pos);
        // Face Center and Pivot only when switched on.
        // the +X side face (the bottom face's center is the pivot)
        let m = &ed.scene.objects[id].evaluated_mesh;
        let pid = m.polygons.keys().max_by(|&p, &q| m.poly_centroid(p).x.total_cmp(&m.poly_centroid(q).x)).unwrap();
        let fc = world(&ed, id, |m| m.poly_centroid(pid));
        ed.snap_cfg = SnapSettings { vertex: false, edge: false, midpoint: false, face: true, pivot: true, ..Default::default() };
        let hit = ed.snap_at(3, near(&ed, fc)).unwrap();
        assert_eq!(hit.kind, SnapKind::Face);
        let piv = ed.scene.objects[id].transform.translation;
        assert_eq!(ed.snap_at(3, near(&ed, piv)).map(|h| (h.kind, h.pos)), Some((SnapKind::Pivot, piv)));
        // Empty space: the nearest grid point; S off: nothing.
        let far = Vec2::new(5.0, 590.0);
        assert!(ed.snap_at(3, far).is_none());
        let g = ed.snap_point(3, far).unwrap();
        assert_eq!(g.kind, SnapKind::Grid);
        assert!(((g.pos / 0.25).round() * 0.25).distance(g.pos) < 1e-4);
        ed.snap = false;
        assert!(ed.snap_point(3, far).is_none());
    }

    #[test]
    fn moving_selection_skips_itself_but_creation_tools_snap_to_it() {
        let (mut ed, id) = editor();
        let a = world(&ed, id, |m| m.vertices.values().next().unwrap().position);
        ed.scene.select_only(Some(id));
        assert!(ed.snap_at(3, near(&ed, a)).is_none_or(|h| h.pos.distance(a) > 1e-3));
        ed.tool = Tool::CreateShape(crate::splines::ShapeKind::Circle);
        assert_eq!(ed.snap_at(3, near(&ed, a)).map(|h| h.kind), Some(SnapKind::Vertex));
        assert!(ed.snapped_point_pub(3, near(&ed, a)).distance(a) < 1e-4);
    }
}
