//! Editable Spline click tools: Refine (add a vertex where a segment is
//! clicked) and Boolean (click the second closed spline).

use super::select::EDGE_PICK_PX;
use super::*;
use crate::splines::bezier;
use crate::viewport::picking::point_segment;

impl Editor {
    /// Closest spline segment under the cursor: (spline, segment, t).
    pub fn pick_spline_segment(&self, view: usize, cursor: Vec2) -> Option<(usize, usize, f32)> {
        let o = self.current_obj()?;
        let sh = o.shape_object()?.to_shape();
        let w = o.world_matrix();
        let v = &self.viewports.views[view];
        let scr = |p: Vec3| v.to_screen(w.transform_point3(p));
        let n = (sh.steps + 1) as usize * 2;
        let mut best: Option<(f32, usize, usize, f32)> = None;
        for (si, s) in sh.splines.iter().enumerate() {
            for i in 0..s.segment_count() {
                let seg = s.segment(i);
                let pts: Vec<Option<Vec2>> = (0..=n).map(|k| scr(bezier(seg, k as f32 / n as f32))).collect();
                for (k, pair) in pts.windows(2).enumerate() {
                    if let [Some(a), Some(b)] = pair {
                        let (d, t) = point_segment(cursor, *a, *b);
                        if best.is_none_or(|x| d < x.0) {
                            best = Some((d, si, i, (k as f32 + t) / n as f32));
                        }
                    }
                }
            }
        }
        best.filter(|b| b.0 <= EDGE_PICK_PX * 1.5).map(|b| (b.1, b.2, b.3))
    }

    /// Refine: inserts a vertex at the clicked point of a segment (keeps the curve).
    pub(super) fn refine_click(&mut self, view: usize, cursor: Vec2) {
        let Some((si, seg, t)) = self.pick_spline_segment(view, cursor) else { return };
        if !(0.01..=0.99).contains(&t) {
            self.status = "Refine: that point is already a vertex".into();
            return;
        }
        self.edit_shape("Refine", |s| {
            s.clear_selection();
            let k = s.splines[si].refine(seg, t);
            s.splines[si].knots[k].selected = true;
        });
    }

    /// Boolean: the selected closed spline combined with the clicked one.
    pub(super) fn spline_boolean_click(&mut self, view: usize, cursor: Vec2) {
        let Some((b, _, _)) = self.pick_spline_segment(view, cursor) else { return };
        let Some(sh) = self.current_obj().and_then(|o| o.shape_object()).map(|so| so.to_shape()) else { return };
        let Some(a) = sh.splines.iter().position(|s| s.selected) else {
            self.status = "Boolean: select a closed spline first".into();
            return;
        };
        let op = self.spline_ui.bool_op;
        let mut test = sh.clone();
        if !test.spline_boolean(a, b, op) {
            self.status = "Boolean: pick a different closed spline of the same shape".into();
            return;
        }
        self.edit_shape(op.label(), |s| {
            s.spline_boolean(a, b, op);
        });
        self.tool = Tool::None;
        self.status = format!("Spline Boolean {}", op.label());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::splines::boolean::SplineBoolOp;
    use crate::splines::{Knot, Shape, ShapeObject, Spline};

    fn square(x: f32) -> Spline {
        let pts = [(0., 0.), (2., 0.), (2., 2.), (0., 2.)];
        Spline { knots: pts.iter().map(|&(a, b)| Knot::corner(Vec3::new(x + a, b, 0.0))).collect(), closed: true, selected: false }
    }

    /// A shape seen straight down in a maximized Top view.
    fn editor() -> Editor {
        let mut ed = Editor::default();
        let shape = Shape { splines: vec![square(0.0), square(1.0)], steps: 6 };
        let id = ed.scene.add(crate::scene::BaseObject::Shape(ShapeObject::editable(shape)), Vec3::ZERO);
        ed.scene.objects[id].evaluate();
        ed.scene.select_only(Some(id));
        ed.tab = CommandTab::Modify;
        let area = crate::viewport::layout::ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) };
        ed.viewports.layout(area, 0.0);
        ed
    }

    fn screen(ed: &Editor, p: Vec3) -> Vec2 {
        ed.viewports.views[0].to_screen(p).unwrap()
    }

    #[test]
    fn refine_and_boolean_by_clicking() {
        let mut ed = editor();
        let at = screen(&ed, Vec3::new(1.0, 0.0, 0.0));
        let (si, seg, t) = ed.pick_spline_segment(0, at).unwrap();
        assert_eq!((si, seg), (0, 0));
        assert!((t - 0.5).abs() < 0.05, "{t}");
        ed.tool = Tool::RefineSpline;
        ed.tool_click(0, at);
        let sh = ed.current_obj().unwrap().shape.clone().unwrap();
        assert_eq!(sh.splines[0].knots.len(), 5);
        assert!(sh.splines[0].knots[1].selected && sh.splines[0].knots[1].pos.distance(Vec3::X) < 0.05);
        // Boolean: select spline 0, click spline 1 (its right edge).
        ed.edit_shape("select", |s| s.splines[0].selected = true);
        ed.spline_ui.bool_op = SplineBoolOp::Union;
        ed.tool = Tool::PickSplineBoolean;
        ed.tool_click(0, screen(&ed, Vec3::new(3.0, 1.0, 0.0)));
        let sh = ed.current_obj().unwrap().shape.clone().unwrap();
        assert_eq!(sh.splines.len(), 1);
        assert!((sh.polylines()[0].signed_area_xy() - 6.0).abs() < 1e-3);
        assert_eq!(ed.tool, Tool::None);
        ed.undo();
        assert_eq!(ed.current_obj().unwrap().shape.as_ref().unwrap().splines.len(), 2);
    }
}
