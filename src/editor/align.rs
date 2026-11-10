//! Tools -> Align (Alt+A): pick a target object, then align the selection's
//! X / Y / Z position (Minimum / Center / Pivot Point / Maximum on the
//! current and target objects), local orientation axes and scale, with a
//! live preview like the 3ds Max Align Selection dialog.

use super::*;
use crate::mesh::Aabb;
use crate::scene::BaseObject;
use bevy::math::Quat;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AlignPoint {
    #[default]
    Minimum,
    Center,
    PivotPoint,
    Maximum,
}

impl AlignPoint {
    pub const ALL: [AlignPoint; 4] = [Self::Minimum, Self::Center, Self::PivotPoint, Self::Maximum];
    pub fn label(self) -> &'static str {
        ["Minimum", "Center", "Pivot Point", "Maximum"][self as usize]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlignSettings {
    /// X / Y / Z Position (world).
    pub pos: [bool; 3],
    pub current: AlignPoint,
    pub target: AlignPoint,
    /// Align Orientation (Local) X / Y / Z axes.
    pub orient: [bool; 3],
    /// Match Scale X / Y / Z.
    pub scale: [bool; 3],
}

impl Default for AlignSettings {
    fn default() -> Self {
        Self { pos: [true; 3], current: AlignPoint::Center, target: AlignPoint::Center, orient: [false; 3], scale: [false; 3] }
    }
}

#[derive(Clone, Debug)]
pub struct AlignDialog {
    pub target: ObjId,
    pub settings: AlignSettings,
    pub applied: Option<AlignSettings>,
    /// Aligned objects with their original transforms.
    pub sources: Vec<(ObjId, Xform)>,
}

/// World-space bounds of an object (helpers: their pivot).
pub fn world_bounds(o: &SceneObject) -> Aabb {
    let p = o.transform.translation;
    if o.is_helper() {
        return Aabb { min: p, max: p };
    }
    let w = o.world_matrix();
    let mut pts: Vec<Vec3> = o.evaluated_mesh.vertices.values().map(|v| w.transform_point3(v.position)).collect();
    if let Some(sh) = &o.shape {
        pts.extend(sh.polylines().into_iter().flat_map(|pl| pl.pts).map(|q| w.transform_point3(q)));
    }
    Aabb::from_points(pts).unwrap_or(Aabb { min: p, max: p })
}

fn point(o: &SceneObject, which: AlignPoint) -> Vec3 {
    let b = world_bounds(o);
    match which {
        AlignPoint::Minimum => b.min,
        AlignPoint::Center => b.center(),
        AlignPoint::PivotPoint => o.transform.translation,
        AlignPoint::Maximum => b.max,
    }
}

impl AlignSettings {
    /// `cur` aligned to `target`: orientation and scale first, then position.
    pub fn apply(&self, cur: &SceneObject, target: &SceneObject) -> Xform {
        let mut o = cur.clone();
        let (r, tr) = (o.transform.rotation, target.transform.rotation);
        let axes = [Vec3::X, Vec3::Y, Vec3::Z];
        let picked: Vec<usize> = (0..3).filter(|&a| self.orient[a]).collect();
        o.transform.rotation = match picked.as_slice() {
            [] => r,
            // One axis: turn the current local axis onto the target's.
            [a] => (Quat::from_rotation_arc(r * axes[*a], tr * axes[*a]) * r).normalize(),
            // Two axes fix the third: full orientation match.
            _ => tr,
        };
        for a in 0..3 {
            if self.scale[a] {
                o.transform.scale[a] = target.transform.scale[a];
            }
        }
        let (c, t) = (point(&o, self.current), point(target, self.target));
        for a in 0..3 {
            if self.pos[a] {
                o.transform.translation[a] += t[a] - c[a];
            }
        }
        o.transform
    }
}

impl Editor {
    /// Tools -> Align: waits for a click on the target object.
    pub fn start_align(&mut self) {
        if self.align_sources().is_empty() {
            self.status = "Align needs a selected object".into();
            return;
        }
        self.tool = Tool::PickAlign;
    }

    fn align_sources(&self) -> Vec<ObjId> {
        self.scene.selected().into_iter().filter(|&k| !matches!(self.scene.objects[k].base, BaseObject::Target)).collect()
    }

    /// Opens the Align Selection dialog for `target`.
    pub fn open_align(&mut self, target: ObjId) {
        self.tool = Tool::None;
        let sources: Vec<_> =
            self.align_sources().into_iter().filter(|&k| k != target).map(|k| (k, self.scene.objects[k].transform)).collect();
        if sources.is_empty() || !self.scene.objects.contains_key(target) {
            self.status = "Align: pick a target other than the selection".into();
            return;
        }
        self.checkpoint("Align");
        self.align_dialog = Some(AlignDialog { target, settings: self.align_settings.clone(), applied: None, sources });
    }

