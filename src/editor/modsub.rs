//! Modifier sub-objects that are not mesh components: FFD Control Points /
//! Lattice, the UVW Map / Bend / Twist / Taper Gizmo and Center, the Symmetry
//! Mirror and the Lathe Axis (selection, W/E/R frames, drags, align tools).

use super::select::{SelectOp, VERTEX_PICK_PX};
use super::*;
use crate::mesh::EditablePoly;
use crate::modifiers::{ModSub, ModifierKind};
use crate::viewport::picking::{in_rect, pick_polygon};
use bevy::math::{Mat4, Quat};

impl Editor {
    /// The modifier sub-object mode in effect (valid for the active stack level).
    pub fn active_mod_sub(&self) -> Option<ModSub> {
        let kind = self.current_obj()?.active_modifier()?;
        // Edit Normals is always in its Normal sub-object (as in 3ds Max)
        if matches!(kind, ModifierKind::EditNormals(_)) {
            return Some(ModSub::Normal);
        }
        let s = self.mod_sub?;
        kind.gizmo_subs().contains(&s).then_some(s)
    }

    /// Keys 1/2… on a modifier with gizmo sub-objects; pressing the active one again leaves it.
    pub fn set_mod_sub(&mut self, s: Option<ModSub>) {
        self.mod_sub = if s.is_some() && self.active_mod_sub() == s { None } else { s };
        self.sub = SubObject::Object;
        self.tool = Tool::None;
        self.tab = CommandTab::Modify;
    }

    /// Edits the active modifier with its input mesh, recording undo.
    pub fn edit_active_modifier(&mut self, label: &str, f: impl FnOnce(&mut ModifierKind, &EditablePoly)) {
        let Some(id) = self.current() else { return };
        let Some(level) = self.scene.objects[id].active_level else { return };
        self.checkpoint(label);
        let input = self.scene.objects[id].input_of(level);
        let o = &mut self.scene.objects[id];
        if let Some(kind) = o.active_modifier_mut() {
            f(kind, &input);
        }
        o.evaluate();
        self.scene.propagate_from(id);
    }

    /// World-space control points of the active FFD (for drawing / picking).
    pub fn ffd_points_world(&self) -> Option<(Vec<Vec3>, Vec<bool>)> {
        let o = self.current_obj()?;
        let ModifierKind::Ffd(f) = o.active_modifier()? else { return None };
        let w = o.world_matrix();
        Some(((0..f.points.len()).map(|i| w.transform_point3(f.point_pos(i))).collect(), f.selected.clone()))
    }

    /// Gizmo frame for W/E/R while a modifier sub-object is active.
    pub fn mod_sub_frame(&self) -> Option<Xform> {
        let sub = self.active_mod_sub()?;
        let o = self.current_obj()?;
        let w = o.world_matrix();
        let (_, rot, _) = w.to_scale_rotation_translation();
        let local_rot = if self.coord == CoordSystem::World { Quat::IDENTITY } else { rot };
        let (center, r) = match (sub, o.active_modifier()?) {
            (ModSub::ControlPoints, ModifierKind::Ffd(f)) => {
                let sel: Vec<Vec3> = (0..f.points.len()).filter(|&i| f.selected[i]).map(|i| f.point_pos(i)).collect();
                if sel.is_empty() {
                    return None;
                }
                (sel.iter().sum::<Vec3>() / sel.len() as f32, local_rot)
            }
            (ModSub::Lattice, ModifierKind::Ffd(f)) => (f.lattice.transform_point3(f.bounds.center()), local_rot),
            (ModSub::Gizmo, ModifierKind::UvwMap(u)) => (u.center, rot * u.rotation),
            (ModSub::Normal, ModifierKind::EditNormals(_)) => (self.normals_center()?, local_rot),
            (ModSub::Axis, ModifierKind::Lathe(l)) => {
                let shape = o.shape_input_of(o.active_level?)?;
                (l.axis_point(&shape), local_rot)
            }
            (ModSub::Gizmo, k) if k.deformer_gizmo().is_some() => {
                let (g, c) = k.deformer_gizmo()?;
                (g.transform_point3(c), rot * Quat::from_mat4(&g).normalize())
            }
            (ModSub::Center, k) if k.deformer_gizmo().is_some() => {
                let (g, c) = k.deformer_gizmo()?;
                (g.transform_point3(c), local_rot)
            }
            (ModSub::Mirror, ModifierKind::Symmetry(s)) => {
                let (_, r, _) = s.mirror.to_scale_rotation_translation();
                (s.point(), rot * r)
            }
            _ => return None,
        };
        Some(Xform { translation: w.transform_point3(center), rotation: r, scale: Vec3::ONE })
    }

