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
    egui::Area::new(egui::Id::new(("vp_labels", vi))).fixed_pos(rect.min + egui::vec2(10.0, 8.0)).order(egui::Order::Middle).show(
        ui.ctx(),
        |ui| {
            ui.horizontal(|ui| {
                // Soft translucent pills over the view.
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().button_padding = egui::vec2(9.0, 3.0);
                let w = &mut ui.visuals_mut().widgets;
                w.inactive.weak_bg_fill = Color32::from_black_alpha(110);
                w.hovered.weak_bg_fill = Color32::from_black_alpha(160);
                w.open.weak_bg_fill = Color32::from_black_alpha(160);
                for wv in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
                    wv.corner_radius = egui::CornerRadius::same(10);
                }
                let small = |t: &str| egui::RichText::new(t).size(12.0).color(theme::text());
                ui.menu_button(small("+"), |ui| {
                    let v = &mut ed.viewports;
                    if ui.button(if v.maximized { "Restore 2x2 Layout   Alt+W" } else { "Maximize Viewport   Alt+W" }).clicked() {
                        v.active = vi;
                        v.maximized = !v.maximized;
                        ui.close();
                    }
                    ui.checkbox(&mut v.views[vi].grid, "Show Grid   G");
                });
                let kind = ed.viewports.views[vi].camera.kind;
                ui.menu_button(small(&format!("{} ⏷", ed.view_label(vi))), |ui| {
                    for k in [ViewKind::Perspective, ViewKind::Top, ViewKind::Front, ViewKind::Left] {
                        if ui.selectable_label(kind == k, k.label()).clicked() {
                            ed.look_through(vi, None);
                            let cam = &mut ed.viewports.views[vi].camera;
                            let keep = (cam.target, cam.distance, cam.ortho_height);
                            *cam = ViewCamera::new(k);
                            (cam.target, cam.distance, cam.ortho_height) = keep;
                            ui.close();
                        }
                    }
                    let cams = ed.scene.cameras();
                    ui.separator();
                    ui.add_enabled_ui(!cams.is_empty(), |ui| {
                        ui.menu_button("Cameras   C", |ui| {
                            for c in cams {
                                let on = ed.viewports.views[vi].cam_obj == Some(c);
                                if ui.selectable_label(on, &ed.scene.objects[c].name).clicked() {
                                    ed.look_through(vi, Some(c));
                                    ui.close();
                                }
                            }
                        });
                    });
                });
                let label = ed.viewports.views[vi].shading_label();
                ui.menu_button(small(&format!("{label} ⏷")), |ui| {
                    let v = &mut ed.viewports.views[vi];
                    if ui.selectable_label(v.shading == Shading::Default, "Default Shading").clicked() {
                        v.shading = Shading::Default;
                    }
                    if ui.selectable_label(v.shading == Shading::Wireframe, "Wireframe   F3").clicked() {
                        v.shading = Shading::Wireframe;
                    }
                    ui.checkbox(&mut v.edged_faces, "Edged Faces   F4");
                    ui.separator();
                    ui.label(small("Background"));
                    let env = &mut ed.prefs.viewport.use_env_background;
                    let was = *env;
                    ui.horizontal(|ui| {
                        ui.selectable_value(env, false, "Viewport").on_hover_text("Neutral viewport grey (Preferences -> Viewports)");
                        ui.selectable_value(env, true, "Environment").on_hover_text("The Environment (8) background colour used by renders");
                    });
                    if *env != was {
                        ed.iface.layout_dirty = true;
                    }
                });
            });
        },
    );
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
        let Some(s) = v.to_screen(w.transform_point3(vx.position)) else {
            continue;
        };
        if !v.rect.contains(s) {
            continue;
        }
        let color = match soft.as_ref().and_then(|ws| ws.get(&id)) {
            Some(&wt) if !vx.selected => {
                // rainbow heat gradient: red (1.0) -> yellow -> blue (0.0)
                let hue = (1.0 - wt) * 0.66;
                egui::ecolor::Hsva::new(hue, 0.9, 1.0, 1.0).into()
            }
            _ if vx.selected => theme::vertex_sel(),
            _ => theme::vertex(),
        };
        let r = if vx.selected { 3.5 } else { 2.5 };
        painter.circle_filled(p(s), r, color);
    }
}

/// FFD control points: orange handles, red when selected (always shown while FFD is active).
fn control_points(painter: &egui::Painter, ed: &Editor, vi: usize) {
    let Some((pts, sel)) = ed.ffd_points_world() else { return };
    let v = &ed.viewports.views[vi];
    let hot = ed.active_mod_sub() == Some(crate::modifiers::ModSub::ControlPoints);
    for (w, s) in pts.iter().zip(sel) {
        let Some(sp) = v.to_screen(*w) else { continue };
        let (color, r) = if s && hot { (theme::vertex_sel(), 4.0) } else { (Color32::from_rgb(255, 140, 30), 3.0) };
        painter.circle_filled(p(sp), r, color);
    }
}

