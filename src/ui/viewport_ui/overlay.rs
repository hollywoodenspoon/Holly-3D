//! Viewport overlays: gold active border, `[ + ] [ View ] [ Shading ]` labels,
//! XYZ tripod, vertex dots, marquee rectangle and tool rubber bands.

use crate::editor::{Editor, Tool};
use crate::mesh::SubObject;
use crate::ui::theme;
use crate::viewport::camera::{ViewCamera, ViewKind};
use crate::viewport::layout::Shading;
use bevy::math::{Vec2, Vec3};
use bevy_egui::egui::{self, Color32, FontId, Pos2, Stroke};

fn p(v: Vec2) -> Pos2 {
    Pos2::new(v.x, v.y)
}

fn label_menus(ui: &egui::Ui, ed: &mut Editor, vi: usize, rect: egui::Rect) {
    egui::Area::new(egui::Id::new(("vp_labels", vi)))
        .fixed_pos(rect.min + egui::vec2(6.0, 4.0))
        .order(egui::Order::Middle)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.spacing_mut().button_padding = egui::vec2(4.0, 1.0);
                let small = |t: &str| egui::RichText::new(t).size(12.0).color(theme::TEXT);
                ui.menu_button(small("[ + ]"), |ui| {
                    let v = &mut ed.viewports;
                    if ui.button(if v.maximized { "Restore 2x2 Layout   Alt+W" } else { "Maximize Viewport   Alt+W" }).clicked() {
                        v.active = vi;
                        v.maximized = !v.maximized;
                        ui.close();
                    }
                    ui.checkbox(&mut v.views[vi].grid, "Show Grid   G");
                });
                let kind = ed.viewports.views[vi].camera.kind;
                ui.menu_button(small(&format!("[ {} ]", kind.label())), |ui| {
                    for k in [ViewKind::Perspective, ViewKind::Top, ViewKind::Front, ViewKind::Left] {
                        if ui.selectable_label(kind == k, k.label()).clicked() {
                            let cam = &mut ed.viewports.views[vi].camera;
                            let keep = (cam.target, cam.distance, cam.ortho_height);
                            *cam = ViewCamera::new(k);
                            (cam.target, cam.distance, cam.ortho_height) = keep;
                            ui.close();
                        }
                    }
                });
                let label = ed.viewports.views[vi].shading_label();
                ui.menu_button(small(&format!("[ {label} ]")), |ui| {
                    let v = &mut ed.viewports.views[vi];
                    if ui.selectable_label(v.shading == Shading::Default, "Default Shading").clicked() {
                        v.shading = Shading::Default;
                    }
                    if ui.selectable_label(v.shading == Shading::Wireframe, "Wireframe   F3").clicked() {
                        v.shading = Shading::Wireframe;
                    }
                    ui.checkbox(&mut v.edged_faces, "Edged Faces   F4");
                });
            });
        });
}

fn tripod(painter: &egui::Painter, cam: &ViewCamera, rect: egui::Rect) {
    let origin = Pos2::new(rect.min.x + 34.0, rect.max.y - 34.0);
    for (axis, color, name) in [
        (Vec3::X, Color32::from_rgb(235, 70, 70), "X"),
        (Vec3::Y, Color32::from_rgb(90, 210, 90), "Y"),
        (Vec3::Z, Color32::from_rgb(80, 140, 255), "Z"),
    ] {
        let d = Vec2::new(axis.dot(cam.right()), -axis.dot(cam.up())) * 22.0;
        let tip = origin + egui::vec2(d.x, d.y);
        painter.line_segment([origin, tip], Stroke::new(2.0, color));
        painter.text(tip + egui::vec2(d.x, d.y) * 0.25, egui::Align2::CENTER_CENTER, name, FontId::proportional(11.0), color);
    }
}

fn vertex_dots(painter: &egui::Painter, ed: &Editor, vi: usize) {
    let Some(o) = ed.current_obj() else { return };
    let v = &ed.viewports.views[vi];
    let w = o.world_matrix();
    let m = &o.level_mesh;
    let soft = if ed.soft.enabled {
        let sel = m.selected_vertices();
        Some(crate::mesh::ops::geometry::soft_weights(m, &sel, ed.soft.falloff, ed.soft.pinch, ed.soft.bubble))
    } else {
        None
    };
    for (id, vx) in &m.vertices {
        let Some(s) = v.to_screen(w.transform_point3(vx.position)) else { continue };
        if !v.rect.contains(s) {
            continue;
        }
        let color = match soft.as_ref().and_then(|ws| ws.get(&id)) {
            Some(&wt) if !vx.selected => {
                // rainbow heat gradient: red (1.0) -> yellow -> blue (0.0)
                let hue = (1.0 - wt) * 0.66;
                egui::ecolor::Hsva::new(hue, 0.9, 1.0, 1.0).into()
            }
            _ if vx.selected => theme::VERTEX_SEL,
            _ => theme::VERTEX,
        };
        let r = if vx.selected { 3.5 } else { 2.5 };
        painter.circle_filled(p(s), r, color);
    }
}

pub fn draw(ui: &egui::Ui, ed: &mut Editor, vi: usize, rect: egui::Rect, marquee: Option<Vec2>, pointer: Option<Vec2>) {
    let painter = ui.painter().with_clip_rect(rect);
    let active = vi == ed.viewports.active;
    let border = if active { Stroke::new(2.0, theme::AMBER) } else { Stroke::new(1.0, theme::BORDER) };
    painter.rect_stroke(rect.shrink(1.0), 0.0, border, egui::StrokeKind::Inside);
    let cam = ed.viewports.views[vi].camera;
    tripod(&painter, &cam, rect);
    if ed.in_sub_object() && ed.sub == SubObject::Vertex {
        vertex_dots(&painter, ed, vi);
    }
    if let (Some(a), Some(b)) = (marquee, pointer) {
        let r = egui::Rect::from_two_pos(p(a), p(b));
        painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(229, 156, 36, 18));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, theme::AMBER), egui::StrokeKind::Inside);
    }
    if active {
        let world_of = |ed: &Editor, v: crate::mesh::VertexId| {
            ed.current_obj().and_then(|o| o.level_mesh.vertices.get(v).map(|x| o.world_matrix().transform_point3(x.position)))
        };
        let anchor = match &ed.tool {
            Tool::TargetWeld { first: Some(v) } | Tool::Cut { last: Some(v) } => world_of(ed, *v).and_then(|w| ed.viewports.views[vi].to_screen(w)),
            Tool::QuickSlice { start: Some(s) } => Some(*s),
            _ => None,
        };
        if let (Some(a), Some(b)) = (anchor, pointer) {
            let color = if matches!(ed.tool, Tool::QuickSlice { .. }) { Color32::from_rgb(255, 220, 30) } else { theme::AMBER };
            painter.add(egui::Shape::dashed_line(&[p(a), p(b)], Stroke::new(1.5, color), 6.0, 4.0));
        }
        if let Some(hint) = ed.tool.label() {
            painter.text(rect.center_bottom() - egui::vec2(0.0, 14.0), egui::Align2::CENTER_BOTTOM, hint, FontId::proportional(13.0), theme::AMBER);
        }
    }
    label_menus(ui, ed, vi, rect);
}
