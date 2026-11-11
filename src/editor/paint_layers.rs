//! Texture paint layers of the selected object: making its material
//! paintable without touching its UVs or its look, layer commands, mesh
//! map baking, Smart Mask layers and texture export.

use super::*;
use crate::materials::{MatId, Material};
use crate::modifiers::{ModifierKind, make_modifier};
use crate::paint::bake::GeoMap;
use crate::paint::layers::composite::Composite;
use crate::paint::{LayerKind, MeshMaps, PaintLayer, PaintMode, PaintStack, SmartKind, SmartMask, Source, has_uvs, paint_tris, uv_overlap};
use crate::scene::BaseObject;
use std::sync::Arc;

/// Geometry map the composite of `m` on object `o` needs (`None` when
/// the material is a plain UV-mapped one).
pub fn geo_for(o: &SceneObject, m: &Material, size: u32) -> Option<GeoMap> {
    let triplanar = m.projection.on() || m.multi.as_ref().is_some_and(|ms| ms.subs.iter().any(|s| s.mat.projection.on()));
    if !triplanar && !m.is_multi() {
        return None;
    }
    // Projection space: the object's own space at world scale.
    let (scale, _, _) = o.world_matrix().to_scale_rotation_translation();
    let tris = paint_tris(&o.evaluated_mesh, bevy::math::Mat4::from_scale(scale), false);
    Some(GeoMap::build(&tris, size, triplanar))
}

impl Editor {
    /// Paint stack of the current object's material.
    pub fn paint_stack(&self) -> Option<&PaintStack> {
        let m = self.current_obj()?.material?;
        self.scene.materials.mats.get(m)?.paint.as_ref()
    }

    fn paint_stack_mut(&mut self) -> Option<&mut PaintStack> {
        let m = self.current_obj()?.material?;
        self.scene.materials.mats.get_mut(m)?.paint.as_mut()
    }

    /// True when the object's UVs were made on purpose (a UVW Map or
    /// Unwrap UVW modifier, or an imported / collapsed mesh with UVs).
    fn has_authored_uvs(o: &SceneObject) -> bool {
        o.modifiers.iter().any(|m| matches!(m.kind, ModifierKind::UvwMap(_) | ModifierKind::Unwrap(_)))
            || (matches!(o.base, BaseObject::EditablePoly(_)) && has_uvs(&o.evaluated_mesh))
    }

    /// Makes object `id` paintable and returns its material:
    /// - existing UVs are kept exactly; only an object without usable UVs
    ///   gets a Smart UV Project;
    /// - its material stays as "Layer 0: Base Material" (a material shared
    ///   with other objects is copied first) under a new empty paint layer.
    pub fn paint_target(&mut self, id: ObjId) -> Option<MatId> {
        let o = self.scene.objects.get(id).filter(|o| !o.is_helper())?;
        let mut notes = vec![];
        let authored = Self::has_authored_uvs(o);
        let overlap = uv_overlap(&o.evaluated_mesh);
        if !authored && overlap > 0.02 {
            let o = &mut self.scene.objects[id];
            o.active_level = o.modifiers.len().checked_sub(1);
            o.add_modifier(make_modifier("Unwrap UVW")?);
            let i = o.active_level?;
            let input = o.input_of(i);
            if let ModifierKind::Unwrap(u) = &mut o.modifiers[i].kind {
                match u.smart_unwrap(&input) {
                    Ok(n) => notes.push(format!("no UVs found, Smart UV Project made {n} charts")),
                    Err(e) => notes.push(e),
                }
            }
            o.evaluate();
        } else if overlap > 0.02 {
            notes.push("existing UVs kept (they overlap, so paint repeats where they do)".into());
        }
        let o = &self.scene.objects[id];
        let shared = |m: MatId| self.scene.objects.iter().any(|(k, x)| k != id && x.material == Some(m));
        let mat = match o.material.filter(|m| self.scene.materials.mats.contains_key(*m)) {
            Some(m) if self.scene.materials.mats[m].paint.is_some() => m,
            Some(m) if !shared(m) => m,
            other => {
                let mut m = other.map_or_else(|| Material { base_color: o.wire_color, ..Material::named("Paint") }, |m| self.scene.materials.mats[m].clone());
                m.name = format!("{} Paint", o.name);
                m.show_in_viewport = true;
                let mid = self.scene.materials.mats.insert(m);
                self.scene.objects[id].material = Some(mid);
                mid
            }
        };
        let m = &mut self.scene.materials.mats[mat];
        if m.paint.is_none() {
            m.paint = Some(PaintStack::new(self.paint.canvas_size));
            notes.push(format!("\"{}\" kept as Layer 0: Base Material, Paint Layer added on top", m.name));
        }
        if !notes.is_empty() {
            self.status = format!("Texture Paint: {}", notes.join(" · "));
        }
        Some(mat)
    }

    /// `[ Auto-Setup Paint Canvas ]`: makes the selected object paintable
    /// (one undo step).
    pub fn auto_setup_canvas(&mut self) {
        let Some(id) = self.current().filter(|&id| !self.scene.objects[id].is_helper()) else {
            self.status = "Auto-Setup Paint Canvas: select a geometry object".into();
            return;
        };
        self.checkpoint("Auto-Setup Paint Canvas");
        if self.paint_target(id).is_some() {
            self.paint.mode = PaintMode::Texture;
        }
    }

    /// Runs `f` on the current object's paint stack as one undo step
    /// (making the object paintable first).
    pub fn edit_paint_stack(&mut self, label: &str, f: impl FnOnce(&mut PaintStack)) {
        let Some(id) = self.current() else { return };
        self.checkpoint(label);
        if self.paint_target(id).is_none() {
            self.history.discard_last();
            return;
        }
        if let Some(s) = self.paint_stack_mut() {
            f(s);
            s.touch();
        }
    }

