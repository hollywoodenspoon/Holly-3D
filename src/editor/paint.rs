//! Viewport Paint: PaintBox actions and brush strokes for the VertexPaint
//! modifier and the texture layer painter (one undo step per stroke).

use super::*;
use crate::materials::MatId;
use crate::modifiers::{ModifierKind, make_modifier};
use crate::paint::{Alpha, Brush, BrushTool, LayerStroke, PaintMode, Source, Stamp, VertexStroke, paint_tris, vertex};
use crate::viewport::picking::pick_polygon;

/// Transient stroke state (not part of undo).
pub enum PaintStroke {
    Vertex { obj: ObjId, modifier: usize, stroke: VertexStroke, last: Vec3 },
    Layer { mat: MatId, stroke: LayerStroke, source: Source, last: Vec3, view_dir: Vec3, first: bool },
}

impl Editor {
    /// `[ 🖌 Paint ]` toolbar toggle.
    pub fn toggle_paint(&mut self) {
        self.paint.active = !self.paint.active;
        self.paint_stroke = None;
        if self.paint.active {
            self.tool = Tool::None;
            self.status = match self.paint.mode {
                PaintMode::Vertex => "Viewport Paint: drag on the selected object to paint vertex colors (right-click exits)".into(),
                PaintMode::Texture => "Viewport Paint: drag on the selected object to paint its active layer (right-click exits)".into(),
            };
        } else {
            self.status = "Viewport Paint off".into();
        }
    }

    /// Topmost VertexPaint modifier of the current object (adds one on top).
    pub fn vertex_paint_modifier(&mut self, create: bool) -> Option<usize> {
        let o = self.current_obj_mut().filter(|o| !o.is_helper())?;
        if let Some(i) = o.modifiers.iter().rposition(|m| matches!(m.kind, ModifierKind::VertexPaint(_))) {
            return Some(i);
        }
        if !create {
            return None;
        }
        o.active_level = o.modifiers.len().checked_sub(1);
        o.add_modifier(make_modifier("VertexPaint")?);
        o.active_level
    }

    /// World hit point and surface normal under the cursor on the current object.
    pub fn paint_hit(&self, view: usize, cursor: Vec2) -> Option<(Vec3, Vec3)> {
        let o = self.current_obj().filter(|o| !o.is_helper() && !o.hidden)?;
        let ray = self.viewports.views[view].ray(cursor);
        let h = pick_polygon(&o.evaluated_mesh, o.world_matrix(), &ray, false)?;
        let nm = bevy::math::Mat3::from_mat4(o.world_matrix()).inverse().transpose();
        let mut n = (nm * o.evaluated_mesh.poly_normal_raw(h.poly)).normalize_or(-ray.dir);
        if n.dot(ray.dir) > 0.0 {
            n = -n;
        }
        Some((h.point, n))
    }

    /// Left-button press: starts a stroke (one undo step) and stamps once.
    pub fn paint_begin(&mut self, view: usize, cursor: Vec2) {
        let Some(id) = self.current() else {
            self.status = "Viewport Paint: select an object to paint".into();
            return;
        };
        let Some((hit, normal)) = self.paint_hit(view, cursor) else { return };
        let view_dir = self.viewports.views[view].ray(cursor).dir;
        match self.paint.mode {
            PaintMode::Vertex => {
                self.checkpoint("Vertex Paint");
                let Some(i) = self.vertex_paint_modifier(true) else { return };
                let o = &self.scene.objects[id];
                let ModifierKind::VertexPaint(vp) = &o.modifiers[i].kind else { return };
                let stroke = VertexStroke::new(&o.input_of(i), o.world_matrix(), &vp.layer, self.paint.selected_only);
                self.paint_stroke = Some(PaintStroke::Vertex { obj: id, modifier: i, stroke, last: hit });
            }
            PaintMode::Texture => {
                self.checkpoint("Texture Paint");
                if !self.begin_layer_stroke(id, hit, view_dir) {
                    self.history.discard_last();
                    return;
                }
            }
        }
        self.stamp_at(hit, normal, None);
    }

    /// Sets up a stroke on the active paint layer of object `id`.
    pub fn begin_layer_stroke(&mut self, id: ObjId, hit: Vec3, view_dir: Vec3) -> bool {
        let Some(mat) = self.paint_target(id) else { return false };
        let o = &self.scene.objects[id];
        let tris = paint_tris(&o.evaluated_mesh, o.world_matrix(), self.paint.selected_only);
        let source = self.paint.source();
        let stack = self.scene.materials.mats[mat].paint.as_mut().expect("paint target has a stack");
        let size = stack.size;
        let Some(layer) = stack.active_layer_mut() else {
            self.status = "Texture Paint: add a paint layer first".into();
            return false;
        };
        if !layer.visible {
            self.status = format!("Texture Paint: layer \"{}\" is hidden", layer.name);
            return false;
        }
        let stroke = LayerStroke::new(tris, layer, size, crate::scene::next_version() as u32);
        self.paint_stroke = Some(PaintStroke::Layer { mat, stroke, source, last: hit, view_dir, first: true });
        true
    }

