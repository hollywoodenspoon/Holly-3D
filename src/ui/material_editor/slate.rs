//! Optional Slate node view: map nodes wired into the material's channel
//! sockets. Drag from a node's output socket onto a channel (or onto a Color
//! Correction input) to wire it; drop on empty canvas to unwire. Right-click
//! the canvas to add nodes.

use super::edit;
use super::maps::node_params;
use crate::editor::Editor;
use crate::materials::{MapChannel, MapNode, Material};
use crate::ui::theme;
use crate::ui::widgets::dim;
use bevy_egui::egui::{self, Color32, Pos2, Rect, RichText, Stroke, pos2, vec2};
use std::collections::HashMap;

const NODE: egui::Vec2 = vec2(132.0, 40.0);
const ROW: f32 = 28.0;
const CANVAS_H: f32 = 300.0;
const INSPECTOR_H: f32 = 150.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeRef {
    /// Node feeding a material channel.
    Wired(usize),
    /// Input of the Color Correction node on a channel.
    Input(usize),
    /// Unwired node on the canvas.
    Free(usize),
}

#[derive(Default)]
pub struct SlateState {
    offsets: HashMap<NodeRef, egui::Vec2>,
    selected: Option<NodeRef>,
    wiring: Option<NodeRef>,
}

fn node_mut(m: &mut Material, r: NodeRef) -> Option<&mut MapNode> {
    match r {
        NodeRef::Wired(c) => m.maps[c].node.as_mut(),
        NodeRef::Input(c) => match m.maps[c].node.as_mut() {
            Some(MapNode::ColorCorrect { input, .. }) => Some(input),
            _ => None,
        },
        NodeRef::Free(i) => m.slate_nodes.get_mut(i),
    }
}

/// Removes a node from where it sits (Color Correction inputs are copied:
/// the corrector always keeps an input).
fn take(m: &mut Material, r: NodeRef) -> Option<MapNode> {
    match r {
        NodeRef::Wired(c) => m.maps[c].node.take(),
        NodeRef::Input(_) => node_mut(m, r).cloned(),
        NodeRef::Free(i) => (i < m.slate_nodes.len()).then(|| m.slate_nodes.remove(i)),
    }
}

/// Drop targets: material channel sockets or a corrector's input socket.
#[derive(Clone, Copy, PartialEq)]
enum Target {
    Channel(usize),
    CorrectorInput(usize),
}

/// Rewires `src` into `dst`; displaced nodes go back on the canvas.
fn wire(m: &mut Material, src: NodeRef, dst: Option<Target>) {
    if dst
        == Some(Target::Channel(match src {
            NodeRef::Wired(c) => c,
            _ => usize::MAX,
        }))
    {
        return;
    }
    let Some(node) = take(m, src) else { return };
    let old = match dst {
        Some(Target::Channel(c)) => m.maps[c].node.replace(node),
        Some(Target::CorrectorInput(c)) => match m.maps[c].node.as_mut() {
            Some(MapNode::ColorCorrect { input, .. }) => Some(std::mem::replace(&mut **input, node)),
            _ => Some(node),
        },
        None if matches!(src, NodeRef::Input(_)) => None,
        None => Some(node),
    };
    m.slate_nodes.extend(old);
}

struct Layout {
    mat: Rect,
    nodes: Vec<(NodeRef, Rect)>,
}

impl Layout {
    fn socket(&self, ch: usize) -> Pos2 {
        pos2(self.mat.left(), self.mat.top() + 30.0 + ch as f32 * ROW + ROW * 0.5)
    }
    fn rect(&self, r: NodeRef) -> Option<Rect> {
        self.nodes.iter().find(|(k, _)| *k == r).map(|(_, rc)| *rc)
    }
}

fn layout(m: &Material, canvas: Rect, st: &SlateState) -> Layout {
    let mat = Rect::from_min_size(pos2(canvas.right() - 150.0, canvas.top() + 12.0), vec2(140.0, 30.0 + 6.0 * ROW + 6.0));
    let mut nodes = vec![];
    let mut put = |r: NodeRef, p: Pos2| {
        let off = st.offsets.get(&r).copied().unwrap_or_default();
        nodes.push((r, Rect::from_min_size(p + off, NODE)));
    };
    // Wired nodes sit level with their socket, stacked so they never overlap.
    let mut next_y = f32::MIN;
    for c in 0..6 {
        let y = (mat.top() + 30.0 + c as f32 * ROW + ROW * 0.5 - NODE.y * 0.5).max(next_y);
        if let Some(n) = &m.maps[c].node {
            next_y = y + NODE.y + 6.0;
            put(NodeRef::Wired(c), pos2(mat.left() - NODE.x - 40.0, y));
            if matches!(n, MapNode::ColorCorrect { .. }) {
                put(NodeRef::Input(c), pos2(mat.left() - 2.0 * NODE.x - 70.0, y));
            }
        }
    }
    // Unwired nodes line up along the bottom of the canvas.
    let per_row = ((canvas.width() - 20.0) / (NODE.x + 10.0)).floor().max(1.0) as usize;
    for i in 0..m.slate_nodes.len() {
        let (col, row) = ((i % per_row) as f32, (i / per_row) as f32);
        put(NodeRef::Free(i), pos2(canvas.left() + 10.0 + col * (NODE.x + 10.0), canvas.bottom() - (row + 1.0) * (NODE.y + 10.0)));
    }
    Layout { mat, nodes }
}

