//! UV Editor canvas: draws the 0..1 page, UV edges and selection, and handles
//! picking, marquee selection and Move / Rotate / Scale drags.

use super::{UvEditorState, UvTool, edit};
use crate::editor::Editor;
use crate::modifiers::ModifierKind;
use crate::modifiers::unwrap_uvw::{CornerRef, UvSel};
use crate::ui::theme;
use bevy::math::Vec2;
use bevy_egui::egui::{self, Color32, Pos2, Rect, Sense, Stroke, pos2};
use std::collections::BTreeSet;

const PICK_PX: f32 = 8.0;
const EDGE: Color32 = Color32::from_rgb(0xB8, 0xC4, 0xD6);
const SEL: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x2A);

pub(super) enum UvDrag {
    Transform { start: Pos2, before: Vec<Vec<Vec2>> },
    Marquee { start: Pos2 },
    Pan,
}

#[derive(Clone, Copy, PartialEq)]
enum Op {
    Replace,
    Add,
    Subtract,
}

/// UV <-> screen mapping (v points up on screen).
struct View {
    origin: Pos2,
    s: f32,
}

impl View {
    fn new(rect: Rect, st: &UvEditorState) -> Self {
        let s = rect.width().min(rect.height()) * 0.82 * st.zoom;
        let c = rect.center() + st.pan;
        Self { origin: pos2(c.x - 0.5 * s, c.y + 0.5 * s), s }
    }
    fn to_screen(&self, uv: Vec2) -> Pos2 {
        pos2(self.origin.x + uv.x * self.s, self.origin.y - uv.y * self.s)
    }
    fn to_uv(&self, p: Pos2) -> Vec2 {
        Vec2::new((p.x - self.origin.x) / self.s, (self.origin.y - p.y) / self.s)
    }
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let (ab, ap) = (b - a, p - a);
    let t = (ap.dot(ab) / ab.length_sq().max(1e-9)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

fn in_polygon(p: Pos2, pts: &[Pos2]) -> bool {
    let mut inside = false;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// Corners hit by a click at `p` in the given selection mode.
fn pick(scr: &[Vec<Pos2>], mode: UvSel, p: Pos2) -> Vec<CornerRef> {
    match mode {
        UvSel::Vertex => scr
            .iter()
            .enumerate()
            .flat_map(|(pi, cs)| cs.iter().enumerate().map(move |(c, q)| ((pi, c), q.distance(p))))
            .filter(|x| x.1 <= PICK_PX)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|x| vec![x.0])
            .unwrap_or_default(),
        UvSel::Edge => scr
            .iter()
            .enumerate()
            .flat_map(|(pi, cs)| (0..cs.len()).map(move |c| (pi, c, seg_dist(p, cs[c], cs[(c + 1) % cs.len()]))))
            .filter(|x| x.2 <= PICK_PX * 0.75)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(pi, c, _)| vec![(pi, c), (pi, (c + 1) % scr[pi].len())])
            .unwrap_or_default(),
        UvSel::Face => scr
            .iter()
            .enumerate()
            .rev()
            .find(|(_, cs)| in_polygon(p, cs))
            .map(|(pi, cs)| (0..cs.len()).map(|c| (pi, c)).collect())
            .unwrap_or_default(),
    }
}

fn marquee(scr: &[Vec<Pos2>], mode: UvSel, r: Rect) -> Vec<CornerRef> {
    let mut hits = vec![];
    for (pi, cs) in scr.iter().enumerate() {
        match mode {
            UvSel::Vertex => hits.extend((0..cs.len()).filter(|&c| r.contains(cs[c])).map(|c| (pi, c))),
            UvSel::Edge => {
                for c in 0..cs.len() {
                    let d = (c + 1) % cs.len();
                    if r.contains(cs[c]) && r.contains(cs[d]) {
                        hits.extend([(pi, c), (pi, d)]);
                    }
                }
            }
            UvSel::Face => {
                if cs.iter().all(|q| r.contains(*q)) {
                    hits.extend((0..cs.len()).map(|c| (pi, c)));
                }
            }
        }
    }
    hits
}

fn select(ed: &mut Editor, hits: Vec<CornerRef>, op: Op) {
    edit(ed, None, |u, cv| {
        let prev = std::mem::take(&mut u.selected);
        u.selected = hits.into_iter().collect();
        u.expand_selection(cv);
        let h = std::mem::take(&mut u.selected);
        u.selected = match op {
            Op::Replace => h,
            Op::Add => prev.union(&h).copied().collect(),
            Op::Subtract => prev.difference(&h).copied().collect(),
        };
    });
}

fn draw(p: &egui::Painter, rect: Rect, v: &View, st: &UvEditorState, scr: &[Vec<Pos2>], sel: &BTreeSet<CornerRef>, mode: UvSel) {
    p.rect_filled(rect, 0.0, theme::INPUT);
    let page = Rect::from_two_pos(v.to_screen(Vec2::ZERO), v.to_screen(Vec2::ONE));
    if st.show_checker {
        const N: usize = 8;
        let step = page.width() / N as f32;
        for i in 0..N {
            for j in 0..N {
                let c = if (i + j) % 2 == 0 { Color32::from_gray(58) } else { Color32::from_gray(84) };
                let min = page.min + egui::vec2(i as f32 * step, j as f32 * step);
                p.rect_filled(Rect::from_min_size(min, egui::vec2(step, step)), 0.0, c);
            }
        }
    } else {
        p.rect_filled(page, 0.0, theme::CARD);
    }
    for k in -3..=4 {
        let g = Stroke::new(1.0, Color32::from_gray(48));
        let k = k as f32;
        p.line_segment([v.to_screen(Vec2::new(k, -3.0)), v.to_screen(Vec2::new(k, 4.0))], g);
        p.line_segment([v.to_screen(Vec2::new(-3.0, k)), v.to_screen(Vec2::new(4.0, k))], g);
    }
    p.rect_stroke(page, 0.0, Stroke::new(1.5, theme::AMBER.gamma_multiply(0.8)), egui::StrokeKind::Middle);
    for (pi, cs) in scr.iter().enumerate() {
        let all = (0..cs.len()).all(|c| sel.contains(&(pi, c)));
        if mode == UvSel::Face && all && cs.len() >= 3 {
            p.add(egui::Shape::convex_polygon(cs.clone(), SEL.gamma_multiply(0.3), Stroke::NONE));
        }
        for c in 0..cs.len() {
            let d = (c + 1) % cs.len();
            let on = mode != UvSel::Vertex && sel.contains(&(pi, c)) && sel.contains(&(pi, d));
            p.line_segment([cs[c], cs[d]], if on { Stroke::new(2.0, SEL) } else { Stroke::new(1.0, EDGE) });
        }
    }
    if mode == UvSel::Vertex {
        for (pi, cs) in scr.iter().enumerate() {
            for (c, q) in cs.iter().enumerate() {
                let on = sel.contains(&(pi, c));
                p.circle_filled(*q, if on { 3.5 } else { 2.2 }, if on { SEL } else { theme::VERTEX });
            }
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UvEditorState) {
    let Some(ModifierKind::Unwrap(u)) = ed.current_obj().and_then(|o| o.active_modifier()) else { return };
    let (uvs, sel, mode) = (u.uvs.clone(), u.selected.clone(), u.sel_mode);
    let size = ui.available_size().max(egui::vec2(200.0, 200.0));
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let (mods, scroll, hover) = ui.input(|i| (i.modifiers, i.smooth_scroll_delta.y, i.pointer.hover_pos()));
    if let (true, Some(cur)) = (resp.hovered() && scroll != 0.0, hover) {
        let uv = View::new(rect, st).to_uv(cur);
        st.zoom = (st.zoom * (scroll * 0.002).exp()).clamp(0.05, 50.0);
        let s = View::new(rect, st).s;
        st.pan = pos2(cur.x - (uv.x - 0.5) * s, cur.y + (uv.y - 0.5) * s) - rect.center();
    }
    let v = View::new(rect, st);
    let scr: Vec<Vec<Pos2>> = uvs.iter().map(|cs| cs.iter().map(|&q| v.to_screen(q)).collect()).collect();
    let op = if mods.command || mods.ctrl {
        Op::Add
    } else if mods.alt {
        Op::Subtract
    } else {
        Op::Replace
    };

    if resp.drag_started_by(egui::PointerButton::Middle) {
        st.drag = Some(UvDrag::Pan);
    } else if resp.drag_started_by(egui::PointerButton::Primary) {
        let start = ui.input(|i| i.pointer.press_origin()).unwrap_or(rect.center());
        let hits = pick(&scr, mode, start);
        if hits.is_empty() || op != Op::Replace {
            st.drag = Some(UvDrag::Marquee { start });
        } else {
            if !hits.iter().any(|c| sel.contains(c)) {
                select(ed, hits, Op::Replace);
            }
            let label = match st.tool {
                UvTool::Move => "UV Move",
                UvTool::Rotate => "UV Rotate",
                UvTool::Scale => "UV Scale",
            };
            ed.checkpoint(label);
            st.drag = Some(UvDrag::Transform { start, before: uvs.clone() });
        }
    }
    let cur = resp.interact_pointer_pos().or(hover);
    match (&st.drag, cur) {
        (Some(UvDrag::Pan), _) => st.pan += resp.drag_delta(),
        (Some(UvDrag::Transform { start, before }), Some(cur)) => {
            let (d, tool, before) = (cur - *start, st.tool, before.clone());
            let shift = mods.shift;
            edit(ed, None, |u, _| {
                u.uvs = before;
                match tool {
                    UvTool::Move => {
                        let mut m = Vec2::new(d.x, -d.y) / v.s;
                        if shift {
                            // Shift constrains to the dominant axis (3ds Max UV editor)
                            if m.x.abs() > m.y.abs() { m.y = 0.0 } else { m.x = 0.0 }
                        }
                        u.move_selected(m);
                    }
                    UvTool::Rotate => {
                        let mut a = -d.x * 0.01;
                        if shift {
                            let snap = 15f32.to_radians();
                            a = (a / snap).round() * snap;
                        }
                        u.rotate_selected(a);
                    }
                    UvTool::Scale => u.scale_selected(Vec2::splat((d.x * 0.006).exp())),
                }
            });
        }
        _ => {}
    }
    let painter = ui.painter_at(rect);
    let (uvs_now, sel_now) = match ed.current_obj().and_then(|o| o.active_modifier()) {
        Some(ModifierKind::Unwrap(u)) => (u.uvs.clone(), u.selected.clone()),
        _ => (uvs, sel),
    };
    let scr_now: Vec<Vec<Pos2>> = uvs_now.iter().map(|cs| cs.iter().map(|&q| v.to_screen(q)).collect()).collect();
    draw(&painter, rect, &v, st, &scr_now, &sel_now, mode);
    if let (Some(UvDrag::Marquee { start }), Some(cur)) = (&st.drag, cur) {
        let r = Rect::from_two_pos(*start, cur);
        painter.rect_filled(r, 0.0, theme::BLUE.gamma_multiply(0.12));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, theme::BLUE), egui::StrokeKind::Middle);
        if resp.drag_stopped() {
            select(ed, marquee(&scr_now, mode, r), op);
        }
    }
    if resp.drag_stopped() {
        st.drag = None;
    }
    if let (true, Some(p)) = (resp.clicked(), resp.interact_pointer_pos()) {
        select(ed, pick(&scr_now, mode, p), op);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> Vec<Vec<Pos2>> {
        vec![vec![pos2(0.0, 0.0), pos2(100.0, 0.0), pos2(100.0, 100.0), pos2(0.0, 100.0)]]
    }

    #[test]
    fn pick_modes() {
        let s = quad();
        assert_eq!(pick(&s, UvSel::Vertex, pos2(98.0, 3.0)), vec![(0, 1)]);
        assert!(pick(&s, UvSel::Vertex, pos2(50.0, 50.0)).is_empty());
        assert_eq!(pick(&s, UvSel::Edge, pos2(50.0, 2.0)), vec![(0, 0), (0, 1)]);
        assert_eq!(pick(&s, UvSel::Face, pos2(50.0, 50.0)).len(), 4);
        assert!(pick(&s, UvSel::Face, pos2(150.0, 50.0)).is_empty());
    }

    #[test]
    fn marquee_modes() {
        let s = quad();
        let r = Rect::from_min_max(pos2(-5.0, -5.0), pos2(105.0, 10.0));
        assert_eq!(marquee(&s, UvSel::Vertex, r).len(), 2);
        assert_eq!(marquee(&s, UvSel::Edge, r), vec![(0, 0), (0, 1)]);
        assert!(marquee(&s, UvSel::Face, r).is_empty());
        let all = Rect::from_min_max(pos2(-5.0, -5.0), pos2(105.0, 105.0));
        assert_eq!(marquee(&s, UvSel::Face, all).len(), 4);
    }

    #[test]
    fn view_roundtrip() {
        let st = UvEditorState::default();
        let v = View::new(Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(400.0, 300.0)), &st);
        let uv = Vec2::new(0.25, 0.75);
        assert!((v.to_uv(v.to_screen(uv)) - uv).length() < 1e-5);
        assert!(v.to_screen(Vec2::Y).y < v.to_screen(Vec2::ZERO).y, "v points up on screen");
    }
}
