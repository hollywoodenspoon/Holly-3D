//! Reusable 3ds Max-style widgets.

use super::theme;
use bevy_egui::egui::{self, Color32, Response, RichText, Ui, emath::Numeric};

/// 3ds Max spinner: drag/type a number; **right-click resets to `default`**.
pub fn max_spinner<T: Numeric>(ui: &mut Ui, value: &mut T, speed: f64, default: T) -> Response {
    let mut r = ui.add(egui::DragValue::new(value).speed(speed));
    if r.secondary_clicked() {
        *value = default;
        r.mark_changed();
    }
    r.on_hover_text("Drag or type • Right-click resets")
}

/// Spinner clamped to a range.
pub fn max_spinner_range<T: Numeric>(ui: &mut Ui, value: &mut T, speed: f64, default: T, range: std::ops::RangeInclusive<T>) -> Response {
    let mut r = ui.add(egui::DragValue::new(value).speed(speed).range(range));
    if r.secondary_clicked() {
        *value = default;
        r.mark_changed();
    }
    r.on_hover_text("Drag or type • Right-click resets")
}

/// Tracks spinner/checkbox interaction so undo is recorded once per gesture.
#[derive(Default)]
pub struct Track {
    pub started: bool,
    pub changed: bool,
}

impl Track {
    pub fn r(&mut self, r: Response) -> Response {
        self.changed |= r.changed();
        self.started |= r.drag_started() || r.gained_focus() || r.secondary_clicked() || r.clicked();
        r
    }
}

/// Labeled spinner row inside a two-column grid.
pub fn spin_row<T: Numeric>(ui: &mut Ui, label: &str, value: &mut T, speed: f64, default: T) -> bool {
    ui.label(label);
    let c = max_spinner(ui, value, speed, default).changed();
    ui.end_row();
    c
}

/// Amber-lit toggle button (active Q/W/E/R, sub-object modes, snaps).
pub fn toggle_button(ui: &mut Ui, on: bool, text: impl Into<String>) -> Response {
    let text: String = text.into();
    let rich = if on { RichText::new(text).color(theme::INPUT).strong() } else { RichText::new(text) };
    ui.add(egui::Button::new(rich).selected(on))
}

/// Split Caddy button: 80% main action + 20% `[□]` settings popup.
/// Returns (main clicked, caddy clicked).
pub fn split_button(ui: &mut Ui, label: &str, enabled: bool, width: f32) -> (bool, bool) {
    let mut main = false;
    let mut caddy = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let h = 24.0;
        let cw = (width * 0.2).max(22.0);
        main = ui.add_enabled(enabled, egui::Button::new(label).min_size(egui::vec2(width - cw - 2.0, h))).clicked();
        caddy = ui
            .add_enabled(enabled, egui::Button::new("☐").min_size(egui::vec2(cw, h)))
            .on_hover_text(format!("{label} settings (floating caddy)"))
            .clicked();
    });
    (main, caddy)
}

/// Full-width plain action button.
pub fn action(ui: &mut Ui, label: &str, enabled: bool, width: f32) -> bool {
    ui.add_enabled(enabled, egui::Button::new(label).min_size(egui::vec2(width, 24.0))).clicked()
}

/// Rollout header (collapsing card) with the Max card fill.
pub fn rollout(ui: &mut Ui, title: &str, open: bool, body: impl FnOnce(&mut Ui)) {
    egui::Frame::new().fill(theme::CARD).corner_radius(4).inner_margin(egui::Margin::symmetric(8, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::CollapsingHeader::new(RichText::new(title).strong()).default_open(open).show_unindented(ui, |ui| {
            ui.add_space(2.0);
            body(ui);
        });
    });
    ui.add_space(4.0);
}

pub fn dim(text: &str) -> RichText {
    RichText::new(text).color(theme::TEXT_DIM).small()
}

pub fn color32(c: [f32; 3]) -> Color32 {
    Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8)
}

/// Labeled spinner grid row (right-click resets to `default`).
pub fn num_row(ui: &mut Ui, t: &mut Track, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, default: f32) {
    ui.label(label);
    let speed = ((range.end() - range.start()) / 200.0) as f64;
    t.r(max_spinner_range(ui, v, speed, default, range));
    ui.end_row();
}

/// Labeled color swatch grid row.
pub fn color_row(ui: &mut Ui, t: &mut Track, label: &str, c: &mut [f32; 3]) {
    ui.label(label);
    t.r(ui.color_edit_button_rgb(c));
    ui.end_row();
}
