//! Rendering: Render Setup (F10) state, Quick Render (F9), the Rendered
//! Frame Window's buffer, region rendering, saving and cloned windows.

use super::Editor;
use crate::render::job::{Region, clamp_region};
use crate::render::scene_build::RenderScene;
use crate::render::{ChannelView, Engine, FrameBuffer, RenderJob};
use crate::viewport::camera::ViewCamera;

/// A cloned Rendered Frame Window (frozen copy for comparison).
pub struct ClonedFrame {
    pub id: u32,
    pub label: String,
    pub fb: FrameBuffer,
    pub open: bool,
}

#[derive(Default)]
pub struct RenderUi {
    pub setup_open: bool,
    pub rfw_open: bool,
    pub fb: Option<FrameBuffer>,
    pub job: Option<RenderJob>,
    /// Region / Crop toggle and the region (output pixels).
    pub region_mode: bool,
    pub region: Option<Region>,
    pub channels: ChannelView,
    pub save_path: String,
    pub clones: Vec<ClonedFrame>,
    pub next_clone: u32,
    /// Last render / save message shown in the RFW.
    pub message: String,
}

impl Editor {
    /// Index of the viewport rendered (Render Setup's View, else the active one).
    pub fn render_view(&self) -> usize {
        self.render.view.filter(|v| *v < self.viewports.views.len()).unwrap_or(self.viewports.active)
    }

    /// The chosen camera object, else the rendered viewport's view.
    pub fn render_camera(&self) -> ViewCamera {
        match self.render.camera.and_then(|id| self.scene.camera_view(id)) {
            Some(c) => c,
            None => self.viewports.views[self.render_view()].camera,
        }
    }

    /// Name of what F9 renders ("Camera001", "Perspective"…).
    pub fn render_label(&self) -> String {
        match self.render.camera.and_then(|id| self.scene.objects.get(id)).filter(|o| o.camera().is_some()) {
            Some(o) => o.name.clone(),
            None => self.view_label(self.render_view()),
        }
    }

    pub fn is_rendering(&self) -> bool {
        self.rfw.job.is_some()
    }

    /// F9 / Render button: opens the RFW and starts rendering.
    pub fn start_render(&mut self) {
        self.cancel_render();
        let (w, h) = (self.render.width, self.render.height);
        let rs = RenderScene::build(&self.scene, self.render_camera(), self.uses_default_lights());
        let fb_ok = self.rfw.fb.as_ref().is_some_and(|f| f.width == w && f.height == h);
        let region = if self.rfw.region_mode && fb_ok { clamp_region(self.rfw.region, w, h) } else { [0, 0, w, h] };
        if !fb_ok {
            self.rfw.fb = Some(FrameBuffer::new(w, h));
        }
        let cfg = self.render.clone();
        self.rfw.job = Some(match cfg.engine {
            Engine::Raytracer => RenderJob::raytrace(rs, cfg, region),
            Engine::Cycles => RenderJob::cycles(rs, cfg),
        });
        self.rfw.rfw_open = true;
        self.rfw.message = format!("Rendering {} ({w}x{h})…", self.render_label());
        self.status = self.rfw.message.clone();
    }

    pub fn cancel_render(&mut self) {
        if let Some(j) = self.rfw.job.take() {
            j.cancel();
            self.rfw.message = "Render cancelled".into();
        }
    }

    /// Pulls finished rows into the frame buffer (call once per UI frame).
    pub fn poll_render(&mut self) {
        let (Some(job), Some(fb)) = (self.rfw.job.as_mut(), self.rfw.fb.as_mut()) else { return };
        if let Some(r) = job.poll(fb) {
            self.rfw.message = r.unwrap_or_else(|e| e);
            self.status = self.rfw.message.clone();
            self.rfw.job = None;
        }
    }

    /// Saves the RFW image (.png / .jpg / .exr by extension).
    pub fn save_render(&mut self, path: &str) {
        let Some(fb) = &self.rfw.fb else {
            self.rfw.message = "Nothing rendered yet".into();
            return;
        };
        self.rfw.message = match fb.save(path) {
            Ok(()) => format!("Saved {path}"),
            Err(e) => format!("Save failed: {e}"),
        };
        self.status = self.rfw.message.clone();
    }

    /// Clone Render Window: freezes the current image in its own window.
    pub fn clone_render_window(&mut self) {
        let Some(fb) = self.rfw.fb.clone() else { return };
        self.rfw.next_clone += 1;
        let id = self.rfw.next_clone;
        self.rfw.clones.push(ClonedFrame { id, label: format!("Clone {id}"), fb, open: true });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};
    use crate::scene::BaseObject;
    use bevy::math::Vec3;

    #[test]
    fn quick_render_fills_the_frame_buffer() {
        let mut ed = Editor::default();
        let id = ed.scene.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Sphere)), Vec3::ZERO);
        ed.scene.objects[id].evaluate();
        ed.render.width = 40;
        ed.render.height = 30;
        ed.render.aa = 1;
        ed.start_render();
        assert!(ed.rfw.rfw_open && ed.is_rendering());
        let t0 = std::time::Instant::now();
        while ed.is_rendering() && t0.elapsed().as_secs() < 30 {
            ed.poll_render();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(ed.rfw.message.starts_with("Rendered 40x30"), "{}", ed.rfw.message);
        let fb = ed.rfw.fb.as_ref().unwrap();
        let c = fb.display[(15 * 40 + 20) as usize];
        assert!(c.w > 0.99 && c.truncate().max_element() > 0.1, "sphere in the centre: {c:?}");
        ed.clone_render_window();
        assert_eq!(ed.rfw.clones.len(), 1);
        // Region render keeps the pixels outside the region.
        ed.rfw.region_mode = true;
        ed.rfw.region = Some([0, 0, 10, 10]);
        ed.scene.environment.background = [1.0, 0.0, 0.0];
        ed.start_render();
        while ed.is_rendering() {
            ed.poll_render();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let fb = ed.rfw.fb.as_ref().unwrap();
        assert!(fb.display[0].x > 0.99 && fb.display[0].y < 0.01, "region re-rendered with the red background");
        assert!(fb.display[(29 * 40 + 39) as usize].y > 0.05, "outside the region untouched");
    }
}