    /// Mouse move with the button held: stamps along the path.
    pub fn paint_drag(&mut self, view: usize, cursor: Vec2) {
        if self.paint_stroke.is_none() {
            return;
        }
        if let Some((hit, normal)) = self.paint_hit(view, cursor) {
            let dir = self.viewports.views[view].ray(cursor).dir;
            self.stamp_at(hit, normal, Some(dir));
        }
    }

    pub fn paint_end(&mut self) {
        self.paint_stroke = None;
    }

    /// Stamps from the stroke's last point to `hit`, spaced by the brush.
    pub fn stamp_at(&mut self, hit: Vec3, normal: Vec3, dir: Option<Vec3>) {
        let brush = self.paint.brush.clone();
        let Some(stroke) = &mut self.paint_stroke else { return };
        match stroke {
            PaintStroke::Vertex { obj, modifier, stroke, last } => {
                let from = std::mem::replace(last, hit);
                let steps = ((from.distance(hit) / (brush.size * 0.3).max(1e-4)).ceil() as usize).clamp(1, 24);
                let Some(o) = self.scene.objects.get_mut(*obj) else { return };
                if let Some(ModifierKind::VertexPaint(vp)) = o.modifiers.get_mut(*modifier).map(|m| &mut m.kind) {
                    for k in 1..=steps {
                        stroke.stamp(&mut vp.layer, from.lerp(hit, k as f32 / steps as f32), &brush);
                    }
                    o.evaluate();
                }
            }
            PaintStroke::Layer { mat, stroke, source, last, view_dir, first } => {
                if let Some(d) = dir {
                    *view_dir = d;
                }
                // Stamps sit a fixed distance apart; the rest of the move
                // carries over to the next mouse event.
                let step = (brush.size * brush.alpha.spacing()).max(1e-4);
                let dist = last.distance(hit);
                let count = if *first { 1 } else { ((dist / step).floor() as usize).min(48) };
                if count == 0 {
                    return;
                }
                let travel = (hit - *last).normalize_or_zero();
                let Some(stack) = self.scene.materials.mats.get_mut(*mat).and_then(|m| m.paint.as_mut()) else { return };
                let Some(layer) = stack.active_layer_mut() else { return };
                for k in 1..=count {
                    let center = if *first { hit } else { *last + travel * step * k as f32 };
                    stroke.stamp(layer, &Stamp { center, normal, tangent: travel, view_dir: *view_dir }, &brush, source);
                }
                *last = if *first { hit } else { *last + travel * step * count as f32 };
                *first = false;
            }
        }
    }

    /// PaintBox `[ Fill Object ]` with the brush color and opacity.
    pub fn paint_fill(&mut self) {
        let Some(id) = self.current().filter(|&id| !self.scene.objects[id].is_helper()) else { return };
        let (color, opacity, masked) = (self.paint.brush.color, self.paint.brush.opacity, self.paint.selected_only);
        match self.paint.mode {
            PaintMode::Vertex => {
                self.checkpoint("Fill Vertex Colors");
                let Some(i) = self.vertex_paint_modifier(true) else { return };
                let o = &mut self.scene.objects[id];
                let input = o.input_of(i);
                if let ModifierKind::VertexPaint(vp) = &mut o.modifiers[i].kind {
                    let n = vertex::fill(&input, &mut vp.layer, color, opacity, masked);
                    self.status = format!("Fill: {n} vertices painted");
                }
                o.evaluate();
            }
            PaintMode::Texture => {
                self.checkpoint("Fill Layer");
                if !self.begin_layer_stroke(id, Vec3::ZERO, Vec3::ZERO) {
                    self.history.discard_last();
                    return;
                }
                // One flood stamp: no facing test, every texel of the mesh.
                let flood = Brush { size: 1e9, opacity, falloff: 0.0, color, tool: self.paint.brush.tool, alpha: Alpha::HardRound };
                if let Some(PaintStroke::Layer { mat, mut stroke, source, .. }) = self.paint_stroke.take()
                    && let Some(layer) = self.scene.materials.mats[mat].paint.as_mut().and_then(|s| s.active_layer_mut())
                {
                    let flood = Brush { tool: if flood.tool == BrushTool::Blur { BrushTool::Paint } else { flood.tool }, ..flood };
                    let n = stroke.stamp(layer, &Stamp { center: Vec3::ZERO, normal: Vec3::ZERO, tangent: Vec3::X, view_dir: Vec3::ZERO }, &flood, &source);
                    self.status = format!("Fill: {n} texels of layer \"{}\"", layer.name);
                }
            }
        }
    }

    /// Clears the VertexPaint layer of the current object.
    pub fn clear_vertex_paint(&mut self) {
        let Some(i) = self.vertex_paint_modifier(false) else { return };
        self.checkpoint("Clear Vertex Paint");
        let o = self.current_obj_mut().unwrap();
        if let ModifierKind::VertexPaint(vp) = &mut o.modifiers[i].kind {
            vp.layer.clear();
        }
        o.evaluate();
    }
}
