//! The Orientation Cube in the top-right corner of every view: click a face,
//! edge or corner to fly there, drag the cube to orbit, drag the compass ring
//! to spin around Z, Home returns to the saved view. Right-click for options.

use crate::editor::Editor;
use crate::ui::theme;
use crate::viewport::camera::{ViewCamera, ViewKind};
use crate::viewport::view_cube::{self as vc, EDGE_BAND, FACES, Flight};
use bevy::math::{IVec3, Vec2, Vec3};
use bevy_egui::egui::{self, Color32, FontId, Pos2, Sense, Shape, Stroke};

const W: f32 = 120.0;
const H: f32 = 116.0;
/// Pixels per cube unit (the cube is 2 units wide).
const S: f32 = 23.0;
const RING_R: f32 = 1.75;
const RING_Z: f32 = -1.0;

#[derive(Clone, Copy, PartialEq)]
enum Hot {
    Home,
    Cube(IVec3),
    Ring,
}

struct Widget {
    center: Pos2,
    r: Vec3,
    u: Vec3,
    f: Vec3,
}

impl Widget {
    fn screen(&self, p: Vec3) -> Pos2 {
        self.center + egui::vec2(p.dot(self.r), -p.dot(self.u)) * S
    }
    fn local(&self, pos: Pos2) -> Vec2 {
        let d = (pos - self.center) / S;
        Vec2::new(d.x, -d.y)
    }
    fn ring(&self) -> Vec<(Pos2, f32)> {
        (0..=48)
            .map(|i| {
                let a = i as f32 / 48.0 * std::f32::consts::TAU;
                let q = Vec3::new(a.cos() * RING_R, a.sin() * RING_R, RING_Z);
                (self.screen(q), q.dot(self.f))
            })
            .collect()
    }
}

fn home_rect(wr: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(wr.min + egui::vec2(8.0, 4.0), egui::vec2(18.0, 18.0))
}

fn hot_at(w: &Widget, cam: &ViewCamera, wr: egui::Rect, pos: Pos2) -> Option<Hot> {
    if home_rect(wr).contains(pos) {
        return Some(Hot::Home);
    }
    if let Some(d) = vc::pick(cam, w.local(pos)) {
        return Some(Hot::Cube(d));
    }
    let ring = w.ring();
    let near = ring.windows(2).any(|s| {
        let seg = egui::emath::Rect::from_two_pos(s[0].0, s[1].0).expand(7.0);
        seg.contains(pos) && dist_to_segment(pos, s[0].0, s[1].0) < 7.0
    });
    near.then_some(Hot::Ring)
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// Advances a running flight, then draws and drives the cube for view `vi`.
pub fn show(ui: &egui::Ui, ed: &mut Editor, vi: usize, rect: egui::Rect) {
    if let Some(mut fl) = ed.viewports.views[vi].flight.take() {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        let (cam, done) = fl.step(dt);
        ed.viewports.views[vi].camera = cam;
        if !done {
            ed.viewports.views[vi].flight = Some(fl);
            ui.ctx().request_repaint();
        }
        ed.push_view_to_camera(vi);
    }
    let v = &ed.viewports.views[vi];
    if !ed.prefs.viewport.show_cube || v.cam_obj.is_some() || rect.width() < 220.0 || rect.height() < 170.0 {
        return;
    }
    egui::Area::new(egui::Id::new(("view_cube", vi)))
        .fixed_pos(rect.right_top() + egui::vec2(-W - 8.0, 8.0))
        .order(egui::Order::Middle)
        .show(ui.ctx(), |ui| widget(ui, ed, vi));
}

fn widget(ui: &mut egui::Ui, ed: &mut Editor, vi: usize) {
    let (wr, resp) = ui.allocate_exact_size(egui::vec2(W, H), Sense::click_and_drag());
    let cam = ed.viewports.views[vi].camera;
    let (r, u, f) = vc::axes(&cam);
    let w = Widget { center: wr.center() + egui::vec2(4.0, 2.0), r, u, f };
    let drag_id = egui::Id::new(("view_cube_drag", vi));
    let dragging: Option<Hot> = ui.data(|d| d.get_temp(drag_id)).flatten();
    let hot = if resp.dragged() { dragging } else { resp.hover_pos().and_then(|p| hot_at(&w, &cam, wr, p)) };
    draw(ui.painter(), &w, wr, hot);

    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(match hot {
            Some(Hot::Ring) => egui::CursorIcon::Grab,
            Some(_) => egui::CursorIcon::PointingHand,
            None => egui::CursorIcon::Default,
        });
    }
    if resp.drag_started() {
        ed.viewports.active = vi;
        let start = resp.interact_pointer_pos().and_then(|p| hot_at(&w, &cam, wr, p));
        ui.data_mut(|d| d.insert_temp(drag_id, start.filter(|h| *h != Hot::Home)));
    }
    if resp.dragged_by(egui::PointerButton::Primary) {
        let d = resp.drag_delta();
        let speed = ed.prefs.viewport.orbit_speed;
        let view = &mut ed.viewports.views[vi];
        view.flight = None;
        let pivot = view.camera.target;
        match dragging {
            Some(Hot::Cube(_)) => view.camera.orbit(Vec2::new(d.x, d.y) * speed, pivot),
            Some(Hot::Ring) => view.camera.orbit(Vec2::new(d.x, 0.0) * speed, pivot),
            _ => {}
        }
        ed.push_view_to_camera(vi);
    }
    if resp.drag_stopped() {
        ui.data_mut(|d| d.remove::<Option<Hot>>(drag_id));
    }
    if resp.clicked() {
        ed.viewports.active = vi;
        match resp.interact_pointer_pos().and_then(|p| hot_at(&w, &cam, wr, p)) {
            Some(Hot::Home) => go_home(ed, vi),
            Some(Hot::Cube(d)) => fly(ed, vi, vc::camera_for(&cam, d)),
            _ => {}
        }
    }
    resp.context_menu(|ui| menu(ui, ed, vi));
}