fn wire_curve(p: &egui::Painter, a: Pos2, b: Pos2, color: Color32) {
    let k = ((b.x - a.x).abs() * 0.5).max(30.0);
    let pts = [a, a + vec2(k, 0.0), b - vec2(k, 0.0), b];
    p.add(egui::epaint::CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(2.0, color)));
}

fn draw_node(p: &egui::Painter, rect: Rect, title: &str, sub: &str, selected: bool, output: bool) {
    p.rect_filled(rect, 4.0, theme::CARD);
    let border = if selected { Stroke::new(2.0, theme::AMBER) } else { Stroke::new(1.0, theme::BORDER) };
    p.rect_stroke(rect, 4.0, border, egui::StrokeKind::Inside);
    p.text(rect.left_top() + vec2(8.0, 5.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(13.0), theme::TEXT);
    p.text(rect.left_top() + vec2(8.0, 22.0), egui::Align2::LEFT_TOP, sub, egui::FontId::proportional(11.0), theme::TEXT_DIM);
    if output {
        p.circle_filled(pos2(rect.right(), rect.center().y), 5.0, theme::BLUE);
    }
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut SlateState) {
    let m = ed.scene.materials.mats[ed.active_material()].clone();
    let (canvas, bg) = ui.allocate_exact_size(vec2(ui.available_width(), CANVAS_H), egui::Sense::click());
    let p = ui.painter_at(canvas);
    p.rect_filled(canvas, 4.0, theme::INPUT);
    let lay = layout(&m, canvas, st);
    // Material node with its six channel sockets.
    draw_node(&p, Rect::from_min_size(lay.mat.min, vec2(lay.mat.width(), 28.0)), &m.name, m.model.label(), false, false);
    p.rect_stroke(lay.mat, 4.0, Stroke::new(1.0, theme::AMBER), egui::StrokeKind::Inside);
    for ch in MapChannel::ALL {
        let s = lay.socket(ch as usize);
        p.circle_filled(s, 5.0, theme::AMBER);
        p.text(s + vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, ch.label(), egui::FontId::proportional(11.5), theme::TEXT);
    }
    for &(r, rect) in &lay.nodes {
        let out = pos2(rect.right(), rect.center().y);
        match r {
            NodeRef::Wired(c) => wire_curve(&p, out, lay.socket(c), theme::AMBER),
            NodeRef::Input(c) => {
                if let Some(cc) = lay.rect(NodeRef::Wired(c)) {
                    wire_curve(&p, out, pos2(cc.left(), cc.center().y), theme::BLUE);
                }
            }
            NodeRef::Free(_) => {}
        }
    }
    let mut action: Option<(NodeRef, Option<Target>)> = None;
    let mut m2 = m.clone();
    for &(r, rect) in &lay.nodes {
        let Some(n) = node_mut(&mut m2, r) else { continue };
        let sub = match r {
            NodeRef::Free(_) => "unwired",
            NodeRef::Input(_) => "into Color Correction",
            NodeRef::Wired(c) => MapChannel::ALL[c].label(),
        };
        draw_node(&p, rect, n.kind_name(), sub, st.selected == Some(r), true);
        if matches!(n, MapNode::ColorCorrect { .. }) {
            p.circle_filled(pos2(rect.left(), rect.center().y), 5.0, theme::BLUE);
        }
        // One widget per node: a drag that starts on the right-hand output
        // strip pulls a wire, anywhere else moves the node.
        let body = ui.interact(rect.expand2(vec2(6.0, 0.0)), ui.id().with(("slate_node", r)), egui::Sense::click_and_drag());
        let on_socket = |q: Option<Pos2>| q.is_some_and(|q| q.x > rect.right() - 18.0);
        if body.drag_started() && on_socket(ui.input(|i| i.pointer.press_origin())) {
            st.wiring = Some(r);
        }
        if body.clicked() || body.drag_started() {
            st.selected = Some(r);
        }
        if body.dragged() && st.wiring != Some(r) {
            *st.offsets.entry(r).or_default() += body.drag_delta();
        }
        let s = body;
        if s.drag_stopped()
            && let Some(src) = st.wiring.take()
        {
            let at = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            let near = |q: Pos2| q.distance(at) < 14.0;
            let mut dst = (0..6).find(|&c| near(lay.socket(c))).map(Target::Channel);
            if dst.is_none() {
                dst = (0..6)
                    .find(|&c| {
                        matches!(m.maps[c].node, Some(MapNode::ColorCorrect { .. }))
                            && lay.rect(NodeRef::Wired(c)).is_some_and(|rc| near(pos2(rc.left(), rc.center().y)))
                    })
                    .map(Target::CorrectorInput);
            }
            action = Some((src, dst));
        }
    }
    if let (Some(src), Some(at)) = (st.wiring, ui.input(|i| i.pointer.interact_pos()))
        && let Some(rc) = lay.rect(src)
    {
        wire_curve(&p, pos2(rc.right(), rc.center().y), at, theme::TEXT_DIM);
    }
    let mut add = None;
    bg.context_menu(|ui| {
        ui.label(RichText::new("Add Map Node").strong());
        for (i, k) in MapNode::KINDS.iter().enumerate() {
            if ui.button(*k).clicked() {
                add = Some(i);
                ui.close();
            }
        }
    });
    if let Some((src, dst)) = action {
        edit(ed, |m, t| {
            wire(m, src, dst);
            (t.started, t.changed) = (true, true);
            None
        });
        st.selected = None;
        st.offsets.clear();
    } else if let Some(i) = add {
        edit(ed, |m, t| {
            m.slate_nodes.push(MapNode::new_kind(i, None));
            (t.started, t.changed) = (true, true);
            None
        });
    }
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(dim("Drag a node's blue output onto a channel to wire · drop on the canvas to unwire · right-click to add"))
                .wrap(),
        );
    });
    // Fixed-height inspector: selecting a node must not resize the window
    // (egui would push it back on screen and shift the sockets mid-drag).
    let mut delete = None;
    egui::ScrollArea::vertical().id_salt("slate_inspector").auto_shrink([false, false]).max_height(INSPECTOR_H).show(ui, |ui| {
        let Some(sel) = st.selected else {
            ui.label(dim("Select a node to edit its parameters."));
            return;
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new("Selected Node").strong());
            if !matches!(sel, NodeRef::Input(_)) && ui.button("🗑 Delete Node").clicked() {
                delete = Some(sel);
            }
        });
        edit(ed, |m, t| node_mut(m, sel).and_then(|n| node_params(ui, "slate", n, t)));
    });
    if let Some(sel) = delete {
        edit(ed, |m, t| {
            take(m, sel);
            (t.started, t.changed) = (true, true);
            None
        });
        st.selected = None;
        st.offsets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiring_moves_nodes_between_canvas_and_channels() {
        let mut m = Material::default();
        m.slate_nodes.push(MapNode::new_kind(2, None)); // Checker
        wire(&mut m, NodeRef::Free(0), Some(Target::Channel(0)));
        assert!(m.slate_nodes.is_empty());
        assert_eq!(m.maps[0].node.as_ref().unwrap().kind_name(), "Checker");
        // Replacing a channel pushes the old node back on the canvas.
        m.slate_nodes.push(MapNode::new_kind(5, None));
        wire(&mut m, NodeRef::Free(0), Some(Target::Channel(0)));
        assert_eq!(m.maps[0].node.as_ref().unwrap().kind_name(), "Color Correction");
        assert_eq!(m.slate_nodes.len(), 1);
        // Feed the checker into the corrector's input.
        wire(&mut m, NodeRef::Free(0), Some(Target::CorrectorInput(0)));
        let Some(MapNode::ColorCorrect { input, .. }) = &m.maps[0].node else { panic!() };
        assert_eq!(input.kind_name(), "Checker");
        assert_eq!(m.slate_nodes.len(), 1, "the default noise input is displaced");
        // Unwire the channel: the corrector goes back to the canvas.
        wire(&mut m, NodeRef::Wired(0), None);
        assert!(m.maps[0].node.is_none());
        assert_eq!(m.slate_nodes.len(), 2);
        // Moving a wired node onto another channel.
        wire(&mut m, NodeRef::Free(1), Some(Target::Channel(2)));
        wire(&mut m, NodeRef::Wired(2), Some(Target::Channel(4)));
        assert!(m.maps[2].node.is_none() && m.maps[4].node.is_some());
    }
}
