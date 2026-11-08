//! 2x2 four-split / maximized viewport layout state.

use super::camera::{Ray, ViewCamera, ViewKind};
use bevy::math::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shading {
    Default,
    Wireframe,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ScreenRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl ScreenRect {
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }
    pub fn aspect(&self) -> f32 {
        let s = self.size();
        if s.y <= 0.0 { 1.0 } else { s.x / s.y }
    }
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.y >= self.min.y && p.x < self.max.x && p.y < self.max.y
    }
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

#[derive(Clone, Debug)]
pub struct ViewState {
    pub camera: ViewCamera,
    pub shading: Shading,
    pub edged_faces: bool,
    pub grid: bool,
    pub rect: ScreenRect,
    pub visible: bool,
}

impl ViewState {
    pub fn new(kind: ViewKind) -> Self {
        let shading = if kind == ViewKind::Perspective { Shading::Default } else { Shading::Wireframe };
        Self { camera: ViewCamera::new(kind), shading, edged_faces: false, grid: true, rect: ScreenRect::default(), visible: true }
    }

    pub fn shading_label(&self) -> String {
        let base = match self.shading {
            Shading::Default => "Default Shading",
            Shading::Wireframe => "Wireframe",
        };
        if self.edged_faces && self.shading == Shading::Default { format!("{base} + Edged Faces") } else { base.to_string() }
    }

    pub fn to_ndc(&self, p: Vec2) -> Vec2 {
        let s = self.rect.size().max(Vec2::ONE);
        let u = (p - self.rect.min) / s;
        Vec2::new(u.x * 2.0 - 1.0, 1.0 - u.y * 2.0)
    }

    pub fn from_ndc(&self, n: Vec2) -> Vec2 {
        let s = self.rect.size();
        self.rect.min + Vec2::new((n.x + 1.0) * 0.5 * s.x, (1.0 - n.y) * 0.5 * s.y)
    }

    pub fn ray(&self, screen: Vec2) -> Ray {
        self.camera.ray(self.to_ndc(screen), self.rect.aspect())
    }

    pub fn to_screen(&self, world: Vec3) -> Option<Vec2> {
        let ndc = self.camera.project(world, self.rect.aspect())?;
        Some(self.from_ndc(ndc))
    }

    /// Point on the construction plane facing the view (Home Grid for perspective/top).
    pub fn construction_point(&self, screen: Vec2) -> Vec3 {
        let ray = self.ray(screen);
        let n = match self.camera.kind {
            ViewKind::Front => Vec3::Y,
            ViewKind::Left => Vec3::X,
            _ => Vec3::Z,
        };
        let t = ray.plane(Vec3::ZERO, n).filter(|t| *t > 0.0).unwrap_or(self.camera.distance);
        ray.at(t)
    }
}

#[derive(Clone, Debug)]
pub struct Viewports {
    pub views: [ViewState; 4],
    pub active: usize,
    pub maximized: bool,
}

impl Default for Viewports {
    fn default() -> Self {
        Self {
            views: [
                ViewState::new(ViewKind::Top),
                ViewState::new(ViewKind::Front),
                ViewState::new(ViewKind::Left),
                ViewState::new(ViewKind::Perspective),
            ],
            active: 3,
            maximized: false,
        }
    }
}

impl Viewports {
    /// Splits the central area into quadrants (or one maximized view).
    pub fn layout(&mut self, area: ScreenRect, gap: f32) {
        if self.maximized {
            for (i, v) in self.views.iter_mut().enumerate() {
                v.visible = i == self.active;
                v.rect = area;
            }
            return;
        }
        let mid = area.center();
        let h = gap * 0.5;
        let rects = [
            ScreenRect { min: area.min, max: Vec2::new(mid.x - h, mid.y - h) },
            ScreenRect { min: Vec2::new(mid.x + h, area.min.y), max: Vec2::new(area.max.x, mid.y - h) },
            ScreenRect { min: Vec2::new(area.min.x, mid.y + h), max: Vec2::new(mid.x - h, area.max.y) },
            ScreenRect { min: Vec2::new(mid.x + h, mid.y + h), max: area.max },
        ];
        for (v, r) in self.views.iter_mut().zip(rects) {
            v.rect = r;
            v.visible = true;
        }
    }

    pub fn active_view(&self) -> &ViewState {
        &self.views[self.active]
    }

    pub fn active_view_mut(&mut self) -> &mut ViewState {
        &mut self.views[self.active]
    }

    pub fn view_at(&self, p: Vec2) -> Option<usize> {
        self.views.iter().position(|v| v.visible && v.rect.contains(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_layout_and_maximize() {
        let mut vp = Viewports::default();
        let area = ScreenRect { min: Vec2::new(200.0, 36.0), max: Vec2::new(1000.0, 636.0) };
        vp.layout(area, 2.0);
        assert_eq!(vp.view_at(Vec2::new(210.0, 40.0)), Some(0));
        assert_eq!(vp.view_at(Vec2::new(990.0, 630.0)), Some(3));
        vp.maximized = true;
        vp.layout(area, 2.0);
        assert_eq!(vp.view_at(Vec2::new(210.0, 40.0)), Some(3));
    }

    #[test]
    fn screen_roundtrip() {
        let mut vp = Viewports::default();
        vp.layout(ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
        let v = &vp.views[3];
        let s = v.to_screen(Vec3::new(0.2, 0.1, 0.3)).unwrap();
        let r = v.ray(s);
        let p = Vec3::new(0.2, 0.1, 0.3);
        assert!((r.origin + r.dir * (p - r.origin).dot(r.dir) - p).length() < 1e-3);
    }
}
