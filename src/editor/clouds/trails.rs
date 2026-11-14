//! Trails in the editor: `[ Pick Path Spline ]` copies a spline's shape
//! into the selected trail, in the trail's own space, source first.

use crate::editor::*;
use crate::generators::WorldGen;
use crate::scene::BaseObject;

impl Editor {
    pub(crate) fn pick_trail_path_click(&mut self, view: usize, cursor: Vec2) {
        let Some(sid) = self.pick_object(view, cursor) else { return };
        let Some(id) = self.current() else { return };
        self.tool = Tool::None;
        if !self.set_trail_path(id, sid) {
            self.status = "Pick Path Spline: pick a shape".into();
        }
    }

    /// Makes trail `id` follow the first spline of shape `sid` (one undo
    /// step); false when either isn't what it should be.
    pub fn set_trail_path(&mut self, id: ObjId, sid: ObjId) -> bool {
        let (Some(o), Some(s)) = (self.scene.objects.get(id), self.scene.objects.get(sid)) else { return false };
        let BaseObject::World(WorldGen::Trail(p)) = &o.base else { return false };
        let Some(pl) = s.shape.as_ref().and_then(|sh| sh.polylines().into_iter().next()) else { return false };
        let to_local = o.world_matrix().inverse() * s.world_matrix();
        let mut path: Vec<Vec3> = pl.pts.iter().map(|&q| to_local.transform_point3(q)).collect();
        if pl.closed {
            path.push(path[0]);
        }
        if path.len() < 2 {
            return false;
        }
        let mut p = p.clone();
        p.path = path;
        self.checkpoint("Pick Trail Path");
        self.set_world(id, WorldGen::Trail(p));
        self.status = format!("The trail follows {}", self.scene.objects[sid].name);
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::editor::Editor;
    use crate::generators::trails::TrailKind;
    use crate::generators::{WorldGen, WorldKind};
    use crate::scene::BaseObject;
    use crate::splines::{ShapeKind, ShapeParams};
    use bevy::math::Vec3;

    #[test]
    fn picking_a_spline_sets_the_path_in_trail_space() {
        let mut ed = Editor::default();
        ed.create_world(WorldKind::Trail(TrailKind::DamageSmoke), Vec3::new(10.0, 0.0, 0.0));
        let trail = ed.current().unwrap();
        ed.create_shape(ShapeKind::Circle, 0, Vec3::new(10.0, 0.0, 50.0));
        let circle = ed.current().unwrap();
        assert!(!ed.set_trail_path(circle, trail), "only a trail takes a path");
        assert!(ed.set_trail_path(trail, circle));
        let BaseObject::World(WorldGen::Trail(p)) = &ed.scene.objects[trail].base else { panic!() };
        assert!(p.path.len() > 8);
        assert!(p.path.iter().all(|q| (q.z - 50.0).abs() < 30.0), "in the trail's space, around the circle");
        assert!(!ed.scene.objects[trail].evaluated_mesh.polygons.is_empty());
        ed.undo();
        let BaseObject::World(WorldGen::Trail(p)) = &ed.scene.objects[trail].base else { panic!() };
        assert!(p.path.is_empty(), "one undo step");
        let _ = ShapeParams::default_of(ShapeKind::Circle);
    }
}
