//! Applying Preferences to the editor (undo steps, render defaults) and the
//! General -> Autosave timer.

use super::Editor;
use crate::io::scene_file;
use crate::prefs::Preferences;
use std::path::{Path, PathBuf};

impl Editor {
    /// Startup: takes the saved Preferences and copies their defaults in.
    pub fn use_prefs(&mut self, prefs: Preferences) {
        self.prefs = prefs;
        self.prefs.render.apply(&mut self.render);
        self.apply_prefs();
        crate::assets::photo::rescan(&self.prefs.assets.folder);
    }

    /// Settings that apply at once whenever Preferences change.
    pub fn apply_prefs(&mut self) {
        self.history.set_limit(self.prefs.general.undo_steps.max(1) as usize);
    }

    /// Where autosaves go: `~/.local/share/ilex/autosave/autosave.ilex`.
    pub fn autosave_path() -> PathBuf {
        crate::prefs::data_dir().join("autosave").join(format!("autosave.{}", scene_file::EXT))
    }

    /// True when autosave is on, `minutes` have passed and something changed.
    pub fn autosave_due(&self, now_s: f64) -> bool {
        let m = self.prefs.general.autosave_minutes;
        m > 0 && self.history.pushes() != self.iface.autosave_edits && now_s - self.iface.autosave_at >= m as f64 * 60.0
    }

    /// Saves a copy of the scene to `path` without changing the scene's own
    /// file name (Save still writes where it did).
    pub fn autosave_to(&mut self, path: &Path, now_s: f64) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        self.iface.autosave_edits = self.history.pushes();
        self.iface.autosave_at = now_s;
        scene_file::save(&self.scene, path)?;
        self.status = format!("Autosaved to {}", path.display());
        Ok(())
    }

    /// Called every frame with the app clock.
    pub fn autosave_tick(&mut self, now_s: f64) {
        if self.autosave_due(now_s) {
            let r = self.autosave_to(&Self::autosave_path(), now_s);
            if let Err(e) = r {
                self.status = format!("Autosave failed: {e}");
                self.iface.autosave_at = now_s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::PrimitiveKind;
    use bevy::math::Vec3;

    #[test]
    fn undo_steps_preference_limits_history() {
        let mut ed = Editor::default();
        for i in 0..10 {
            ed.create_primitive(PrimitiveKind::Box, Vec3::splat(i as f32));
        }
        ed.prefs.general.undo_steps = 3;
        ed.apply_prefs();
        assert_eq!(ed.history.limit(), 3);
        let mut undos = 0;
        while ed.history.can_undo() {
            ed.undo();
            undos += 1;
        }
        assert_eq!(undos, 3);
        assert_eq!(ed.scene.objects.len(), 7);
    }

    #[test]
    fn render_defaults_come_from_preferences() {
        let mut ed = Editor::default();
        let mut p = Preferences::default();
        p.render.width = 1920;
        p.render.height = 1080;
        p.render.device = crate::render::ComputeDevice::Cpu;
        ed.use_prefs(p);
        assert_eq!((ed.render.width, ed.render.height), (1920, 1080));
        assert_eq!(ed.render.path.device, crate::render::ComputeDevice::Cpu);
    }

    #[test]
    fn autosave_waits_for_time_and_changes() {
        let mut ed = Editor::default();
        ed.prefs.general.autosave_minutes = 1;
        assert!(!ed.autosave_due(1000.0), "nothing changed yet");
        ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        assert!(!ed.autosave_due(30.0), "a minute hasn't passed");
        assert!(ed.autosave_due(61.0));
        let dir = std::env::temp_dir().join(format!("ilex_autosave_{}", std::process::id()));
        let path = dir.join("autosave.ilex");
        ed.autosave_to(&path, 61.0).unwrap();
        assert!(path.exists());
        assert!(ed.scene_path.is_none(), "autosave leaves the scene's own file alone");
        assert!(!ed.autosave_due(200.0), "no new edits since");
        ed.prefs.general.autosave_minutes = 0;
        ed.create_primitive(PrimitiveKind::Box, Vec3::ONE);
        assert!(!ed.autosave_due(1e6), "off");
        std::fs::remove_dir_all(&dir).ok();
    }
}
