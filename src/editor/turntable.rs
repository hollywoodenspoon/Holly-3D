//! Editor side of the Turntable / Sprite-Sheet renderer: the dialog state
//! and a background job that renders every frame with the F10 settings.

use super::Editor;
use crate::render::scene_build::RenderScene;
use crate::render::turntable::{SheetMode, SheetSettings, render_sheet, views};
use bevy::math::Vec3;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

pub struct SheetJob {
    pub cancel: Arc<AtomicBool>,
    pub done: Arc<AtomicU32>,
    pub total: u32,
    rx: Mutex<Receiver<Result<String, String>>>,
}

#[derive(Default)]
pub struct SheetUi {
    pub open: bool,
    pub settings: SheetSettings,
    pub job: Option<SheetJob>,
    pub message: String,
}

impl SheetJob {
    pub fn progress(&self) -> f32 {
        self.done.load(Ordering::Relaxed) as f32 / self.total.max(1) as f32
    }
}

impl Editor {
    /// Bounds the sheet frames: the selection, else every visible object.
    fn sheet_bounds(&self) -> (Vec3, Vec3) {
        let sel = self.sheet_selection_only().then(|| self.selection_bounds()).flatten();
        let b = sel.or_else(|| {
            let pts: Vec<Vec3> = self
                .scene
                .objects
                .values()
                .filter(|o| !o.hidden && !o.is_helper())
                .filter_map(|o| o.bounds().map(|b| (o.world_matrix(), b)))
                .flat_map(|(w, b)| [w.transform_point3(b.min), w.transform_point3(b.max)])
                .collect();
            crate::mesh::Aabb::from_points(pts)
        });
        b.map_or((Vec3::splat(-1.0), Vec3::ONE), |b| (b.min, b.max))
    }

    fn sheet_selection_only(&self) -> bool {
        self.rfw.sheet.settings.selection_only && !self.scene.selected().is_empty()
    }

    /// `[ Render Turntable / Sprite Sheet ]`: renders every frame in the
    /// background and writes the files.
    pub fn start_sheet(&mut self) {
        if self.rfw.sheet.job.is_some() {
            return;
        }
        let s = self.rfw.sheet.settings.clone();
        let (min, max) = self.sheet_bounds();
        let v = views(&s, min, max, &self.render_camera());
        let rs = RenderScene::build(&self.scene, self.render_camera(), self.uses_default_lights());
        let cfg = self.render.clone();
        let (cancel, done) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicU32::new(0)));
        let (tx, rx) = channel();
        let total = v.len() as u32;
        let (c, d) = (cancel.clone(), done.clone());
        std::thread::spawn(move || {
            let _ = tx.send(render_sheet(rs, &cfg, &s, v, &c, &d));
        });
        self.rfw.sheet.job = Some(SheetJob { cancel, done, total, rx: Mutex::new(rx) });
        let what = if self.rfw.sheet.settings.mode == SheetMode::Turntable { "turntable frames" } else { "sprite directions" };
        self.rfw.sheet.message = format!("Rendering {total} {what}…");
        self.status = self.rfw.sheet.message.clone();
    }

    pub fn cancel_sheet(&mut self) {
        if let Some(j) = &self.rfw.sheet.job {
            j.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Picks up the finished sheet job's result.
    pub fn poll_sheet(&mut self) {
        let Some(j) = &self.rfw.sheet.job else { return };
        let r = j.rx.lock().ok().and_then(|rx| rx.try_recv().ok());
        if let Some(r) = r {
            self.rfw.sheet.message = r.unwrap_or_else(|e| e);
            self.status = self.rfw.sheet.message.clone();
            self.rfw.sheet.job = None;
        }
    }

    pub fn toggle_shadow_catcher(&mut self) {
        let ids = self.scene.selected();
        self.set_flag("Shadow & Reflection Catcher", ids, |o| o.shadow_catcher = !o.shadow_catcher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};
    use crate::scene::BaseObject;

    #[test]
    fn sheet_job_renders_and_reports() {
        let mut ed = Editor::default();
        let id = ed.scene.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::ZERO);
        ed.scene.objects[id].evaluate();
        ed.toggle_shadow_catcher();
        let dir = std::env::temp_dir().join(format!("mmx_ed_sheet_{}", std::process::id()));
        (ed.render.width, ed.render.height, ed.render.aa) = (8, 8, 1);
        ed.render.path.integrator = crate::render::pathtrace::settings::Integrator::Classic;
        let s = &mut ed.rfw.sheet.settings;
        (s.frames, s.folder, s.name) = (8, dir.to_string_lossy().into(), "spin".into());
        ed.start_sheet();
        assert_eq!(ed.rfw.sheet.job.as_ref().map(|j| j.total), Some(8));
        for _ in 0..2000 {
            ed.poll_sheet();
            if ed.rfw.sheet.job.is_none() {
                break;
            }
            std::thread::yield_now();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(ed.rfw.sheet.message.starts_with("Saved 8 turntable frames"), "{}", ed.rfw.sheet.message);
        assert!(dir.join("spin_0007.png").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
