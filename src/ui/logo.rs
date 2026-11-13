//! The Ilex mark in the header: a holly tree cut out of a blue-green tile,
//! the same shape as `packaging/ilex.svg`, drawn in the theme accent.

use super::theme;
use bevy_egui::egui::{self, Pos2, Rect, Sense, Shape, Stroke};

/// The three foliage tiers and trunk on a 0..1 square (x right, y down).
const TIERS: [[(f32, f32); 3]; 3] = [
    [(0.50, 0.13), (0.68, 0.40), (0.32, 0.40)],
    [(0.50, 0.26), (0.77, 0.58), (0.23, 0.58)],
    [(0.50, 0.41), (0.85, 0.77), (0.15, 0.77)],
];

/// Draws the mark `size` points tall and returns its response (for a tooltip).
pub fn mark(ui: &mut egui::Ui, size: f32) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, size * 0.22, theme::accent());
    let cut = theme::panel();
    let at = |(x, y): (f32, f32)| -> Pos2 { rect.min + egui::vec2(x, y) * rect.size() };
    for tier in TIERS {
        p.add(Shape::convex_polygon(tier.iter().map(|&q| at(q)).collect(), cut, Stroke::NONE));
    }
    p.rect_filled(Rect::from_min_max(at((0.46, 0.76)), at((0.54, 0.88))), 0.0, cut);
    r
}

#[cfg(test)]
mod tests {
    use super::TIERS;

    #[test]
    fn tiers_fit_the_tile_and_widen_downwards() {
        for t in TIERS {
            assert!(t.iter().all(|&(x, y)| (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)));
        }
        let width = |t: [(f32, f32); 3]| t[1].0 - t[2].0;
        assert!(width(TIERS[0]) < width(TIERS[1]) && width(TIERS[1]) < width(TIERS[2]));
    }
}