/// Edit Normals lines: cyan unspecified, blue unified, green explicit, red selected.
fn normal_lines(painter: &egui::Painter, ed: &Editor, vi: usize) {
    use crate::modifiers::NormalKind;
    let v = &ed.viewports.views[vi];
    for l in ed.normal_lines() {
        let (Some(a), Some(b)) = (v.to_screen(l.base), v.to_screen(l.tip)) else { continue };
        let color = match (l.selected, l.kind) {
            (true, _) => theme::vertex_sel(),
            (_, NormalKind::Explicit) => Color32::from_rgb(70, 220, 90),
            (_, NormalKind::Unified) => Color32::from_rgb(60, 110, 255),
            (_, NormalKind::Unspecified) => Color32::from_rgb(40, 220, 230),
        };
        painter.line_segment([p(a), p(b)], Stroke::new(if l.selected { 2.0 } else { 1.0 }, color));
    }
}

/// Brush ring at the surface point under the cursor (world-sized).
fn brush_cursor(painter: &egui::Painter, ed: &Editor, vi: usize, ptr: Vec2) {
    let v = &ed.viewports.views[vi];
    let Some((hit, _)) = ed.paint_hit(vi, ptr) else {
        painter.circle_stroke(p(ptr), 4.0, Stroke::new(1.0, theme::text_dim()));
        return;
    };
    let Some(c) = v.to_screen(hit) else { return };
    let r = ed.paint.brush.size / v.camera.units_per_pixel(hit, v.rect.size().y.max(1.0));
    let inner = r * (1.0 - ed.paint.brush.falloff.clamp(0.0, 1.0));
    let col = crate::ui::widgets::color32(ed.paint.brush.color);
    painter.circle_stroke(p(c), r, Stroke::new(2.5, Color32::from_black_alpha(160)));
    painter.circle_stroke(p(c), r, Stroke::new(1.5, col));
    if inner > 2.0 {
        painter.circle_stroke(p(c), inner, Stroke::new(1.0, col.gamma_multiply(0.6)));
    }
    painter.circle_filled(p(c), 1.5, Color32::WHITE);
}

pub fn draw(ui: &egui::Ui, ed: &mut Editor, vi: usize, rect: egui::Rect, region: Option<&[Vec2]>, pointer: Option<Vec2>) {
    let painter = ui.painter().with_clip_rect(rect);
    let active = vi == ed.viewports.active;
    let border = if active { Stroke::new(2.0, theme::accent()) } else { Stroke::new(1.0, theme::border()) };
    super::frame::round_corners(&painter, rect, super::frame::RADIUS, theme::panel());
    painter.rect_stroke(rect, super::frame::RADIUS, border, egui::StrokeKind::Inside);
    let cam = ed.viewports.views[vi].camera;
    tripod(&painter, &cam, rect);
    if ed.in_sub_object() && ed.sub == SubObject::Vertex {
        vertex_dots(&painter, ed, vi);
    }
    control_points(&painter, ed, vi);
    normal_lines(&painter, ed, vi);
    if ed.paint.active
        && let Some(ptr) = pointer.filter(|q| rect.contains(p(*q)))
    {
        brush_cursor(&painter, ed, vi, ptr);
    }
    if ed.snap
        && !ed.paint.active
        && let Some(ptr) = pointer.filter(|q| rect.contains(p(*q)))
    {
        crate::ui::snaps::crosshair(&painter, ed, vi, ptr);
    }
    if let (Some(pts), Some(b)) = (region, pointer) {
        super::region::draw(&painter, ed.region_shape, pts, b);
    }
    if active {
        let world_of = |ed: &Editor, v: crate::mesh::VertexId| {
            ed.current_obj().and_then(|o| o.level_mesh.vertices.get(v).map(|x| o.world_matrix().transform_point3(x.position)))
        };
        let anchor = match &ed.tool {
            Tool::TargetWeld { first: Some(v) } | Tool::Cut { last: Some(v) } => {
                world_of(ed, *v).and_then(|w| ed.viewports.views[vi].to_screen(w))
            }
            Tool::QuickSlice { start: Some(s) } => Some(*s),
            _ => None,
        };
        // Create (polygon): the points so far, rubber-banded to the cursor.
        let pts: Vec<egui::Pos2> = ed.create_poly_points().into_iter().filter_map(|w| ed.viewports.views[vi].to_screen(w)).map(p).collect();
        if !pts.is_empty() {
            let mut line = pts.clone();
            line.extend(pointer.map(p));
            painter.add(egui::Shape::line(line, Stroke::new(1.5, theme::amber())));
            for q in &pts {
                painter.circle_filled(*q, 3.5, theme::amber());
            }
        }
        if let (Some(a), Some(b)) = (anchor, pointer) {
            let color = if matches!(ed.tool, Tool::QuickSlice { .. }) { Color32::from_rgb(255, 220, 30) } else { theme::amber() };
            painter.add(egui::Shape::dashed_line(&[p(a), p(b)], Stroke::new(1.5, color), 6.0, 4.0));
        }
        let paint_hint = ed.paint.active.then_some("Viewport Paint — drag to paint · right-click exits");
        if let Some(hint) = paint_hint.or(ed.tool.label()) {
            painter.text(
                rect.center_bottom() - egui::vec2(0.0, 14.0),
                egui::Align2::CENTER_BOTTOM,
                hint,
                FontId::proportional(13.0),
                theme::amber(),
            );
        }
    }
    label_menus(ui, ed, vi, rect);
    super::view_cube::show(ui, ed, vi, rect);
}
