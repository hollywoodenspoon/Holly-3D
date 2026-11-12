//! Viewport icons for lights and targets (cameras: `camera_icons`) (constant pixel size per view),
//! plus hotspot / falloff cones, beam cylinders, target lines and
//! attenuation spheres for selected lights.

use super::layout::ViewState;
use super::lines::{Vp0Lines, Vp1Lines, Vp2Lines, Vp3Lines};
use crate::editor::Editor;
use crate::render::lighting::emit_dir;
use crate::render::area_lights::{Area, AreaShape};
use crate::render::{Light, LightKind};
use crate::scene::{BaseObject, SceneObject};
use bevy::gizmos::config::GizmoConfigGroup;
use bevy::prelude::*;

const ICON_PX: f32 = 16.0;
const LIGHT_YELLOW: Color = Color::srgb(0.98, 0.85, 0.3);
const OFF_GRAY: Color = Color::srgb(0.45, 0.45, 0.48);
const CONE_BLUE: Color = Color::srgb(0.45, 0.75, 1.0);

/// Unit vectors spanning the plane perpendicular to `n`.
fn basis(n: Vec3) -> (Vec3, Vec3) {
    let n = n.normalize_or(Vec3::Z);
    let a = if n.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let u = n.cross(a).normalize();
    (u, n.cross(u))
}

fn ring<G: GizmoConfigGroup>(g: &mut Gizmos<G>, c: Vec3, n: Vec3, r: f32, color: Color) {
    let (u, v) = basis(n);
    g.linestrip(
        (0..=32).map(|i| {
            let a = i as f32 / 32.0 * std::f32::consts::TAU;
            c + (u * a.cos() + v * a.sin()) * r
        }),
        color,
    );
}

fn sphere<G: GizmoConfigGroup>(g: &mut Gizmos<G>, c: Vec3, r: f32, color: Color) {
    for n in [Vec3::X, Vec3::Y, Vec3::Z] {
        ring(g, c, n, r, color);
    }
}

/// Cone of full angle `deg` from `apex` along `dir`, `len` long.
fn cone<G: GizmoConfigGroup>(g: &mut Gizmos<G>, apex: Vec3, dir: Vec3, len: f32, deg: f32, color: Color) {
    let r = len * (deg.to_radians() * 0.5).tan();
    let c = apex + dir * len;
    ring(g, c, dir, r, color);
    let (u, v) = basis(dir);
    for e in [u, v, -u, -v] {
        g.line(apex, c + e * r, color);
    }
}

fn cylinder<G: GizmoConfigGroup>(g: &mut Gizmos<G>, a: Vec3, dir: Vec3, len: f32, r: f32, color: Color) {
    let b = a + dir * len;
    ring(g, a, dir, r, color);
    ring(g, b, dir, r, color);
    let (u, v) = basis(dir);
    for e in [u, v, -u, -v] {
        g.line(a + e * r, b + e * r, color);
    }
}

/// Area lights draw at their true size, with a short emission arrow.
fn area<G: GizmoConfigGroup>(g: &mut Gizmos<G>, l: &Light, p: Vec3, rot: Quat, color: Color) {
    let a = Area::new(AreaShape::of(l.kind).expect("area kind"), rot, l.area_size, false);
    match a.shape {
        AreaShape::Rect => {
            let c = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0)];
            g.linestrip(c.map(|(x, y)| p + a.u * a.hw * x + a.v * a.hl * y), color);
            g.line(p - a.u * a.hw - a.v * a.hl, p + a.u * a.hw + a.v * a.hl, color);
        }
        AreaShape::Disc => ring(g, p, a.n, a.hw, color),
        AreaShape::Sphere => sphere(g, p, a.hw, color),
        AreaShape::Tube => cylinder(g, p - a.u * a.hl, a.u, a.hl * 2.0, a.hw, color),
    }
    if a.shape != AreaShape::Sphere {
        g.arrow(p, p + a.n * (a.hw.max(a.hl) * 0.8).max(0.3), color);
    }
}

