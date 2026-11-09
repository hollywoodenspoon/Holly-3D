//! Viewport Paint: PaintBox actions and brush strokes for the VertexPaint
//! modifier and texture projection paint (one undo step per stroke).

use super::*;
use crate::materials::bake::direct_base;
use crate::materials::{MapChannel, MapNode, MapSlot, MatId, Material, TexHandle, Texture};
use crate::modifiers::{ModifierKind, make_modifier};
use crate::paint::texture::fill_texture;
use crate::paint::{Brush, BrushTool, PaintMode, TextureStroke, VertexStroke, paint_tris, uv_overlap, vertex};
use crate::viewport::picking::pick_polygon;

pub const CANVAS_PATH: &str = "(paint canvas)";

/// Transient stroke state (not part of undo).
pub enum PaintStroke {
    Vertex { obj: ObjId, modifier: usize, stroke: VertexStroke, last: Vec3 },
    Texture { mat: MatId, stroke: TextureStroke, last: Vec3, view_dir: Vec3 },
}

fn srgb_byte(c: f32) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn canvas_mut(m: &mut Material) -> Option<&mut TexHandle> {
    match &mut m.map_mut(MapChannel::BaseColor).node {
        Some(MapNode::Bitmap { tex: Some(h), .. }) => Some(h),
        _ => None,
    }
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
                PaintMode::Texture => "Viewport Paint: drag on the selected object to paint its canvas (right-click exits)".into(),
            };
        } else {
            self.status = "Viewport Paint off".into();
        }
    }

    /// Material whose full-strength Base Color bitmap is the paint canvas.
    pub fn canvas_of(&self, id: ObjId) -> Option<MatId> {
        let m = self.scene.objects.get(id)?.material?;
        direct_base(self.scene.materials.mats.get(m)?).map(|_| m)
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

    /// World hit point under the cursor on the current object.
    pub fn paint_hit(&self, view: usize, cursor: Vec2) -> Option<Vec3> {
        let o = self.current_obj().filter(|o| !o.is_helper() && !o.hidden)?;
        let ray = self.viewports.views[view].ray(cursor);
        pick_polygon(&o.evaluated_mesh, o.world_matrix(), &ray, false).map(|h| h.point)
    }

    /// Left-button press: starts a stroke (one undo step) and stamps once.
    pub fn paint_begin(&mut self, view: usize, cursor: Vec2) {
        let Some(id) = self.current() else {
            self.status = "Viewport Paint: select an object to paint".into();
            return;
        };
        let Some(hit) = self.paint_hit(view, cursor) else { return };
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
                let Some(mat) = self.canvas_of(id) else {
                    self.status = "Texture Paint: no canvas — press Auto-Setup Paint Canvas in the PaintBox".into();
                    return;
                };
                self.checkpoint("Texture Paint");
                let o = &self.scene.objects[id];
                let tris = paint_tris(&o.evaluated_mesh, o.world_matrix(), self.paint.selected_only);
                let Some(h) = canvas_mut(&mut self.scene.materials.mats[mat]) else { return };
                let stroke = TextureStroke::new(tris, &h.tex);
                self.paint_stroke = Some(PaintStroke::Texture { mat, stroke, last: hit, view_dir });
            }
        }
        self.stamp_at(hit, None);
    }

    /// Mouse move with the button held: stamps along the path.
    pub fn paint_drag(&mut self, view: usize, cursor: Vec2) {
        if self.paint_stroke.is_none() {
            return;
        }
        if let Some(hit) = self.paint_hit(view, cursor) {
            let dir = self.viewports.views[view].ray(cursor).dir;
            self.stamp_at(hit, Some(dir));
        }
    }

    pub fn paint_end(&mut self) {
        self.paint_stroke = None;
    }

    /// Stamps from the stroke's last point to `hit`, spaced by the brush.
    fn stamp_at(&mut self, hit: Vec3, dir: Option<Vec3>) {
        let brush = self.paint.brush.clone();
        let Some(stroke) = &mut self.paint_stroke else { return };
        let last = match stroke {
            PaintStroke::Vertex { last, .. } | PaintStroke::Texture { last, .. } => last,
        };
        let from = std::mem::replace(last, hit);
        let steps = ((from.distance(hit) / (brush.size * 0.3).max(1e-4)).ceil() as usize).clamp(1, 24);
        let points: Vec<Vec3> = (1..=steps).map(|k| from.lerp(hit, k as f32 / steps as f32)).collect();
        match stroke {
            PaintStroke::Vertex { obj, modifier, stroke, .. } => {
                let Some(o) = self.scene.objects.get_mut(*obj) else { return };
                if let Some(ModifierKind::VertexPaint(vp)) = o.modifiers.get_mut(*modifier).map(|m| &mut m.kind) {
                    for p in points {
                        stroke.stamp(&mut vp.layer, p, &brush);
                    }
                    o.evaluate();
                }
            }
            PaintStroke::Texture { mat, stroke, view_dir, .. } => {
                if let Some(d) = dir {
                    *view_dir = d;
                }
                let Some(h) = self.scene.materials.mats.get_mut(*mat).and_then(canvas_mut) else { return };
                let tex = h.make_mut();
                for p in points {
                    stroke.stamp(tex, p, *view_dir, &brush);
                }
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
                let Some(mat) = self.canvas_of(id) else { return };
                self.checkpoint("Fill Canvas");
                let o = &self.scene.objects[id];
                let tris = paint_tris(&o.evaluated_mesh, o.world_matrix(), masked);
                let Some(h) = canvas_mut(&mut self.scene.materials.mats[mat]) else { return };
                let tex = h.make_mut();
                if masked {
                    let brush = Brush { size: 1e9, opacity, falloff: 0.0, color, tool: BrushTool::Paint };
                    TextureStroke::new(tris, tex).stamp(tex, Vec3::ZERO, Vec3::ZERO, &brush);
                } else {
                    fill_texture(tex, color, opacity);
                }
                self.status = "Fill: canvas filled".into();
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

    /// `[ Auto-Setup Paint Canvas ]`: Smart UV Project when the UVs overlap,
    /// then a fresh canvas bitmap on a material unique to the object.
    pub fn auto_setup_canvas(&mut self) {
        let Some(id) = self.current().filter(|&id| !self.scene.objects[id].is_helper()) else {
            self.status = "Auto-Setup Paint Canvas: select a geometry object".into();
            return;
        };
        self.checkpoint("Auto-Setup Paint Canvas");
        let mut notes = vec![];
        let o = &mut self.scene.objects[id];
        if uv_overlap(&o.evaluated_mesh) > 0.02 {
            o.active_level = o.modifiers.len().checked_sub(1);
            o.add_modifier(make_modifier("Unwrap UVW").unwrap());
            let i = o.active_level.unwrap();
            let input = o.input_of(i);
            if let ModifierKind::Unwrap(u) = &mut o.modifiers[i].kind {
                match u.smart_unwrap(&input) {
                    Ok(n) => notes.push(format!("Smart UV Project: {n} charts")),
                    Err(e) => notes.push(e),
                }
            }
            o.evaluate();
        }
        let size = self.paint.canvas_size;
        let mut m = o
            .material
            .and_then(|m| self.scene.materials.mats.get(m).cloned())
            .unwrap_or_else(|| Material { base_color: o.wire_color, ..Material::named("Paint") });
        let fill = [srgb_byte(m.base_color[0]), srgb_byte(m.base_color[1]), srgb_byte(m.base_color[2]), 255];
        m.name = format!("{} Paint", o.name);
        m.show_in_viewport = true;
        *m.map_mut(MapChannel::BaseColor) = MapSlot {
            enabled: true,
            amount: 100.0,
            node: Some(MapNode::Bitmap { path: CANVAS_PATH.into(), tex: Some(TexHandle::new(Texture::new(size, size, fill))) }),
        };
        m.map_mut(MapChannel::Opacity).node = None;
        let mid = self.scene.materials.mats.insert(m);
        self.scene.objects[id].material = Some(mid);
        self.paint.mode = PaintMode::Texture;
        notes.push(format!("{size}x{size} canvas ready"));
        self.status = format!("Auto-Setup Paint Canvas: {}", notes.join(" · "));
    }

    /// Saves the current object's canvas as PNG (and points the bitmap at it).
    pub fn save_canvas(&mut self, path: &str) {
        let Some(mat) = self.current().and_then(|id| self.canvas_of(id)) else {
            self.status = "Save Canvas: the selected object has no paint canvas".into();
            return;
        };
        let m = &mut self.scene.materials.mats[mat];
        let Some(h) = canvas_mut(m) else { return };
        let t = &h.tex;
        self.status = match image::save_buffer(path, &t.bytes(), t.width, t.height, image::ExtendedColorType::Rgba8) {
            Ok(()) => {
                if let Some(MapNode::Bitmap { path: p, .. }) = &mut m.map_mut(MapChannel::BaseColor).node {
                    *p = path.to_string();
                }
                format!("Canvas saved to {path}")
            }
            Err(e) => format!("Save Canvas failed: {e}"),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::PrimitiveKind;

    #[test]
    fn vertex_stroke_adds_modifier_and_undoes_in_one_step() {
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        let id = ed.current().unwrap();
        let top = ed.scene.objects[id].world_matrix().transform_point3(Vec3::new(0.0, 0.0, 1.0));
        ed.paint.mode = PaintMode::Vertex;
        ed.checkpoint("Vertex Paint");
        let i = ed.vertex_paint_modifier(true).unwrap();
        let o = &ed.scene.objects[id];
        let stroke = VertexStroke::new(&o.input_of(i), o.world_matrix(), &Default::default(), false);
        ed.paint_stroke = Some(PaintStroke::Vertex { obj: id, modifier: i, stroke, last: top });
        ed.paint.brush.size = 2.0;
        ed.stamp_at(top, None);
        ed.paint_end();
        let o = &ed.scene.objects[id];
        assert_eq!(o.modifiers.len(), 1);
        assert!(o.evaluated_mesh.vertices.values().any(|v| v.color.x > v.color.y));
        ed.undo();
        assert!(ed.scene.objects[id].modifiers.is_empty());
    }

    #[test]
    fn auto_setup_unwraps_box_and_attaches_canvas() {
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        let id = ed.current().unwrap();
        ed.paint.canvas_size = 64;
        ed.auto_setup_canvas();
        let o = &ed.scene.objects[id];
        assert!(matches!(o.modifiers.last().map(|m| &m.kind), Some(ModifierKind::Unwrap(_))));
        assert!(uv_overlap(&o.evaluated_mesh) < 0.02);
        assert!(ed.canvas_of(id).is_some());
        ed.paint_fill();
        let mat = ed.canvas_of(id).unwrap();
        let tex = &direct_base(&ed.scene.materials.mats[mat]).unwrap().tex;
        assert_eq!(tex.width, 64);
        assert!(tex.data.iter().any(|t| t[0] > t[1]), "canvas filled with the red brush color");
    }
}
