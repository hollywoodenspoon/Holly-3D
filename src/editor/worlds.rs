//! Procedural worlds in the editor: placing Terrain / Ocean / Planet objects
//! (each gets its matching Multi/Sub-Object pack so the generated Material
//! IDs render right away) and the Planet Studio custom texture buttons.

use super::*;
use crate::generators::{PlanetPart, WorldGen, WorldKind};
use crate::materials::{MapChannel, Material, MultiPreset};
use crate::scene::BaseObject;

impl Editor {
    /// Create -> Geometry -> Worlds: places the object and assigns a fresh
    /// copy of its Multi/Sub-Object pack.
    pub fn create_world(&mut self, kind: WorldKind, at: Vec3) {
        self.checkpoint("Create World");
        let w = WorldGen::default_of(kind);
        let pack = w.material_pack();
        let id = self.scene.add(BaseObject::World(w), at);
        self.give_pack(id, pack);
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {} with the {}", self.scene.objects[id].name, pack.label());
    }

    fn give_pack(&mut self, id: ObjId, pack: MultiPreset) {
        let mut m = Material::named(pack.label());
        pack.apply(&mut m);
        self.scene.objects[id].material = Some(self.scene.materials.mats.insert(m));
    }

    /// `[ Assign GhostTown Material Pack ]` (and the other packs): a fresh
    /// Multi/Sub-Object copy of `pack` on the current object, one undo step.
    pub fn assign_pack(&mut self, pack: MultiPreset) {
        let Some(id) = self.current() else { return };
        self.checkpoint("Assign Material Pack");
        self.give_pack(id, pack);
        self.status = format!("Assigned the {} to {}", pack.label(), self.scene.objects[id].name);
    }

    /// Replaces the current world object's parameters (one undo step per
    /// interaction, recorded by the caller) and re-evaluates it.
    pub fn set_world(&mut self, id: ObjId, w: WorldGen) {
        let o = &mut self.scene.objects[id];
        if let BaseObject::World(cur) = &mut o.base {
            *cur = w;
        }
        o.evaluate();
        self.scene.propagate_from(id);
    }

    /// `[✓] Glowing Embers` / Ember Glow: sets the emission of the
    /// object's own GhostTown sub-material 25 (0 = embers off). Adds the
    /// row when the object still carries an older 24-ID pack.
    pub fn set_ember_glow(&mut self, id: ObjId, strength: f32) {
        use crate::generators::kit::ids::EMBERS;
        let mid = self.own_multi(id, MultiPreset::GhostTown);
        let Some(ms) = self.scene.materials.mats[mid].multi.as_mut() else { return };
        if !ms.subs.iter().any(|s| s.id == EMBERS) {
            let mat = crate::materials::ghost_pack::materials(false).swap_remove(EMBERS as usize - 1);
            ms.subs.push(crate::materials::SubMaterial { id: EMBERS, name: mat.name.clone(), enabled: true, mat });
        }
        if let Some(s) = ms.subs.iter_mut().find(|s| s.id == EMBERS) {
            s.mat.emissive_strength = strength.max(0.0);
        }
    }

    /// The object's Multi/Sub-Object material, made unique to it (assigns a
    /// fresh `pack` when it has none or a plain material).
    fn own_multi(&mut self, id: ObjId, pack: MultiPreset) -> crate::materials::MatId {
        let shared = |ed: &Self, mid| ed.scene.objects.values().filter(|o| o.material == Some(mid)).count() > 1;
        match self.scene.objects[id].material {
            Some(mid) if self.scene.materials.mats.get(mid).is_some_and(|m| m.is_multi()) && !shared(self, mid) => mid,
            Some(mid) if self.scene.materials.mats.get(mid).is_some_and(|m| m.is_multi()) => {
                let copy = self.scene.materials.mats[mid].clone();
                let n = self.scene.materials.mats.insert(copy);
                self.scene.objects[id].material = Some(n);
                n
            }
            _ => {
                let mut m = Material::named(pack.label());
                pack.apply(&mut m);
                let n = self.scene.materials.mats.insert(m);
                self.scene.objects[id].material = Some(n);
                n
            }
        }
    }