fn fly(ed: &mut Editor, vi: usize, mut to: ViewCamera) {
    if ed.prefs.viewport.cube_fit_on_change
        && let Some(b) = ed.selection_bounds()
    {
        to.frame(b.min, b.max);
    }
    let view = &mut ed.viewports.views[vi];
    view.flight = Some(Flight::new(view.camera, to));
}

fn go_home(ed: &mut Editor, vi: usize) {
    let cam = ed.viewports.views[vi].camera;
    let to = ed.viewports.views[vi].home.unwrap_or_else(|| {
        let std = ViewCamera::new(ViewKind::Perspective);
        let mut c = cam;
        (c.yaw, c.pitch) = (std.yaw, std.pitch);
        if c.is_ortho() {
            c.kind = ViewKind::User;
        }
        c
    });
    fly(ed, vi, to);
}

fn menu(ui: &mut egui::Ui, ed: &mut Editor, vi: usize) {
    if ui.button("Home").clicked() {
        go_home(ed, vi);
        ui.close();
        return;
    }
    let view = &mut ed.viewports.views[vi];
    if ui.button("Set Current View as Home").clicked() {
        view.home = Some(view.camera);
        ui.close();
    }
    if ui.add_enabled(view.home.is_some(), egui::Button::new("Reset Home")).clicked() {
        view.home = None;
        ui.close();
    }
    ui.separator();
    let ortho = view.camera.is_ortho();
    let persp_clicked = ui.selectable_label(!ortho, "Perspective").clicked();
    let ortho_clicked = ui.selectable_label(ortho, "Orthographic").clicked();
    if (persp_clicked && ortho) || (ortho_clicked && !ortho) {
        view.flight = None;
        vc::toggle_projection(&mut view.camera);
        ui.close();
    }
    ui.separator();
    if ui.checkbox(&mut ed.prefs.viewport.cube_fit_on_change, "Fit to View on Change").changed() {
        ed.iface.layout_dirty = true;
    }
    if ui.button("Hide Orientation Cube").clicked() {
        ed.prefs.viewport.show_cube = false;
        ed.iface.layout_dirty = true;
        ui.close();
    }
}

