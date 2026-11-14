//! Clouds in the editor: `[ Break Into Clouds ]` turns a Cloud Field into
//! separate Cloud objects (same material, same world placement) so each
//! one can be moved with `W`.

use super::*;
use crate::generators::WorldGen;
use crate::scene::BaseObject;

impl Editor {
    /// Replaces Cloud Field `id` by one Cloud object per placement (one undo
    /// step); returns the new objects.
    pub fn break_cloud_field(&mut self, id: ObjId) -> Vec<ObjId> {
        let Some(o) = self.scene.objects.get(id) else { return Vec::new() };
        let BaseObject::World(WorldGen::CloudField(f)) = &o.base else { return Vec::new() };
        let (placements, xf, mat) = (f.placements(), o.transform, o.material);
        self.checkpoint("Break Into Clouds");
        let mut out = Vec::with_capacity(placements.len());
        for p in placements {
            let at = xf.matrix().transform_point3(p.offset);
            let c = self.scene.add(BaseObject::World(WorldGen::Cloud(Box::new(p.params))), at);
            let n = &mut self.scene.objects[c];
            (n.transform.rotation, n.transform.scale, n.material) = (xf.rotation, xf.scale, mat);
            out.push(c);
        }
        self.scene.delete(id);
        for (k, o) in self.scene.objects.iter_mut() {
            o.selected = out.contains(&k);
        }
        self.status = format!("Broke the Cloud Field into {} clouds", out.len());
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::editor::Editor;
    use crate::generators::{WorldGen, WorldKind};
    use crate::scene::BaseObject;
    use bevy::math::Vec3;

    #[test]
    fn breaking_a_field_keeps_every_cloud_in_place() {
        let mut ed = Editor::default();
        ed.create_world(WorldKind::CloudField, Vec3::new(10.0, 0.0, 5.0));
        let id = ed.current().unwrap();
        let BaseObject::World(WorldGen::CloudField(f)) = &ed.scene.objects[id].base else { panic!() };
        let want = f.placements();
        let mat = ed.scene.objects[id].material;
        let made = ed.break_cloud_field(id);
        assert_eq!(made.len(), want.len());
        assert!(!ed.scene.objects.contains_key(id), "the field is gone");
        for (c, p) in made.iter().zip(&want) {
            let o = &ed.scene.objects[*c];
            assert!(o.selected && o.material == mat);
            assert!(o.transform.translation.distance(p.offset + Vec3::new(10.0, 0.0, 5.0)) < 1e-3);
            assert!(matches!(&o.base, BaseObject::World(WorldGen::Cloud(q)) if **q == p.params));
        }
        ed.undo();
        assert!(ed.scene.objects.contains_key(id) || ed.scene.objects.len() == 1, "undo brings the field back");
    }
}
