//! Painting the Move, Rotate and Scale gizmos with the egui painter. Axis
//! colours are red / green / blue; the handle under the cursor or being
//! dragged turns yellow.

use super::handles::{Gz, Handle, RING_STEPS, VIEW_RING, plane_axes};
use crate::editor::TransformMode;
use bevy::math::{Vec2, Vec3};
use bevy_egui::egui::{self, Color32, Pos2, Stroke};

pub const YELLOW: Color32 = Color32::from_rgb(0xFF, 0xD4, 0x00);
const AXIS: [Color32; 3] = [Color32::from_rgb(0xE8, 0x3E, 0x3E), Color32::from_rgb(0x4C, 0xD0, 0x4C), Color32::from_rgb(0x4A, 0x7C, 0xFF)];
const GREY: Color32 = Color32::from_rgb(0xA8, 0xA8, 0xA8);
const W: f32 = 2.0;

fn p(v: Vec2) -> Pos2 {
    Pos2::new(v.x, v.y)
}

/// Arc being swept by a rotate drag: the plane basis, radius and angles.
pub struct Sweep {
    pub center: Vec3,
    pub u: Vec3,
    pub w: Vec3,
    pub radius: f32,
    pub from: f32,
    pub angle: f32,
}

/// What to highlight and show while drawing.
pub struct Look<'a> {
    pub hot: Option<Handle>,
    pub sweep: Option<Sweep>,
    pub readout: Option<&'a str>,
}

fn axis_col(gz: &Gz, k: usize, hot: bool) -> Color32 {
    if hot { YELLOW } else { AXIS[k].gamma_multiply(0.35 + 0.65 * gz.axis_vis(k)) }
}

/// Draws the gizmo for `mode`.
pub fn draw(pt: &egui::Painter, gz: &Gz, mode: TransformMode, look: &Look) {
    match mode {
        TransformMode::Select => return,
        TransformMode::Move => draw_move(pt, gz, look.hot),
        TransformMode::Rotate => draw_rotate(pt, gz, look),
        TransformMode::Scale => draw_scale(pt, gz, look.hot),
    }
    if let Some(text) = look.readout {
        let at = p(gz.c2 + Vec2::new(gz.px * 0.25, gz.px * 0.9 + 14.0));
        let font = egui::FontId::proportional(12.0);
        let galley = pt.layout_no_wrap(text.to_owned(), font, Color32::WHITE);
        let r = egui::Rect::from_min_size(at, galley.size()).expand2(egui::vec2(8.0, 3.0));
        pt.rect_filled(r, 10.0, Color32::from_black_alpha(170));
        pt.galley(at, galley, Color32::WHITE);
    }
}

fn label(pt: &egui::Painter, at: Vec2, text: &str, col: Color32) {
    pt.text(p(at), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(13.0), col);
}

/// Axis letter just past the tip, pushed outwards from the centre.
fn letter(pt: &egui::Painter, gz: &Gz, k: usize, f: f32, col: Color32) {
    let tip = gz.along(k, f);
    let dir = (tip - gz.c2).normalize_or(Vec2::X);
    label(pt, tip + dir * 12.0, ["X", "Y", "Z"][k], col);
}

fn draw_move(pt: &egui::Painter, gz: &Gz, hot: Option<Handle>) {
    // Plane squares: two-colour corner brackets, yellow fill when hot.
    for k in (0..3).filter(|&k| gz.plane_shown(k)) {
        let sq = gz.move_square(k);
        let on = hot == Some(Handle::Plane(k));
        let (a, b) = plane_axes(k);
        if on {
            pt.add(egui::Shape::convex_polygon(sq.iter().map(|v| p(*v)).collect(), YELLOW.gamma_multiply(0.35), Stroke::NONE));
        }
        // sq = [centre, along a, corner, along b]: the edge parallel to b
        // takes b's colour and the one parallel to a takes a's.
        let ca = if on { YELLOW } else { AXIS[b] };
        let cb = if on { YELLOW } else { AXIS[a] };
        pt.line_segment([p(sq[1]), p(sq[2])], Stroke::new(1.5, ca));
        pt.line_segment([p(sq[2]), p(sq[3])], Stroke::new(1.5, cb));
    }
    for k in (0..3).filter(|&k| gz.axis_shown(k)) {
        let on = hot == Some(Handle::Axis(k)) || matches!(hot, Some(Handle::Plane(n)) if n != k);
        let col = axis_col(gz, k, on);
        let (base, tip) = (gz.c2, gz.along(k, 1.0));
        pt.line_segment([p(base), p(tip)], Stroke::new(W, col));
        // Filled arrowhead.
        let dir = (tip - base).normalize_or(Vec2::X);
        let side = dir.perp() * 5.0;
        let back = tip - dir * 14.0;
        let head = vec![p(tip + dir * 4.0), p(back + side), p(back - side)];
        pt.add(egui::Shape::convex_polygon(head, col, Stroke::NONE));
        letter(pt, gz, k, 1.0, col);
    }
    let on = hot == Some(Handle::Screen);
    let r = egui::Rect::from_center_size(p(gz.c2), egui::vec2(8.0, 8.0));
    pt.rect_stroke(r, 1.0, Stroke::new(1.5, if on { YELLOW } else { GREY }), egui::StrokeKind::Middle);
}

