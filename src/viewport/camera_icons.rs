//! Viewport icons for cameras: body + lens (constant pixel size), target
//! line, and the view cone (frustum) when selected or Show Cone is on.

use super::layout::ViewState;
use crate::editor::Editor;
use crate::scene::SceneObject;
use crate::scene::camera::Camera;
use bevy::gizmos::config::GizmoConfigGroup;
use bevy::prelude::*;

const CAM_BLUE: Color = Color::srgb(0.35, 0.75, 0.95);
const CONE: Color = Color::srgb(0.45, 0.85, 1.0);

/// Corners of a rectangle `d` ahead of the eye for half-extents (w, h).
fn quad(eye: Vec3, rot: Quat, d: f32, w: f32, h: f32) -> [Vec3; 4] {
    let (x, y, f) = (rot * Vec3::X, rot * Vec3::Y, rot * Vec3::NEG_Z);
    let c = eye + f * d;
    [c - x * w - y * h, c + x * w - y * h, c + x * w + y * h, c - x * w + y * h]
}

fn pyramid<G: GizmoConfigGroup>(g: &mut Gizmos<G>, eye: Vec3, q: [Vec3; 4], color: Color) {
    g.linestrip([q[0], q[1], q[2], q[3], q[0]], color);
    for p in q {
        g.line(eye, p, color);
    }
}

pub fn draw<G: GizmoConfigGroup>(g: &mut Gizmos<G>, ed: &Editor, o: &SceneObject, c: &Camera, upp: impl Fn(Vec3) -> f32) {
    let (eye, rot) = (o.transform.translation, o.transform.rotation);
    let target = o.target_id().and_then(|t| ed.scene.objects.get(t));
    let color = if o.selected { Color::WHITE } else { CAM_BLUE };
    // Body: a box behind the lens, the lens a small pyramid pointing forward.
    let s = 16.0 * upp(eye);
    let back = rot * Vec3::Z;
    let body = Transform::from_translation(eye + back * s * 0.7).with_rotation(rot).with_scale(Vec3::new(s * 0.8, s * 0.6, s));
    g.cube(body, color);
    pyramid(g, eye + back * s * 0.2, quad(eye + back * s * 0.2, rot, s * 0.6, s * 0.35, s * 0.3), color);
    if let Some(t) = target {
        g.line(eye, t.transform.translation, CAM_BLUE.with_alpha(0.6));
    }
    if !(o.selected || c.show_cone || target.is_some_and(|t| t.selected)) {
        return;
    }
    let len = target.map_or(c.target_distance, |t| t.transform.translation.distance(eye)).max(0.01);
    let aspect = ed.render.width as f32 / ed.render.height.max(1) as f32;
    let w = len * (c.fov.to_radians() * 0.5).tan();
    pyramid(g, eye, quad(eye, rot, len, w, w / aspect), CONE);
    if c.clip_manually {
        for d in [c.near, c.far.min(len * 4.0)] {
            let w = d * (c.fov.to_radians() * 0.5).tan();
            let q = quad(eye, rot, d, w, w / aspect);
            g.linestrip([q[0], q[1], q[2], q[3], q[0]], Color::srgb(0.95, 0.55, 0.2));
        }
    }
}

/// Skips the camera a view is looking through (its icon would fill the view).
pub fn visible_in(v: &ViewState, id: crate::scene::ObjId) -> bool {
    v.cam_obj != Some(id)
}
