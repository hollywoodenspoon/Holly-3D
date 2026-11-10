//! Drawing the selection region with the mouse: Rectangle / Circle drag
//! from a corner / center, Lasso drags a free-hand trail, Fence drags its
//! first side and then adds a corner per click (click the first corner,
//! double-click or right-click to close it).

use super::ViewportUiState;
use crate::editor::Editor;
use crate::editor::select::SelectOp;
use crate::ui::theme;
use crate::viewport::region::{Region, RegionShape};
use bevy::math::Vec2;
use bevy_egui::egui::{self, Color32, PointerButton, Stroke};

/// Fence closes when a click lands this close to its first corner.
const CLOSE_PX: f32 = 8.0;

fn select(ed: &mut Editor, st: &mut ViewportUiState, vi: usize, op: SelectOp, cursor: Vec2) {
    if let Some((_, pts)) = st.region.take()
        && let Some(r) = Region::build(ed.region_shape, &pts, cursor)
    {
        ed.region_select(vi, &r, op);
    }
    st.fence_open = false;
}

/// While dragging: grows the lasso trail; on release selects (or leaves the
/// fence open for more corners).
pub fn drag(ed: &mut Editor, st: &mut ViewportUiState, resp: &egui::Response, vi: usize, op: SelectOp, pointer: Option<Vec2>) {
    let Some((rv, pts)) = st.region.as_mut() else { return };
    let (Some(p), true) = (pointer, *rv == vi) else { return };
    if ed.region_shape == RegionShape::Lasso && pts.last().is_none_or(|l| l.distance(p) > 3.0) {
        pts.push(p);
    }
    if resp.drag_stopped() {
        if ed.region_shape == RegionShape::Fence {
            pts.push(p);
            st.fence_open = true;
        } else {
            select(ed, st, vi, op, p);
        }
    }
}

/// Fence after its first side: clicks add corners until it is closed.
pub fn fence_clicks(ed: &mut Editor, st: &mut ViewportUiState, resp: &egui::Response, vi: usize, op: SelectOp, pointer: Option<Vec2>) {
    let Some(p) = pointer else { return };
    let first = st.region.as_ref().and_then(|r| r.1.first().copied()).unwrap_or(p);
    if resp.clicked_by(PointerButton::Secondary) || resp.double_clicked() {
        let last = st.region.as_ref().and_then(|r| r.1.last().copied()).unwrap_or(p);
        select(ed, st, vi, op, last);
    } else if resp.clicked_by(PointerButton::Primary) {
        if p.distance(first) < CLOSE_PX {
            select(ed, st, vi, op, first);
        } else if let Some((_, pts)) = st.region.as_mut() {
            pts.push(p);
        }
    }
}

fn pos(v: Vec2) -> egui::Pos2 {
    egui::pos2(v.x, v.y)
}

/// Rubber band of the region being drawn.
pub fn draw(painter: &egui::Painter, shape: RegionShape, pts: &[Vec2], pointer: Vec2) {
    let fill = Color32::from_rgba_unmultiplied(229, 156, 36, 18);
    let st = Stroke::new(1.0, theme::amber());
    let Some(&a) = pts.first() else { return };
    match shape {
        RegionShape::Rectangle => {
            let r = egui::Rect::from_two_pos(pos(a), pos(pointer));
            painter.rect_filled(r, 0.0, fill);
            painter.rect_stroke(r, 0.0, st, egui::StrokeKind::Inside);
        }
        RegionShape::Circle => {
            painter.circle(pos(a), a.distance(pointer), fill, st);
            painter.line_segment([pos(a), pos(pointer)], Stroke::new(1.0, theme::text_dim()));
        }
        RegionShape::Fence | RegionShape::Lasso => {
            let mut line: Vec<egui::Pos2> = pts.iter().map(|&p| pos(p)).collect();
            line.push(pos(pointer));
            painter.add(egui::Shape::line(line, st));
            painter.line_segment([pos(pointer), pos(a)], Stroke::new(1.0, theme::text_dim()));
            if shape == RegionShape::Fence {
                for &p in pts {
                    painter.circle_filled(pos(p), 2.5, theme::amber());
                }
            }
        }
    }
}
