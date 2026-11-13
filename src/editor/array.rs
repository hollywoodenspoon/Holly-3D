//! Tools -> Array... (Array dialog): incremental Move / Rotate /
//! Scale per copy in 1D, plus row (2D) and layer (3D) offsets, with a live
//! preview that is rebuilt whenever a setting changes.

use super::*;
use crate::scene::BaseObject;
use bevy::math::{EulerRot, Mat4, Quat};

#[derive(Clone, Debug, PartialEq)]
pub struct ArraySettings {
    /// Incremental move (per copy, world units).
    pub move_inc: Vec3,
    /// Incremental rotation (per copy, degrees).
    pub rotate_inc: Vec3,
    /// Incremental scale (per copy, percent; 100 = unchanged).
    pub scale_inc: Vec3,
    pub uniform: bool,
    pub re_orient: bool,
    pub kind: CloneKind,
    /// 1, 2 or 3.
    pub dims: u8,
    pub count: [u32; 3],
    pub row_offset: Vec3,
    pub layer_offset: Vec3,
    pub preview: bool,
}

impl Default for ArraySettings {
    fn default() -> Self {
        Self {
            move_inc: Vec3::new(0.5, 0.0, 0.0),
            rotate_inc: Vec3::ZERO,
            scale_inc: Vec3::splat(100.0),
            uniform: false,
            re_orient: true,
            kind: CloneKind::Copy,
            dims: 1,
            count: [5, 2, 2],
            row_offset: Vec3::new(0.0, 0.5, 0.0),
            layer_offset: Vec3::new(0.0, 0.0, 0.5),
            preview: true,
        }
    }
}

impl ArraySettings {
    /// Counts in effect for the chosen dimensionality.
    pub fn counts(&self) -> [u32; 3] {
        let c = self.count.map(|n| n.max(1));
        match self.dims {
            1 => [c[0], 1, 1],
            2 => [c[0], c[1], 1],
            _ => c,
        }
    }

    /// Total objects in the array, original included.
    pub fn total(&self) -> u32 {
        self.counts().iter().product()
    }

    fn scale_vec(&self) -> Vec3 {
        if self.uniform { Vec3::splat(self.scale_inc.x) } else { self.scale_inc }
    }

    /// Totals column = incremental x (count - 1); scale grows linearly.
    pub fn totals(&self) -> (Vec3, Vec3, Vec3) {
        let n = (self.counts()[0] - 1) as f32;
        (self.move_inc * n, self.rotate_inc * n, Vec3::splat(100.0) + (self.scale_vec() - 100.0) * n)
    }

    /// Sets the incrementals from edited totals.
    pub fn set_totals(&mut self, mv: Vec3, rot: Vec3, scale: Vec3) {
        let n = (self.counts()[0] - 1).max(1) as f32;
        self.move_inc = mv / n;
        self.rotate_inc = rot / n;
        self.scale_inc = Vec3::splat(100.0) + (scale - 100.0) / n;
    }

    /// New node transforms for one source (the original is skipped), in
    /// 1D-major order. Rotation and scale happen about `center`.
    pub fn transforms(&self, src: Xform, center: Vec3) -> Vec<Xform> {
        let [n1, n2, n3] = self.counts();
        let mut out = vec![];
        for k in 0..n3 {
            for j in 0..n2 {
                for i in 0..n1 {
                    if i == 0 && j == 0 && k == 0 {
                        continue;
                    }
                    out.push(self.copy_xform(src, center, i, j, k));
                }
            }
        }
        out
    }