    fn build_align(&mut self) {
        let Some(d) = self.align_dialog.clone() else { return };
        for &(id, xf) in &d.sources {
            self.scene.objects[id].transform = xf;
        }
        let Some(target) = self.scene.objects.get(d.target).cloned() else { return };
        for &(id, _) in &d.sources {
            let xf = d.settings.apply(&self.scene.objects[id], &target);
            self.scene.objects[id].transform = xf;
        }
        if let Some(d) = &mut self.align_dialog {
            d.applied = Some(d.settings.clone());
        }
    }

    pub fn update_align_preview(&mut self) {
        if self.align_dialog.as_ref().is_some_and(|d| d.applied.as_ref() != Some(&d.settings)) {
            self.build_align();
        }
    }

    /// Apply: keeps the current alignment and starts a fresh one from it.
    pub fn apply_align(&mut self) {
        self.update_align_preview();
        let Some(d) = &mut self.align_dialog else { return };
        for (id, xf) in d.sources.iter_mut() {
            *xf = self.scene.objects[*id].transform;
        }
        d.applied = None;
        let s = d.settings.clone();
        d.settings = AlignSettings { pos: [false; 3], orient: [false; 3], scale: [false; 3], ..s };
        self.checkpoint("Align");
    }

    pub fn confirm_align(&mut self) {
        self.update_align_preview();
        let Some(d) = self.align_dialog.take() else { return };
        let name = self.scene.objects.get(d.target).map(|o| o.name.clone()).unwrap_or_default();
        self.status = format!("Aligned {} object(s) to {name}", d.sources.len());
        self.align_settings = d.settings;
    }

    pub fn cancel_align(&mut self) {
        let Some(d) = self.align_dialog.take() else { return };
        for (id, xf) in d.sources {
            if let Some(o) = self.scene.objects.get_mut(id) {
                o.transform = xf;
            }
        }
        self.align_settings = d.settings;
        self.history.discard_last();
        self.status = "Align cancelled".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::PrimitiveKind;

    fn two_boxes() -> (Editor, ObjId, ObjId) {
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Box, Vec3::new(5.0, 2.0, 0.0));
        ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        let ids: Vec<ObjId> = ed.scene.objects.keys().collect();
        ed.scene.objects[ids[1]].transform.scale = Vec3::splat(2.0);
        ed.scene.objects[ids[1]].transform.rotation = Quat::from_rotation_z(0.5);
        ed.scene.select_only(Some(ids[0]));
        (ed, ids[0], ids[1])
    }

    #[test]
    fn align_positions_min_to_max_and_center_to_center() {
        let (mut ed, cur, tgt) = two_boxes();
        ed.start_align();
        assert_eq!(ed.tool, Tool::PickAlign);
        ed.open_align(tgt);
        let d = ed.align_dialog.as_mut().unwrap();
        (d.settings.pos, d.settings.current, d.settings.target) = ([true, false, false], AlignPoint::Minimum, AlignPoint::Maximum);
        ed.update_align_preview();
        let (bc, bt) = (world_bounds(&ed.scene.objects[cur]), world_bounds(&ed.scene.objects[tgt]));
        assert!((bc.min.x - bt.max.x).abs() < 1e-4, "min X onto max X");
        assert!((ed.scene.objects[cur].transform.translation.y - 2.0).abs() < 1e-5, "Y untouched");
        // switch to center/center on all axes: preview rebuilds from the original
        let d = ed.align_dialog.as_mut().unwrap();
        (d.settings.pos, d.settings.current, d.settings.target) = ([true; 3], AlignPoint::Center, AlignPoint::Center);
        ed.confirm_align();
        let (bc, bt) = (world_bounds(&ed.scene.objects[cur]), world_bounds(&ed.scene.objects[tgt]));
        assert!((bc.center() - bt.center()).length() < 1e-4);
        assert!(ed.align_dialog.is_none());
        ed.undo();
        assert_eq!(ed.scene.objects[cur].transform.translation, Vec3::new(5.0, 2.0, 0.0));
    }

    #[test]
    fn orientation_scale_and_cancel() {
        let (mut ed, cur, tgt) = two_boxes();
        ed.open_align(tgt);
        let d = ed.align_dialog.as_mut().unwrap();
        d.settings = AlignSettings { pos: [false; 3], orient: [true, false, false], scale: [true, false, true], ..Default::default() };
        ed.update_align_preview();
        let o = &ed.scene.objects[cur];
        assert!(o.transform.rotation.angle_between(Quat::from_rotation_z(0.5)) < 1e-4, "local X turned onto the target's X");
        assert_eq!(o.transform.scale, Vec3::new(2.0, 1.0, 2.0));
        assert_eq!(o.transform.translation, Vec3::new(5.0, 2.0, 0.0));
        ed.cancel_align();
        let o = &ed.scene.objects[cur];
        assert_eq!((o.transform.rotation, o.transform.scale), (Quat::IDENTITY, Vec3::ONE));
        // picking the selection itself is refused
        ed.open_align(cur);
        assert!(ed.align_dialog.is_none());
    }
}