    /// `[ + Paint Layer ]`: a transparent layer above the active one.
    pub fn add_paint_layer(&mut self) {
        self.edit_paint_stack("Add Paint Layer", |s| {
            let layer = PaintLayer::paint(s.unique_name("Paint Layer"), s.size);
            s.add(layer);
        });
    }

    /// `[ + Fill Layer ]`: the brush colour or material over everything.
    pub fn add_fill_layer(&mut self) {
        let source = self.paint.source();
        self.edit_paint_stack("Add Fill Layer", |s| {
            let layer = PaintLayer::fill(s.unique_name(&format!("Fill: {}", source.label())), source, None);
            s.add(layer);
        });
    }

    pub fn remove_paint_layer(&mut self, i: usize) {
        self.edit_paint_stack("Delete Layer", |s| s.remove(i));
    }

    pub fn shift_paint_layer(&mut self, i: usize, up: bool) {
        self.edit_paint_stack("Move Layer", |s| s.shift(i, up));
    }

    /// Picks the layer strokes go to (not an undo step).
    pub fn set_active_paint_layer(&mut self, i: usize) {
        if let Some(s) = self.paint_stack_mut() {
            s.active = i.min(s.layers.len().saturating_sub(1));
        }
    }

    /// `[ 🔥 Bake Mesh Maps ]`: Curvature, Cavity AO, World Normal and
    /// Z-Height of the selected object, kept with its paint stack.
    pub fn bake_mesh_maps(&mut self) -> Option<Arc<MeshMaps>> {
        let id = self.current()?;
        self.paint_target(id)?;
        let o = &self.scene.objects[id];
        let mut maps = MeshMaps::bake(&o.evaluated_mesh, o.world_matrix(), &self.paint.bake);
        maps.version = o.version;
        let (size, cover) = (maps.size, maps.coverage() * 100.0);
        let maps = Arc::new(maps);
        self.paint_stack_mut()?.mesh_maps = Some(maps.clone());
        self.status = format!("Bake Mesh Maps: Curvature, Cavity AO, World Normal and Z-Height at {size}x{size} ({cover:.0}% of the UV square used)");
        Some(maps)
    }

    /// Mesh maps of the current object, baked now if missing or stale.
    fn fresh_mesh_maps(&mut self) -> Option<Arc<MeshMaps>> {
        let version = self.current_obj()?.version;
        match self.paint_stack().and_then(|s| s.mesh_maps.clone()) {
            Some(m) if m.version == version && m.size == self.paint.bake.size => Some(m),
            _ => self.bake_mesh_maps(),
        }
    }

    /// 1-click Smart Mask layer: a fill layer masked by the mesh maps.
    pub fn add_smart_layer(&mut self, kind: SmartKind, look: usize) {
        let Some(id) = self.current() else { return };
        self.checkpoint("Add Smart Layer");
        if self.paint_target(id).is_none() {
            self.history.discard_last();
            return;
        }
        let Some(maps) = self.fresh_mesh_maps() else { return };
        let smart = SmartMask { look, seed: crate::scene::next_version() as u32, ..SmartMask::new(kind) };
        let name = smart.look_name();
        let mut layer = PaintLayer::fill(name, Source::Flat(smart.fill()), Some(smart.build(&maps)));
        layer.smart = Some(smart);
        if let Some(s) = self.paint_stack_mut() {
            layer.name = s.unique_name(name);
            s.add(layer);
        }
        self.status = format!("{}: \"{name}\" layer added from the baked mesh maps", kind.label());
    }

    /// Rebuilds the mask of smart layer `i` after its settings changed
    /// (the caller records the undo step).
    pub fn rebuild_smart_layer(&mut self, i: usize) {
        let Some(maps) = self.fresh_mesh_maps() else { return };
        let Some(s) = self.paint_stack_mut() else { return };
        if let Some(l) = s.layers.get_mut(i)
            && let (Some(smart), LayerKind::Fill { mask, .. }) = (&l.smart, &mut l.kind)
        {
            *mask = Some(smart.build(&maps));
        }
        s.touch();
    }

    /// `[ 💾 Export Textures ]`: the painted material flattened to PNGs
    /// (base colour, ORM, normal, emission) plus any baked mesh maps.
    pub fn export_paint_textures(&mut self, dir: &str) {
        let Some(o) = self.current_obj() else { return };
        let Some(m) = o.material.and_then(|m| self.scene.materials.mats.get(m)).filter(|m| m.paint.is_some()) else {
            self.status = "Export Textures: the selected object has no paint layers".into();
            return;
        };
        let stack = m.paint.as_ref().expect("filtered above");
        let comp = Composite::build(m, geo_for(o, m, stack.size).as_ref());
        let name: String = o.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let dir = std::path::Path::new(if dir.is_empty() { "." } else { dir });
        let write = |kind: &str, t: &crate::materials::Texture| {
            let path = dir.join(format!("{name}_{kind}.png"));
            image::save_buffer(&path, &t.bytes(), t.width, t.height, image::ExtendedColorType::Rgba8).map_err(|e| format!("{}: {e}", path.display()))
        };
        let mut count = 0;
        let result = std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display())).and_then(|()| {
            for (kind, t) in comp.images() {
                write(kind, t)?;
                count += 1;
            }
            for (kind, t) in stack.mesh_maps.as_ref().map(|mm| mm.images()).into_iter().flatten() {
                write(kind, &t)?;
                count += 1;
            }
            Ok(())
        });
        self.status = match result {
            Ok(()) => format!("Export Textures: {count} PNGs written to {}", dir.display()),
            Err(e) => format!("Export Textures failed: {e}"),
        };
    }
}