fn draw_rotate(pt: &egui::Painter, gz: &Gz, look: &Look) {
    let r = gz.ring_px();
    let hot = look.hot;
    // Inner trackball circle and outer view ring.
    let tb = if hot == Some(Handle::Trackball) { YELLOW } else { GREY.gamma_multiply(0.6) };
    pt.circle_stroke(p(gz.c2), r, Stroke::new(1.0, tb));
    let vr = if hot == Some(Handle::ViewRing) { YELLOW } else { GREY };
    pt.circle_stroke(p(gz.c2), r * VIEW_RING, Stroke::new(W, vr));
    if let Some(s) = &look.sweep {
        sweep(pt, gz, s);
    }
    let step = std::f32::consts::TAU / RING_STEPS as f32;
    for (k, &base) in AXIS.iter().enumerate() {
        let col = if hot == Some(Handle::Axis(k)) { YELLOW } else { base };
        for i in 0..RING_STEPS {
            let (t0, t1) = (i as f32 * step, (i + 1) as f32 * step);
            let front = gz.front(gz.ring_point(k, (t0 + t1) * 0.5));
            let (a, b) = (gz.s(gz.ring_point(k, t0)), gz.s(gz.ring_point(k, t1)));
            let stroke = if front { Stroke::new(W, col) } else { Stroke::new(1.0, col.gamma_multiply(0.18)) };
            pt.line_segment([p(a), p(b)], stroke);
        }
    }
}

/// Translucent pie wedge for the angle swept so far.
fn sweep(pt: &egui::Painter, gz: &Gz, s: &Sweep) {
    let n = ((s.angle.abs() / std::f32::consts::TAU * 96.0).ceil() as usize).clamp(1, 192);
    let mut mesh = egui::Mesh::default();
    let fill = YELLOW.gamma_multiply(0.22);
    mesh.colored_vertex(p(gz.s(s.center)), fill);
    for i in 0..=n {
        let t = s.from + s.angle * i as f32 / n as f32;
        let w = s.center + (s.u * t.cos() + s.w * t.sin()) * s.radius;
        mesh.colored_vertex(p(gz.s(w)), fill);
        if i > 0 {
            mesh.add_triangle(0, i as u32, i as u32 + 1);
        }
    }
    pt.add(mesh);
    let edge = |t: f32| p(gz.s(s.center + (s.u * t.cos() + s.w * t.sin()) * s.radius));
    let c = p(gz.s(s.center));
    pt.line_segment([c, edge(s.from)], Stroke::new(1.0, YELLOW));
    pt.line_segment([c, edge(s.from + s.angle)], Stroke::new(1.0, YELLOW));
}

fn draw_scale(pt: &egui::Painter, gz: &Gz, hot: Option<Handle>) {
    let tri = gz.scale_triangle();
    // Plane strips between the inner and outer triangle edges.
    for k in (0..3).filter(|&k| gz.plane_shown(k)) {
        let strip = gz.scale_strip(k);
        let on = hot == Some(Handle::Plane(k)) || hot == Some(Handle::Uniform);
        if hot == Some(Handle::Plane(k)) {
            pt.add(egui::Shape::convex_polygon(strip.iter().map(|v| p(*v)).collect(), YELLOW.gamma_multiply(0.3), Stroke::NONE));
        }
        let (a, b) = plane_axes(k);
        let half = |x: Vec2, y: Vec2| (x + y) * 0.5;
        let mid = half(strip[2], strip[3]);
        pt.line_segment([p(strip[3]), p(mid)], Stroke::new(1.5, if on { YELLOW } else { AXIS[a] }));
        pt.line_segment([p(mid), p(strip[2])], Stroke::new(1.5, if on { YELLOW } else { AXIS[b] }));
    }
    let on = hot == Some(Handle::Uniform);
    if on {
        pt.add(egui::Shape::convex_polygon(tri.iter().map(|v| p(*v)).collect(), YELLOW.gamma_multiply(0.35), Stroke::NONE));
    }
    for i in 0..3 {
        let col = if on { YELLOW } else { GREY.gamma_multiply(0.7) };
        pt.line_segment([p(tri[i]), p(tri[(i + 1) % 3])], Stroke::new(1.0, col));
    }
    for k in (0..3).filter(|&k| gz.axis_shown(k)) {
        let plane_on = matches!(hot, Some(Handle::Plane(n)) if n != k);
        let col = axis_col(gz, k, on || plane_on || hot == Some(Handle::Axis(k)));
        let tip = gz.along(k, 1.0);
        pt.line_segment([p(gz.c2), p(tip)], Stroke::new(W, col));
        pt.rect_filled(egui::Rect::from_center_size(p(tip), egui::vec2(8.0, 8.0)), 1.0, col);
        letter(pt, gz, k, 1.0, col);
    }
}
