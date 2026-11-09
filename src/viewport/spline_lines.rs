//! Shapes in the viewports: live splines of every shape object, Editable
//! Spline sub-object display (vertex ticks, Bezier handles, red selection),
//! the Line tool's rubber band and the Lathe Axis sub-object.

use super::layout::ViewState;
use super::lines::{OverlayLines, Vp0Lines, Vp1Lines, Vp2Lines, Vp3Lines};
use crate::editor::{Editor, Tool};
use crate::modifiers::{ModSub, ModifierKind};
use crate::splines::edit::SplineLevel;
use crate::splines::{KnotType, Shape, Spline, bezier};
use bevy::gizmos::config::GizmoConfigGroup;
use bevy::prelude::*;

const RED: Color = Color::srgb(1.0, 0.18, 0.12);
const TICK_PX: f32 = 3.5;

fn square<G: GizmoConfigGroup>(g: &mut Gizmos<G>, v: &ViewState, p: Vec3, px: f32, color: Color) {
    let s = px * v.camera.units_per_pixel(p, v.rect.size().y.max(1.0));
    let (r, u) = (v.camera.right() * s, v.camera.up() * s);
    g.linestrip([p - r - u, p + r - u, p + r + u, p - r + u, p - r - u], color);
}

fn segment_points(s: &Spline, i: usize, steps: u32, w: Mat4) -> Vec<Vec3> {
    let seg = s.segment(i);
    let n = if s.segment_is_line(i) { 1 } else { steps + 1 };
    (0..=n).map(|t| w.transform_point3(bezier(seg, t as f32 / n as f32))).collect()
}

/// Editable Spline sub-object display on the current object.
fn sub_object<G: GizmoConfigGroup>(g: &mut Gizmos<G>, v: &ViewState, sh: &Shape, w: Mat4, level: SplineLevel) {
    for s in &sh.splines {
        for i in 0..s.segment_count() {
            let hot = match level {
                SplineLevel::Segment => s.knots[i].seg_selected,
                SplineLevel::Spline => s.selected,
                SplineLevel::Vertex => false,
            };
            if hot {
                g.linestrip(segment_points(s, i, sh.steps, w), RED);
            }
        }
        for (k, kn) in s.knots.iter().enumerate() {
            let p = w.transform_point3(kn.pos);
            let color = if level == SplineLevel::Vertex && kn.selected {
                RED
            } else if k == 0 {
                Color::srgb(1.0, 0.9, 0.2)
            } else {
                Color::WHITE
            };
            square(g, v, p, if k == 0 { TICK_PX * 1.4 } else { TICK_PX }, color);
            if level == SplineLevel::Vertex && kn.selected && matches!(kn.kind, KnotType::Bezier | KnotType::BezierCorner) {
                let green = Color::srgb(0.3, 0.95, 0.4);
                for h in [kn.in_h, kn.out_h] {
                    let hp = w.transform_point3(h);
                    g.line(p, hp, green);
                    square(g, v, hp, TICK_PX * 0.8, green);
                }
            }
        }
    }
}

fn draw_view<G: GizmoConfigGroup>(g: &mut Gizmos<G>, ed: &Editor, vi: usize) {
    let v = &ed.viewports.views[vi];
    if !v.visible {
        return;
    }
    let current = ed.current();
    let level = ed.spline_level();
    for (id, o) in ed.scene.objects.iter().filter(|(_, o)| !o.hidden) {
        let Some(sh) = &o.shape else { continue };
        let w = o.world_matrix();
        let [r, gg, b] = o.wire_color;
        let color = if o.selected { Color::WHITE } else { Color::srgb(r, gg, b) };
        for pl in sh.polylines() {
            let mut pts: Vec<Vec3> = pl.pts.iter().map(|&p| w.transform_point3(p)).collect();
            if pl.closed
                && let Some(&f) = pts.first()
            {
                pts.push(f);
            }
            g.linestrip(pts, color);
        }
        if let (Some(l), true) = (level, Some(id) == current) {
            sub_object(g, v, sh, w, l);
        }
    }
    // Base level under a consumer modifier (Lathe/Extrude): the live shape is
    // gone, so draw the base spline and its sub-objects from the shape object.
    if let (Some(l), Some(o)) = (level, ed.current_obj())
        && o.shape.is_none()
        && let Some(so) = o.shape_object()
    {
        let (sh, w) = (so.to_shape(), o.world_matrix());
        for pl in sh.polylines() {
            let mut pts: Vec<Vec3> = pl.pts.iter().map(|&p| w.transform_point3(p)).collect();
            if pl.closed
                && let Some(&f) = pts.first()
            {
                pts.push(f);
            }
            g.linestrip(pts, Color::srgb(0.98, 0.85, 0.2));
        }
        sub_object(g, v, &sh, w, l);
    }
    if let Tool::Line { knots, .. } = &ed.tool
        && !knots.is_empty()
    {
        let s = Spline { knots: knots.clone(), closed: false, selected: false };
        for i in 0..s.segment_count() {
            g.linestrip(segment_points(&s, i, 8, Mat4::IDENTITY), Color::WHITE);
        }
        for k in knots {
            square(g, v, k.pos, TICK_PX, Color::srgb(1.0, 0.9, 0.2));
        }
    }
}

/// Lathe Axis sub-object (orange line through the revolution axis).
fn lathe_axis(g: &mut Gizmos<OverlayLines>, ed: &Editor) {
    let Some(o) = ed.current_obj() else { return };
    let (Some(level), Some(ModifierKind::Lathe(l))) = (o.active_level, o.active_modifier()) else { return };
    let Some(shape) = o.shape_input_of(level) else { return };
    let w = o.world_matrix();
    let c = l.axis_point(&shape);
    let len = shape.bounds().map_or(1.0, |b| b.size().length() * 0.75);
    let d = l.axis_dir() * len;
    let color = if ed.active_mod_sub() == Some(ModSub::Axis) { Color::srgb(1.0, 0.55, 0.1) } else { Color::srgb(1.0, 0.86, 0.1) };
    g.line(w.transform_point3(c - d), w.transform_point3(c + d), color);
}

pub fn draw(
    ed: Res<Editor>,
    mut g0: Gizmos<Vp0Lines>,
    mut g1: Gizmos<Vp1Lines>,
    mut g2: Gizmos<Vp2Lines>,
    mut g3: Gizmos<Vp3Lines>,
    mut ov: Gizmos<OverlayLines>,
) {
    draw_view(&mut g0, &ed, 0);
    draw_view(&mut g1, &ed, 1);
    draw_view(&mut g2, &ed, 2);
    draw_view(&mut g3, &ed, 3);
    lathe_axis(&mut ov, &ed);
}