    fn copy_xform(&self, src: Xform, center: Vec3, i: u32, j: u32, k: u32) -> Xform {
        let f = i as f32;
        let r = self.rotate_inc.to_array().map(|d| (d * f).to_radians());
        let rot = Quat::from_euler(EulerRot::XYZ, r[0], r[1], r[2]);
        let s = (Vec3::ONE + (self.scale_vec() / 100.0 - 1.0) * f).max(Vec3::splat(1e-3));
        let shift = self.move_inc * f + self.row_offset * j as f32 + self.layer_offset * k as f32;
        let about = Mat4::from_translation(center) * Mat4::from_quat(rot) * Mat4::from_scale(s) * Mat4::from_translation(-center);
        let a = Mat4::from_translation(shift) * about;
        if self.re_orient {
            Xform::from_matrix(a * src.matrix())
        } else {
            Xform { translation: a.transform_point3(src.translation), rotation: src.rotation, scale: src.scale * s }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArrayDialog {
    pub settings: ArraySettings,
    /// Settings the current preview copies were built with.
    pub applied: Option<ArraySettings>,
    pub sources: Vec<ObjId>,
    /// Instance groups of the sources before cloning (restored on Cancel).
    pub groups: Vec<Option<u32>>,
    pub copies: Vec<ObjId>,
    pub names: std::collections::HashMap<String, u32>,
}

impl Editor {
    /// Rotation / scale center for Array and Mirror (toolbar pivot mode).
    pub fn tool_center(&self, id: ObjId) -> Vec3 {
        match self.pivot_mode {
            PivotMode::PivotPoint => self.scene.objects[id].transform.translation,
            PivotMode::SelectionCenter => self.selection_center().unwrap_or(Vec3::ZERO),
            PivotMode::TransformCenter => Vec3::ZERO,
        }
    }

    pub fn open_array(&mut self) {
        let sources: Vec<ObjId> =
            self.scene.selected().into_iter().filter(|&k| !matches!(self.scene.objects[k].base, BaseObject::Target)).collect();
        if sources.is_empty() {
            self.status = "Array needs a selected object".into();
            return;
        }
        self.checkpoint("Array");
        let groups = sources.iter().map(|&k| self.scene.objects[k].instance_group).collect();
        let settings = self.array_settings.clone();
        let names = self.scene.name_state();
        self.array_dialog = Some(ArrayDialog { settings, applied: None, sources, groups, copies: vec![], names });
    }

    fn clear_array_copies(&mut self) {
        let Some(d) = &mut self.array_dialog else { return };
        for id in std::mem::take(&mut d.copies) {
            self.scene.delete(id);
        }
        d.applied = None;
        self.scene.restore_names(d.names.clone());
    }

    fn build_array(&mut self) {
        self.clear_array_copies();
        let Some(d) = self.array_dialog.clone() else { return };
        let mut copies = vec![];
        for &id in &d.sources {
            let (src, center) = (self.scene.objects[id].transform, self.tool_center(id));
            for xf in d.settings.transforms(src, center) {
                copies.extend(self.scene.clone_object(id, d.settings.kind, xf));
            }
        }
        if let Some(d) = &mut self.array_dialog {
            d.copies = copies;
            d.applied = Some(d.settings.clone());
        }
    }

    /// Called every frame while the dialog is open: keeps the preview live.
    pub fn update_array_preview(&mut self) {
        let Some(d) = &self.array_dialog else { return };
        if !d.settings.preview {
            if d.applied.is_some() {
                self.clear_array_copies();
            }
        } else if d.applied.as_ref() != Some(&d.settings) {
            self.build_array();
        }
    }

    pub fn confirm_array(&mut self) {
        let Some(d) = &self.array_dialog else { return };
        if d.applied.as_ref() != Some(&d.settings) {
            self.build_array();
        }
        let Some(d) = self.array_dialog.take() else { return };
        self.status = format!("Array: {} new object(s)", d.copies.len());
        self.array_settings = d.settings;
    }

    pub fn cancel_array(&mut self) {
        self.clear_array_copies();
        let Some(d) = self.array_dialog.take() else { return };
        for (id, g) in d.sources.iter().zip(d.groups) {
            if let Some(o) = self.scene.objects.get_mut(*id) {
                o.instance_group = g;
            }
        }
        self.history.discard_last();
        self.array_settings = d.settings;
        self.status = "Array cancelled".into();
    }
}