    /// `[ Load Custom Surface Texture ]` / custom cloud map: the image goes
    /// on the layer's sub-material (surface ID 1 color; clouds ID 5 color
    /// and opacity), and the generator switches that layer to texture mode.
    /// `None` returns the layer to its procedural look.
    pub fn planet_custom_texture(&mut self, id: ObjId, part: PlanetPart, path: Option<&std::path::Path>) {
        let BaseObject::World(WorldGen::Planet(p)) = &self.scene.objects[id].base else { return };
        let mut p = p.clone();
        let node = match path.map(crate::materials::import::bitmap_node) {
            Some(Ok(n)) => Some(n),
            Some(Err(e)) => {
                self.status = format!("Could not load texture: {e}");
                return;
            }
            None => None,
        };
        self.checkpoint("Planet Texture");
        let label = path.map(|p| p.to_string_lossy().to_string());
        let mid = self.own_multi(id, MultiPreset::PlanetStudio);
        let (sub_id, channels): (u16, &[MapChannel]) = match part {
            PlanetPart::Surface => (1, &[MapChannel::BaseColor]),
            PlanetPart::Clouds => (5, &[MapChannel::BaseColor, MapChannel::Opacity]),
        };
        if let Some(ms) = self.scene.materials.mats[mid].multi.as_mut()
            && let Some(s) = ms.subs.iter_mut().find(|s| s.id == sub_id)
        {
            for &ch in channels {
                s.mat.map_mut(ch).node = node.clone();
            }
        }
        match part {
            PlanetPart::Surface => p.custom_surface = label,
            PlanetPart::Clouds => p.clouds.custom_map = label,
        }
        self.set_world(id, WorldGen::Planet(p));
        self.status = if path.is_some() { "Custom planet texture loaded".into() } else { "Procedural planet layer restored".into() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_world_assigns_matching_pack() {
        let mut ed = Editor::default();
        for (kind, pack) in [
            (WorldKind::Terrain, MultiPreset::AlpineTerrain),
            (WorldKind::Ocean, MultiPreset::OceanWater),
            (WorldKind::Planet, MultiPreset::PlanetStudio),
        ] {
            ed.create_world(kind, Vec3::ZERO);
            let id = ed.current().unwrap();
            let o = &ed.scene.objects[id];
            let m = &ed.scene.materials.mats[o.material.unwrap()];
            assert_eq!(m.multi, Some(pack.build()), "{kind:?}");
            assert!(o.evaluated_mesh.polygons.len() > 100, "{kind:?} evaluated");
            // Every generated ID has a sub-material in the pack.
            let n = m.multi.as_ref().unwrap().subs.len() as u16;
            assert!(o.evaluated_mesh.polygons.values().all(|p| (1..=n).contains(&p.material_id)), "{kind:?}");
        }
        ed.undo();
        assert_eq!(ed.scene.objects.len(), 2);
    }

    #[test]
    fn ghosttown_pack_assigns_to_any_object() {
        let mut ed = Editor::default();
        ed.create_world(WorldKind::Settlement(crate::generators::Era::Village), Vec3::ZERO);
        let id = ed.current().unwrap();
        let o = &ed.scene.objects[id];
        let m = &ed.scene.materials.mats[o.material.unwrap()];
        assert_eq!(m.multi, Some(MultiPreset::GhostTown.build()));
        // Assigning the pack again (from the modifier rollouts) makes a fresh slot.
        let before = o.material;
        ed.assign_pack(MultiPreset::GhostTown);
        let after = ed.scene.objects[id].material;
        assert!(after.is_some() && after != before);
        ed.undo();
        assert_eq!(ed.scene.objects[id].material, before);
    }

    #[test]
    fn custom_surface_texture_switches_layer_and_back() {
        let mut ed = Editor::default();
        ed.create_world(WorldKind::Planet, Vec3::ZERO);
        let id = ed.current().unwrap();
        let dir = std::env::temp_dir().join(format!("minimax_planet_tex_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("earth.png");
        image::RgbaImage::from_pixel(8, 4, image::Rgba([10, 80, 200, 255])).save(&png).unwrap();
        ed.planet_custom_texture(id, PlanetPart::Surface, Some(&png));
        let o = &ed.scene.objects[id];
        let mat = &ed.scene.materials.mats[o.material.unwrap()];
        assert!(mat.for_id(1).active_map(MapChannel::BaseColor).is_some(), "ID 1 shows the image");
        let BaseObject::World(WorldGen::Planet(p)) = &o.base else { panic!() };
        assert!(p.custom_surface.is_some());
        ed.planet_custom_texture(id, PlanetPart::Surface, None);
        let BaseObject::World(WorldGen::Planet(p)) = &ed.scene.objects[id].base else { panic!() };
        assert!(p.custom_surface.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
