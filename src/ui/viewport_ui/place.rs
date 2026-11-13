//! Prefab place mode in a viewport: wheel rotates, `[` / `]` scale,
//! click drops, Shift-click stamps, right-click ends. A footprint outline
//! with a front arrow follows the cursor on the surface below it.

use crate::editor::{Editor, Tool};
use bevy::math::{Quat, Vec2, Vec3};
use bevy_egui::egui::{self, Key, PointerButton, Stroke};

/// Runs place mode for viewport `vi`. True when it used this frame.
pub fn run(ui: &egui::Ui, ed: &mut Editor, resp: &egui::Response, vi: usize, shift: bool, scroll: f32, pointer: Option<Vec2>) -> bool {
    let Tool::PlacePrefab(kind) = ed.tool else { return false };
    if resp.hovered() {
        if scroll.abs() > 0.0 {
            ed.iface.place.rotate(-scroll.signum());
        }
        let (down, up) = ui.input(|i| (i.key_pressed(Key::OpenBracket), i.key_pressed(Key::CloseBracket)));
        if down || up {
            ed.iface.place.rescale(up);
        }
    }
    if resp.clicked_by(PointerButton::Primary)
        && let Some(p) = resp.interact_pointer_pos()
    {
        ed.place_prefab(kind, vi, Vec2::new(p.x, p.y), shift);
    }
    if resp.clicked_by(PointerButton::Secondary) {
        ed.cancel_tool();
    }
    if let Some(c) = pointer.filter(|_| resp.hovered()) {
        footprint(ui, ed, vi, kind, c);
    }
    true
}

fn footprint(ui: &egui::Ui, ed: &Editor, vi: usize, kind: crate::generators::buildings::BuildingKind, cursor: Vec2) {
    let at = ed.place_point(vi, cursor);
    let o = ed.iface.place;
    let (w, l, _) = kind.defaults();
    let rot = Quat::from_rotation_z(o.yaw_deg.to_radians());
    let half = Vec2::new(w, l) * 0.5 * o.scale;
    let world = |x: f32, y: f32| at + rot * Vec3::new(x, y, 0.02);
    let v = &ed.viewports.views[vi];
    let corners = [(-half.x, -half.y), (half.x, -half.y), (half.x, half.y), (-half.x, half.y)];
    let pts: Option<Vec<egui::Pos2>> = corners.iter().map(|&(x, y)| v.to_screen(world(x, y)).map(|s| egui::pos2(s.x, s.y))).collect();
    let Some(mut pts) = pts else { return };
    let col = crate::ui::theme::accent();
    let p = ui.painter();
    pts.push(pts[0]);
    p.add(egui::Shape::line(pts, Stroke::new(2.0, col)));
    // Front (-Y) arrow.
    let tip = world(0.0, -half.y - half.y.max(0.5) * 0.4);
    if let (Some(a), Some(b)) = (v.to_screen(world(0.0, -half.y)), v.to_screen(tip)) {
        p.arrow(egui::pos2(a.x, a.y), egui::vec2(b.x - a.x, b.y - a.y), Stroke::new(2.0, col));
    }
    if let Some(s) = v.to_screen(at) {
        let txt = format!("{}  {:.0}°  ×{:.2}", kind.label(), o.yaw_deg, o.scale);
        p.text(egui::pos2(s.x + 14.0, s.y + 14.0), egui::Align2::LEFT_TOP, txt, egui::FontId::proportional(12.0), col);
    }
}
