//! Parameter editors of the procedural engine's map nodes: Tileable
//! Pattern, Levels (with a histogram of its input), Height Blend, Slope
//! Blur and Normal From Height. Used by both the Compact Maps rollout and
//! the Node Graph inspector.

use super::params::{color, num};
use crate::materials::MapNode;
use crate::materials::proc_engine::{TileKind, adjust, node_key};
use crate::ui::theme;
use crate::ui::widgets::{Track, max_spinner_range};
use bevy_egui::egui::{self, pos2, vec2};

/// Histogram of a node's luminance with the Levels input markers on it.
fn histogram(ui: &mut egui::Ui, input: &MapNode, low: f32, mid: f32, high: f32) {
    let grid = node_key::grid(input);
    let bins = adjust::histogram(grid.data.iter().map(|c| c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722));
    let (rect, _) = ui.allocate_exact_size(vec2(150.0, 44.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, theme::input());
    let w = rect.width() / bins.len() as f32;
    for (i, b) in bins.iter().enumerate() {
        let x = rect.left() + i as f32 * w;
        p.rect_filled(egui::Rect::from_min_max(pos2(x, rect.bottom() - b * (rect.height() - 4.0)), pos2(x + w - 1.0, rect.bottom())), 0.0, theme::text_dim());
    }
    // In low / mid / high: mid sits between low and high.
    let mid_at = low + (high - low) * mid;
    for (v, c) in [(low, theme::blue()), (mid_at, theme::text()), (high, theme::amber())] {
        let x = rect.left() + v.clamp(0.0, 1.0) * rect.width();
        p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], egui::Stroke::new(1.5, c));
    }
}

/// Grid rows for the engine's nodes; false when `node` is not one of them.
pub(super) fn rows(ui: &mut egui::Ui, salt: &str, node: &mut MapNode, t: &mut Track) -> bool {
    match node {
        MapNode::Tile { kind, scale, detail, width, warp, seed, color_a, color_b } => {
            ui.label("Generator");
            let before = *kind;
            egui::ComboBox::from_id_salt(("tile_kind", salt)).selected_text(kind.label()).show_ui(ui, |ui| {
                for k in TileKind::ALL {
                    t.r(ui.selectable_value(kind, k, k.label()));
                }
            });
            ui.end_row();
            if *kind != before {
                // A different generator reads the knobs differently: start from its defaults.
                [*scale, *detail, *width, *warp] = kind.defaults();
            }
            for (value, (label, lo, hi, default)) in [scale, detail, width, warp].into_iter().zip(kind.knobs()) {
                if !label.is_empty() {
                    num(ui, t, label, value, lo..=hi, default);
                }
            }
            ui.label("Seed");
            t.r(max_spinner_range(ui, seed, 0.2, 0, 0..=9999));
            ui.end_row();
            color(ui, t, "Low / Gap", color_a);
            color(ui, t, "High / Surface", color_b);
        }
        MapNode::Levels { input, in_low, in_mid, in_high, out_low, out_high } => {
            ui.label("Histogram");
            histogram(ui, input, *in_low, *in_mid, *in_high);
            ui.end_row();
            num(ui, t, "In Low", in_low, 0.0..=1.0, 0.0);
            num(ui, t, "In Mid", in_mid, 0.01..=0.99, 0.5);
            num(ui, t, "In High", in_high, 0.0..=1.0, 1.0);
            num(ui, t, "Out Low", out_low, 0.0..=1.0, 0.0);
            num(ui, t, "Out High", out_high, 0.0..=1.0, 1.0);
        }
        MapNode::HeightBlend { level, contrast, .. } => {
            num(ui, t, "Fill Level", level, 0.0..=1.0, 0.35);
            num(ui, t, "Contrast", contrast, 0.0..=1.0, 0.8);
        }
        MapNode::SlopeBlur { intensity, samples, .. } => {
            num(ui, t, "Intensity", intensity, 0.0..=64.0, 8.0);
            ui.label("Samples");
            t.r(max_spinner_range(ui, samples, 0.1, 12, 1..=64));
            ui.end_row();
        }
        MapNode::NormalFromHeight { intensity, .. } => num(ui, t, "Intensity", intensity, 0.0..=8.0, 1.0),
        _ => return false,
    }
    true
}