fn icon<G: GizmoConfigGroup>(g: &mut Gizmos<G>, l: &Light, p: Vec3, dir: Vec3, rot: Quat, s: f32, color: Color) {
    match l.kind {
        LightKind::Omni => {
            sphere(g, p, s * 0.45, color);
            for d in [Vec3::X, Vec3::Y, Vec3::Z] {
                g.line(p - d * s * 0.8, p + d * s * 0.8, color);
            }
        }
        LightKind::TargetSpot | LightKind::FreeSpot => {
            sphere(g, p, s * 0.25, color);
            cone(g, p, dir, s * 1.4, 60.0, color);
        }
        LightKind::TargetDirect | LightKind::FreeDirect => {
            cylinder(g, p - dir * s * 0.3, dir, s * 0.6, s * 0.45, color);
            g.arrow(p, p + dir * s * 1.6, color);
        }
        LightKind::RectArea | LightKind::DiscArea | LightKind::SphereArea | LightKind::TubeArea => area(g, l, p, rot, color),
        LightKind::Skylight => {
            // Dome: horizon ring and two half arcs.
            ring(g, p, Vec3::Z, s * 0.8, color);
            for (u, _) in [(Vec3::X, 0), (Vec3::Y, 1)] {
                g.linestrip(
                    (0..=16).map(|i| {
                        let a = i as f32 / 16.0 * std::f32::consts::PI;
                        p + (u * a.cos() + Vec3::Z * a.sin()) * s * 0.8
                    }),
                    color,
                );
            }
        }
    }
}

fn draw_light<G: GizmoConfigGroup>(g: &mut Gizmos<G>, ed: &Editor, o: &SceneObject, l: &Light, upp: impl Fn(Vec3) -> f32) {
    let p = o.transform.translation;
    let dir = emit_dir(o.transform.rotation);
    let base = if l.on { LIGHT_YELLOW } else { OFF_GRAY };
    let color = if o.selected { Color::WHITE } else { base };
    icon(g, l, p, dir, o.transform.rotation, ICON_PX * upp(p), color);
    let target = l.target.and_then(|t| ed.scene.objects.get(t)).map(|t| t.transform.translation);
    if let Some(t) = target {
        g.line(p, t, base.with_alpha(0.6));
    }
    let target_sel = l.target.and_then(|t| ed.scene.objects.get(t)).is_some_and(|t| t.selected);
    if !(o.selected || target_sel) {
        return;
    }
    let len = target.map_or(5.0, |t| t.distance(p).max(0.1));
    if l.kind.is_spot() {
        cone(g, p, dir, len, l.hotspot, CONE_BLUE);
        cone(g, p, dir, len, l.falloff, CONE_BLUE.with_alpha(0.5));
    }
    if l.kind.is_directional() {
        cylinder(g, p, dir, len, l.beam_radius, CONE_BLUE);
    }
    // Near Start / End in blues, Far Start / End in yellow / orange.
    let a = l.atten();
    if l.show_atten {
        for (r, c) in [
            (a.near.map(|n| n.0), Color::srgba(0.25, 0.45, 1.0, 0.55)),
            (a.near.map(|n| n.1), Color::srgba(0.4, 0.9, 1.0, 0.6)),
            (a.far.map(|f| f.0), Color::srgba(1.0, 0.85, 0.3, 0.55)),
            (a.far.map(|f| f.1), Color::srgba(0.95, 0.55, 0.2, 0.6)),
        ] {
            if let Some(r) = r.filter(|r| *r > 1e-3) {
                sphere(g, p, r, c);
            }
        }
    }
}

fn draw_view<G: GizmoConfigGroup>(g: &mut Gizmos<G>, ed: &Editor, v: &ViewState) {
    if !v.visible {
        return;
    }
    let h = v.rect.size().y.max(1.0);
    let upp = |p: Vec3| v.camera.units_per_pixel(p, h);
    for (id, o) in ed.scene.objects.iter().filter(|(_, o)| !o.hidden) {
        match &o.base {
            BaseObject::Camera(c) if super::camera_icons::visible_in(v, id) => super::camera_icons::draw(g, ed, o, c, upp),
            BaseObject::Light(l) => draw_light(g, ed, o, l, upp),
            BaseObject::Target => {
                let p = o.transform.translation;
                let s = ICON_PX * 0.3 * upp(p);
                let owner_cam = ed.scene.objects.values().any(|c| c.camera().is_some() && c.target_id() == Some(id));
                let base = if owner_cam { Color::srgb(0.35, 0.75, 0.95) } else { LIGHT_YELLOW };
                let c = if o.selected { Color::WHITE } else { base };
                g.cube(Transform::from_translation(p).with_scale(Vec3::splat(s * 2.0)), c);
            }
            _ => {}
        }
    }
}

pub fn draw_light_icons(
    ed: Res<Editor>,
    mut g0: Gizmos<Vp0Lines>,
    mut g1: Gizmos<Vp1Lines>,
    mut g2: Gizmos<Vp2Lines>,
    mut g3: Gizmos<Vp3Lines>,
) {
    let v = &ed.viewports.views;
    draw_view(&mut g0, &ed, &v[0]);
    draw_view(&mut g1, &ed, &v[1]);
    draw_view(&mut g2, &ed, &v[2]);
    draw_view(&mut g3, &ed, &v[3]);
}