    /// Re-applies a world-space drag delta to the modifier sub-object of `before`.
    pub(crate) fn drag_mod_sub(&mut self, object: ObjId, before: &SceneObject, sub: ModSub, total: Mat4) {
        let w = before.world_matrix();
        let local = w.inverse() * total * w;
        let Some(o) = self.scene.objects.get_mut(object) else { return };
        *o = before.clone();
        match (sub, o.active_modifier_mut()) {
            (ModSub::ControlPoints, Some(ModifierKind::Ffd(f))) => {
                let sel: Vec<usize> = (0..f.points.len()).filter(|&i| f.selected[i]).collect();
                for i in sel {
                    let p = f.point_pos(i);
                    f.set_point_pos(i, local.transform_point3(p));
                }
            }
            (ModSub::Lattice, Some(ModifierKind::Ffd(f))) => {
                // keep the lattice frame rigid + scaled about its own center
                f.lattice = local * f.lattice;
            }
            (ModSub::Gizmo, Some(ModifierKind::UvwMap(u))) => {
                let m = local * u.gizmo_matrix();
                u.set_gizmo_matrix(m);
            }
            (ModSub::Axis, Some(ModifierKind::Lathe(l))) => {
                l.axis_offset += local.to_scale_rotation_translation().2;
            }
            (ModSub::Normal, Some(ModifierKind::EditNormals(e))) => {
                super::normals::rotate_normals(e, &before.level_mesh, local);
            }
            (ModSub::Mirror, Some(ModifierKind::Symmetry(s))) => {
                // rigid: the mirror plane has no scale
                let (_, r, t) = (local * s.mirror).to_scale_rotation_translation();
                s.mirror = Mat4::from_rotation_translation(r, t);
            }
            (sub, Some(k)) => match (sub, k.deformer_gizmo_mut()) {
                (ModSub::Gizmo, Some((g, _))) => *g = local * *g,
                (ModSub::Center, Some((g, c))) => *c = g.inverse().transform_point3(local.transform_point3(g.transform_point3(*c))),
                _ => return,
            },
            _ => return,
        }
        o.evaluate();
        self.scene.propagate_from(object);
    }

    fn ffd_screen_points(&self, view: usize) -> Vec<(usize, Vec2)> {
        let v = &self.viewports.views[view];
        let Some((pts, _)) = self.ffd_points_world() else { return vec![] };
        pts.iter().enumerate().filter_map(|(i, p)| v.to_screen(*p).map(|s| (i, s))).collect()
    }

    fn set_ffd_selection(&mut self, hits: &[usize], op: SelectOp) {
        let Some(id) = self.current() else { return };
        let o = &mut self.scene.objects[id];
        if let Some(ModifierKind::Ffd(f)) = o.active_modifier_mut() {
            if op == SelectOp::Replace {
                f.selected.iter_mut().for_each(|s| *s = false);
            }
            for &i in hits {
                f.selected[i] = op != SelectOp::Subtract;
            }
        }
        o.selection_version = crate::scene::next_version();
    }

    /// Click in a viewport while a modifier sub-object is active. Returns true when handled.
    pub fn mod_click_select(&mut self, view: usize, cursor: Vec2, op: SelectOp) -> bool {
        match self.active_mod_sub() {
            Some(ModSub::ControlPoints) => {
                let hit = self
                    .ffd_screen_points(view)
                    .into_iter()
                    .map(|(i, s)| (i, s.distance(cursor)))
                    .filter(|x| x.1 <= VERTEX_PICK_PX)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                let hits: Vec<usize> = hit.map(|h| h.0).into_iter().collect();
                self.set_ffd_selection(&hits, op);
                true
            }
            Some(ModSub::Normal) => {
                self.normals_click(view, cursor, op);
                true
            }
            Some(_) => true,
            None => false,
        }
    }

    /// Marquee while a modifier sub-object is active. Returns true when handled.
    pub fn mod_marquee_select(&mut self, view: usize, a: Vec2, b: Vec2, op: SelectOp) -> bool {
        match self.active_mod_sub() {
            Some(ModSub::ControlPoints) => {
                let (min, max) = (a.min(b), a.max(b));
                let hits: Vec<usize> =
                    self.ffd_screen_points(view).into_iter().filter(|(_, s)| in_rect(*s, min, max)).map(|x| x.0).collect();
                self.set_ffd_selection(&hits, op);
                true
            }
            Some(ModSub::Normal) => {
                self.normals_marquee(view, a, b, op);
                true
            }
            Some(_) => true,
            None => false,
        }
    }

    /// UVW Map Normal Align: click a face to align the gizmo to its normal.
    pub fn normal_align_click(&mut self, view: usize, cursor: Vec2) -> bool {
        let Some(id) = self.current() else { return false };
        let ray = self.viewports.views[view].ray(cursor);
        let o = &self.scene.objects[id];
        let w = o.world_matrix();
        let Some(hit) = pick_polygon(&o.evaluated_mesh, w, &ray, false) else {
            self.status = "Normal Align: click on the object's surface".into();
            return true;
        };
        let inv = w.inverse();
        let point = inv.transform_point3(hit.point);
        let normal = o.evaluated_mesh.poly_normal(hit.poly);
        self.edit_active_modifier("Normal Align", |k, _| {
            if let ModifierKind::UvwMap(u) = k {
                u.normal_align(point, normal);
            }
        });
        self.tool = Tool::None;
        self.status = "Gizmo aligned to the face normal".into();
        true
    }

    /// UVW Map View Align: faces the gizmo toward the active viewport.
    pub fn view_align(&mut self) {
        let Some(o) = self.current_obj() else { return };
        let inv = o.world_matrix().inverse();
        let cam = self.viewports.active_view().camera;
        let (right, up) = (inv.transform_vector3(cam.right()), inv.transform_vector3(cam.up()));
        self.edit_active_modifier("View Align", |k, input| {
            if let ModifierKind::UvwMap(u) = k {
                u.view_align(right, up, input);
            }
        });
    }
}
