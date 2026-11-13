//! Soft viewport chrome: each viewport sits as a rounded card on the panel,
//! with a margin around the area and a gap between quadrants.

use crate::ui::theme;
use bevy_egui::egui::{self, epaint::Mesh, Color32, Pos2, Rect};

/// Space around the whole viewport area.
pub const MARGIN: f32 = 6.0;
/// Space between quadrants.
pub const GAP: f32 = 8.0;
/// Corner radius of each viewport card.
pub const RADIUS: f32 = 12.0;

/// The rectangle the viewports are laid out in.
pub fn inner(area: Rect) -> Rect {
    area.shrink(MARGIN)
}

/// Fills the margin and the gaps between quadrants with the panel colour.
pub fn gaps(painter: &egui::Painter, area: Rect, quad: bool) {
    let fill = theme::panel();
    let i = inner(area);
    for r in [
        Rect::from_min_max(area.min, egui::pos2(area.max.x, i.min.y)),
        Rect::from_min_max(egui::pos2(area.min.x, i.max.y), area.max),
        Rect::from_min_max(egui::pos2(area.min.x, i.min.y), egui::pos2(i.min.x, i.max.y)),
        Rect::from_min_max(egui::pos2(i.max.x, i.min.y), egui::pos2(area.max.x, i.max.y)),
    ] {
        painter.rect_filled(r, 0.0, fill);
    }
    if quad {
        let c = i.center();
        let h = GAP * 0.5;
        painter.rect_filled(Rect::from_min_max(egui::pos2(c.x - h, i.min.y), egui::pos2(c.x + h, i.max.y)), 0.0, fill);
        painter.rect_filled(Rect::from_min_max(egui::pos2(i.min.x, c.y - h), egui::pos2(i.max.x, c.y + h)), 0.0, fill);
    }
}

/// Points of the four corner masks: for each corner, the corner itself and
/// the arc between its two tangent points. Each fan triangle is convex.
pub fn corner_fans(rect: Rect, radius: f32, steps: usize) -> Vec<(Pos2, Vec<Pos2>)> {
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let corners = [
        (rect.left_top(), egui::pos2(rect.left() + r, rect.top() + r), std::f32::consts::PI),
        (rect.right_top(), egui::pos2(rect.right() - r, rect.top() + r), 1.5 * std::f32::consts::PI),
        (rect.right_bottom(), egui::pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (rect.left_bottom(), egui::pos2(rect.left() + r, rect.bottom() - r), 0.5 * std::f32::consts::PI),
    ];
    corners
        .iter()
        .map(|&(corner, centre, start)| {
            let arc = (0..=steps)
                .map(|k| {
                    let a = start + std::f32::consts::FRAC_PI_2 * k as f32 / steps as f32;
                    centre + egui::vec2(a.cos(), a.sin()) * r
                })
                .collect();
            (corner, arc)
        })
        .collect()
}

/// Paints `color` over the square corners of `rect` so a 3D view or a
/// texture reads as a rounded card.
pub fn round_corners(painter: &egui::Painter, rect: Rect, radius: f32, color: Color32) {
    let mut mesh = Mesh::default();
    for (corner, arc) in corner_fans(rect, radius, 8) {
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(corner, color);
        for p in &arc {
            mesh.colored_vertex(*p, color);
        }
        for k in 0..arc.len() as u32 - 1 {
            mesh.add_triangle(base, base + 1 + k, base + 2 + k);
        }
    }
    painter.add(mesh);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_arcs_touch_both_edges_and_stay_inside() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 200.0));
        let fans = corner_fans(rect, RADIUS, 8);
        assert_eq!(fans.len(), 4);
        for (corner, arc) in fans {
            assert_eq!(arc.len(), 9);
            let (a, b) = (arc[0], *arc.last().unwrap());
            // The arc starts on one edge through the corner and ends on the other.
            let on_edge = |p: Pos2| (p.x - corner.x).abs() < 1e-3 || (p.y - corner.y).abs() < 1e-3;
            assert!(on_edge(a) && on_edge(b), "{corner:?} {a:?} {b:?}");
            assert!(arc.iter().all(|p| rect.expand(1e-3).contains(*p) && corner.distance(*p) <= RADIUS * 1.001));
        }
        assert!(inner(rect).width() < rect.width());
    }

    #[test]
    fn tiny_rects_clamp_the_radius() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(10.0, 6.0));
        for (_, arc) in corner_fans(rect, RADIUS, 4) {
            assert!(arc.iter().all(|p| rect.expand(1e-3).contains(*p)));
        }
    }
}
