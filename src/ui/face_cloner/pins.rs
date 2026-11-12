//! The 2D pin editor: one picture with the landmarks found on it. Drag a
//! point to pin it where it belongs (pins override the detector in the
//! fit); right-click a pinned point to unpin it.

use super::FaceUi;
use crate::character::face_cloner::job::file_name;
use crate::character::face_cloner::model::POINTS;
use crate::ui::widgets::dim;
use bevy_egui::egui;

#[derive(Default)]
pub struct PinEditor {
    shot: Option<usize>,
    tex: Option<(usize, egui::TextureHandle)>,
    drag: Option<usize>,
}

impl PinEditor {
    pub fn open(&mut self, i: usize) {
        self.shot = Some(i);
        self.drag = None;
    }
}

fn texture(ctx: &egui::Context, path: &str, size: (u32, u32)) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?.to_rgba8();
    let img = if img.dimensions() == size { img } else { image::imageops::resize(&img, size.0, size.1, image::imageops::FilterType::Triangle) };
    let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    Some(ctx.load_texture(format!("face_pin_{path}"), ci, egui::TextureOptions::LINEAR))
}

pub fn show(ctx: &egui::Context, st: &mut FaceUi) {
    let Some(i) = st.pins.shot.filter(|&i| i < st.shots.len()) else {
        st.pins.shot = None;
        return;
    };
    if st.pins.tex.as_ref().is_none_or(|t| t.0 != i) {
        let s = &st.shots[i];
        st.pins.tex = texture(ctx, &s.name, s.size).map(|t| (i, t));
    }
    let mut open = true;
    egui::Window::new(format!("Pins — {}", file_name(&st.shots[i].name))).open(&mut open).default_size([560.0, 640.0]).show(ctx, |ui| {
        let ed = &mut st.pins;
        let s = &mut st.shots[i];
        ui.horizontal(|ui| {
            if ui.button("Clear Pins").clicked() {
                s.pins.clear();
            }
            ui.label(dim(&format!("{} pinned · {} {:+.0}° · drag a dot to fix it, right-click to unpin", s.pins.len(), s.view.label(), s.yaw)));
        });
        let avail = ui.available_size();
        let k = (avail.x / s.size.0 as f32).min((avail.y - 8.0).max(200.0) / s.size.1 as f32).min(2.0);
        let size = egui::vec2(s.size.0 as f32, s.size.1 as f32) * k;
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        match &ed.tex {
            Some((_, t)) => {
                painter.image(t.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            None => {
                painter.rect_filled(rect, 0.0, egui::Color32::from_gray(40));
                painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Picture could not be read", egui::FontId::proportional(14.0), egui::Color32::GRAY);
            }
        }
        let to_screen = |x: f32, y: f32| rect.min + egui::vec2(x, y) * k;
        let near = |pos: Option<egui::Pos2>| pos.and_then(|pos| (0..POINTS).map(|j| (j, s.point(j))).map(|(j, p)| (j, to_screen(p.x, p.y).distance(pos))).filter(|e| e.1 < 8.0).min_by(|a, b| a.1.total_cmp(&b.1)).map(|e| e.0));
        let hover = near(resp.hover_pos());
        let under = near(resp.interact_pointer_pos());
        if resp.drag_started() {
            ed.drag = under;
        }
        if let (Some(j), Some(pos)) = (ed.drag, resp.interact_pointer_pos())
            && resp.dragged()
        {
            let q = (pos - rect.min) / k;
            s.pins.insert(j, [q.x, q.y]);
        }
        if resp.drag_stopped() {
            ed.drag = None;
        }
        if resp.secondary_clicked()
            && let Some(j) = under
        {
            s.pins.remove(&j);
        }
        for j in 0..POINTS {
            let p = s.point(j);
            let (c, r) = if s.pins.contains_key(&j) {
                (crate::ui::theme::accent(), 3.0)
            } else if Some(j) == hover || Some(j) == ed.drag {
                (crate::ui::theme::hover(), 3.5)
            } else if j >= 468 {
                (egui::Color32::from_rgb(120, 220, 255), 1.6)
            } else {
                (egui::Color32::from_rgb(90, 255, 140), 1.2)
            };
            painter.circle_filled(to_screen(p.x, p.y), r, c);
        }
    });
    if !open {
        st.pins.shot = None;
    }
}