fn draw(painter: &egui::Painter, w: &Widget, wr: egui::Rect, hot: Option<Hot>) {
    let ring = w.ring();
    let ring_col = if hot == Some(Hot::Ring) { theme::hover() } else { Color32::from_white_alpha(60) };
    let letters = [(Vec3::Y, "N"), (Vec3::X, "E"), (-Vec3::Y, "S"), (-Vec3::X, "W")];
    let ring_pass = |behind: bool| {
        for s in ring.windows(2) {
            if (s[0].1 + s[1].1 > 0.0) == behind {
                painter.line_segment([s[0].0, s[1].0], Stroke::new(4.0, ring_col));
            }
        }
        for (dir, txt) in letters {
            let q = dir * RING_R + Vec3::Z * RING_Z;
            if (q.dot(w.f) > 0.0) == behind {
                let col = if txt == "N" { theme::accent() } else { theme::text() };
                painter.text(w.screen(q), egui::Align2::CENTER_CENTER, txt, FontId::proportional(9.0), col);
            }
        }
    };
    ring_pass(true);
    let hot_dir = match hot {
        Some(Hot::Cube(d)) => Some(d),
        _ => None,
    };
    for (n, label) in FACES {
        face(painter, w, n, label, hot_dir);
    }
    ring_pass(false);
    house(painter, home_rect(wr), hot == Some(Hot::Home));
}

fn face(painter: &egui::Painter, w: &Widget, n: IVec3, label: &str, hot: Option<IVec3>) {
    let facing = -n.as_vec3().dot(w.f);
    if facing <= 0.02 {
        return;
    }
    // The two in-face axes.
    let a = if n.x != 0 { IVec3::Y } else { IVec3::X };
    let b = n.cross(a).abs();
    let cuts = [-1.0, -EDGE_BAND, EDGE_BAND, 1.0];
    let g = (70.0 + 85.0 * facing + if n.z > 0 { 12.0 } else { 0.0 }) as u8;
    let base = Color32::from_gray(g);
    for i in 0..3 {
        for j in 0..3 {
            let target = n + a * (i as i32 - 1) + b * (j as i32 - 1);
            let fill = if hot == Some(target) { theme::accent().gamma_multiply(0.85) } else { base };
            let corner = |s: usize, t: usize| w.screen(n.as_vec3() + a.as_vec3() * cuts[s] + b.as_vec3() * cuts[t]);
            let pts = vec![corner(i, j), corner(i + 1, j), corner(i + 1, j + 1), corner(i, j + 1)];
            painter.add(Shape::convex_polygon(pts, fill, Stroke::NONE));
        }
    }
    let outline = [(0, 0), (3, 0), (3, 3), (0, 3), (0, 0)].map(|(s, t)| {
        w.screen(n.as_vec3() + a.as_vec3() * cuts[s] + b.as_vec3() * cuts[t])
    });
    painter.add(Shape::line(outline.to_vec(), Stroke::new(1.0, Color32::from_black_alpha(140))));
    // Flat text sized to fit the face's on-screen width (the UI font is monospace).
    let bb = egui::Rect::from_points(&outline);
    let size = (bb.width() * 0.8 / (label.len() as f32 * 0.62)).min(10.0);
    if facing > 0.3 && size >= 6.0 && bb.height() > size * 1.6 {
        let col = Color32::from_gray(235).gamma_multiply(((facing - 0.3) / 0.4).min(1.0));
        painter.text(w.screen(n.as_vec3()), egui::Align2::CENTER_CENTER, label, FontId::proportional(size), col);
    }
}

fn house(painter: &egui::Painter, r: egui::Rect, hot: bool) {
    let col = if hot { theme::hover() } else { theme::text_dim() };
    let st = Stroke::new(1.4, col);
    let c = r.center();
    let roof = [c + egui::vec2(-6.0, -0.5), c + egui::vec2(0.0, -6.0), c + egui::vec2(6.0, -0.5)];
    painter.add(Shape::line(roof.to_vec(), st));
    let body = [
        c + egui::vec2(-4.0, -2.0),
        c + egui::vec2(-4.0, 5.5),
        c + egui::vec2(4.0, 5.5),
        c + egui::vec2(4.0, -2.0),
    ];
    painter.add(Shape::line(body.to_vec(), st));
}
