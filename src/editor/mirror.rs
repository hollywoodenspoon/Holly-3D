//! Tools -> Mirror (3ds Max Mirror dialog): mirror axis X / Y / Z / XY /
//! YZ / ZX about the transform center, with an offset and No Clone / Copy /
//! Instance / Reference, previewed live.

use super::*;
use crate::scene::BaseObject;
use bevy::math::Mat4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MirrorAxis {
    #[default]
    X,
    Y,
    Z,
    XY,
    YZ,
    ZX,
}

impl MirrorAxis {
    pub const ALL: [MirrorAxis; 6] = [Self::X, Self::Y, Self::Z, Self::XY, Self::YZ, Self::ZX];
    pub fn label(self) -> &'static str {
        ["X", "Y", "Z", "XY", "YZ", "ZX"][self as usize]
    }
    /// Axes that get negated (1 = keep, -1 = flip).
    pub fn flips(self) -> Vec3 {
        match self {
            Self::X => Vec3::new(-1.0, 1.0, 1.0),
            Self::Y => Vec3::new(1.0, -1.0, 1.0),
            Self::Z => Vec3::new(1.0, 1.0, -1.0),
            Self::XY => Vec3::new(-1.0, -1.0, 1.0),
            Self::YZ => Vec3::new(1.0, -1.0, -1.0),
            Self::ZX => Vec3::new(-1.0, 1.0, -1.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MirrorSettings {
    pub axis: MirrorAxis,
    pub offset: f32,
    /// None = No Clone (mirror the selection itself).
    pub clone: Option<CloneKind>,
}

impl MirrorSettings {
    /// World matrix that mirrors about `center`, then shifts by the offset
    /// along every mirrored axis.
    pub fn matrix(&self, center: Vec3) -> Mat4 {
        let f = self.axis.flips();
        let shift = Vec3::select(f.cmplt(Vec3::ZERO), Vec3::splat(self.offset), Vec3::ZERO);
        Mat4::from_translation(center + shift) * Mat4::from_scale(f) * Mat4::from_translation(-center)
    }

    pub fn apply(&self, src: Xform, center: Vec3) -> Xform {
        Xform::from_matrix(self.matrix(center) * src.matrix())
    }
}

#[derive(Clone, Debug)]
pub struct MirrorDialog {
    pub settings: MirrorSettings,
    pub applied: Option<MirrorSettings>,
    /// Sources with their original transforms and instance groups.
    pub sources: Vec<(ObjId, Xform, Option<u32>)>,
    pub copies: Vec<ObjId>,
    pub names: std::collections::HashMap<String, u32>,
}

impl Editor {
    pub fn open_mirror(&mut self) {
        let sources: Vec<_> = self
            .scene
            .selected()
            .into_iter()
            .filter(|&k| !matches!(self.scene.objects[k].base, BaseObject::Target))
            .map(|k| (k, self.scene.objects[k].transform, self.scene.objects[k].instance_group))
            .collect();
        if sources.is_empty() {
            self.status = "Mirror needs a selected object".into();
            return;
        }
        self.checkpoint("Mirror");
        let settings = self.mirror_settings.clone();
        let names = self.scene.name_state();
        self.mirror_dialog = Some(MirrorDialog { settings, applied: None, sources, copies: vec![], names });
    }

    /// Puts the sources back and removes preview copies.
    fn reset_mirror(&mut self) {
        let Some(d) = &mut self.mirror_dialog else { return };
        let (copies, sources) = (std::mem::take(&mut d.copies), d.sources.clone());
        d.applied = None;
        self.scene.restore_names(d.names.clone());
        for id in copies {
            self.scene.delete(id);
        }
        for (id, xf, g) in sources {
            if let Some(o) = self.scene.objects.get_mut(id) {
                o.transform = xf;
                o.instance_group = g;
            }
        }
    }

    fn build_mirror(&mut self) {
        self.reset_mirror();
        let Some(d) = self.mirror_dialog.clone() else { return };
        let center = match self.pivot_mode {
            PivotMode::TransformCenter => Vec3::ZERO,
            _ => self.selection_center().unwrap_or(Vec3::ZERO),
        };
        let mut copies = vec![];
        for &(id, xf, _) in &d.sources {
            let c = if self.pivot_mode == PivotMode::PivotPoint { xf.translation } else { center };
            let m = d.settings.apply(xf, c);
            match d.settings.clone {
                None => self.scene.objects[id].transform = m,
                Some(kind) => copies.extend(self.scene.clone_object(id, kind, m)),
            }
        }
        if let Some(d) = &mut self.mirror_dialog {
            d.copies = copies;
            d.applied = Some(d.settings.clone());
        }
    }

    pub fn update_mirror_preview(&mut self) {
        let Some(d) = &self.mirror_dialog else { return };
        if d.applied.as_ref() != Some(&d.settings) {
            self.build_mirror();
        }
    }

    pub fn confirm_mirror(&mut self) {
        self.update_mirror_preview();
        let Some(d) = self.mirror_dialog.take() else { return };
        if !d.copies.is_empty() {
            self.scene.select_only(None);
            for &c in &d.copies {
                self.scene.objects[c].selected = true;
            }
        }
        self.status = format!("Mirrored {} object(s) on {}", d.sources.len(), d.settings.axis.label());
        self.mirror_settings = d.settings;
    }

    pub fn cancel_mirror(&mut self) {
        self.reset_mirror();
        if let Some(d) = self.mirror_dialog.take() {
            self.mirror_settings = d.settings;
        }
        self.history.discard_last();
        self.status = "Mirror cancelled".into();
    }
}
