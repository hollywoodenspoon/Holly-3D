//! 3ds Max-style Move / Rotate / Scale gizmo (transform-gizmo-egui) wired to
//! object transforms, sub-object selections, pivots and the Slice Plane.

use crate::editor::{CoordSystem, Editor, PivotMode, TransformMode};
use crate::scene::{ObjId, Xform};
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use bevy_egui::egui;
use transform_gizmo_egui::math::Transform as GTransform;
use transform_gizmo_egui::{
    Gizmo, GizmoConfig, GizmoInteraction, GizmoMode, GizmoOrientation, GizmoVisuals, config::TransformPivotPoint, mint,
};

fn to_rows(m: Mat4) -> mint::RowMatrix4<f64> {
    let c = m.as_dmat4().to_cols_array_2d();
    let r = |i: usize| mint::Vector4 { x: c[0][i], y: c[1][i], z: c[2][i], w: c[3][i] };
    mint::RowMatrix4 { x: r(0), y: r(1), z: r(2), w: r(3) }
}

fn mv(v: Vec3) -> mint::Vector3<f64> {
    mint::Vector3 { x: v.x as f64, y: v.y as f64, z: v.z as f64 }
}

fn to_g(x: &Xform) -> GTransform {
    let q = x.rotation;
    let rot = mint::Quaternion { v: mv(Vec3::new(q.x, q.y, q.z)), s: q.w as f64 };
    GTransform::from_scale_rotation_translation(mv(x.scale), rot, mv(x.translation))
}

fn from_g(t: &GTransform) -> Xform {
    let v = |m: mint::Vector3<f64>| Vec3::new(m.x as f32, m.y as f32, m.z as f32);
    let r = t.rotation;
    let rotation = Quat::from_xyzw(r.v.x as f32, r.v.y as f32, r.v.z as f32, r.s as f32).normalize();
    Xform { scale: v(t.scale), rotation, translation: v(t.translation) }
}

/// Targets for the gizmo: object list (object mode) or a single frame.
enum Targets {
    Objects(Vec<ObjId>, Vec<Xform>),
    Frame(Xform),
}

fn targets(ed: &Editor) -> Option<Targets> {
    if ed.active_mod_sub().is_some() {
        return ed.mod_sub_frame().map(Targets::Frame);
    }
    if ed.spline_level().is_some() {
        let center = ed.selection_center()?;
        let rot = match ed.coord {
            CoordSystem::Local => ed.current_obj().map(|o| o.transform.rotation).unwrap_or(Quat::IDENTITY),
            _ => Quat::IDENTITY,
        };
        return Some(Targets::Frame(Xform { translation: center, rotation: rot, scale: Vec3::ONE }));
    }
    if ed.in_sub_object() {
        if let Some(sp) = &ed.slice_plane {
            return Some(Targets::Frame(sp.xform));
        }
        let center = ed.selection_center()?;
        let rot = match ed.coord {
            CoordSystem::Local => ed.current_obj().map(|o| o.transform.rotation).unwrap_or(Quat::IDENTITY),
            _ => Quat::IDENTITY,
        };
        return Some(Targets::Frame(Xform { translation: center, rotation: rot, scale: Vec3::ONE }));
    }
    let ids = ed.scene.selected();
    if ids.is_empty() {
        return None;
    }
    if ed.affect_pivot_only {
        let o = &ed.scene.objects[ids[0]];
        return Some(Targets::Frame(Xform { translation: o.transform.translation, ..Default::default() }));
    }
    let xs = ids.iter().map(|&k| ed.scene.objects[k].transform).collect();
    Some(Targets::Objects(ids, xs))
}

/// Runs the gizmo in `view`. Returns true while the pointer is over / dragging it.
pub fn run(ui: &egui::Ui, ed: &mut Editor, gz: &mut Gizmo, view: usize, hovered: bool, dragging: &mut bool) -> bool {
    let modes = match ed.mode {
        TransformMode::Select => return false,
        TransformMode::Move => GizmoMode::all_translate(),
        TransformMode::Rotate => GizmoMode::all_rotate(),
        TransformMode::Scale => GizmoMode::all_scale(),
    };
    let modes = if ed.affect_pivot_only || ed.slice_plane.is_some() && ed.mode == TransformMode::Scale {
        GizmoMode::all_translate()
    } else {
        modes
    };
    let Some(t) = targets(ed) else { return false };
    let gtargets: Vec<GTransform> = match &t {
        Targets::Objects(_, xs) => xs.iter().map(to_g).collect(),
        Targets::Frame(x) => vec![to_g(x)],
    };
    let v = &ed.viewports.views[view];
    let r = v.rect;
    let snapping = if ed.mode == TransformMode::Rotate { ed.angle_snap } else { ed.snap && ed.snap_cfg.grid };
    gz.update_config(GizmoConfig {
        view_matrix: to_rows(v.camera.view_matrix()),
        projection_matrix: to_rows(v.camera.projection_matrix(r.aspect())),
        viewport: egui::Rect::from_min_max(egui::pos2(r.min.x, r.min.y), egui::pos2(r.max.x, r.max.y)),
        modes,
        orientation: if ed.coord == CoordSystem::Local { GizmoOrientation::Local } else { GizmoOrientation::Global },
        pivot_point: if ed.pivot_mode == PivotMode::PivotPoint {
            TransformPivotPoint::IndividualOrigins
        } else {
            TransformPivotPoint::MedianPoint
        },
        snapping,
        snap_angle: 5f32.to_radians(),
        snap_distance: ed.snap_cfg.grid_size,
        snap_scale: 0.1,
        pixels_per_point: ui.ctx().pixels_per_point(),
        visuals: GizmoVisuals { gizmo_size: 80.0, stroke_width: 3.0, ..Default::default() },
        ..Default::default()
    });
    let (cursor, pressed, down, shift) =
        ui.input(|i| (i.pointer.hover_pos().unwrap_or_default(), i.pointer.primary_pressed(), i.pointer.primary_down(), i.modifiers.shift));
    let result = gz.update(
        GizmoInteraction { cursor_pos: (cursor.x, cursor.y), hovered, drag_started: pressed && hovered, dragging: down },
        &gtargets,
    );
    let draw = gz.draw();
    let mesh = egui::Mesh {
        indices: draw.indices,
        vertices: draw
            .vertices
            .into_iter()
            .zip(draw.colors)
            .map(|(pos, [r, g, b, a])| egui::epaint::Vertex {
                pos: pos.into(),
                uv: egui::Pos2::ZERO,
                color: egui::Rgba::from_rgba_premultiplied(r, g, b, a).into(),
            })
            .collect(),
        ..Default::default()
    };
    ui.painter().with_clip_rect(egui::Rect::from_min_max(egui::pos2(r.min.x, r.min.y), egui::pos2(r.max.x, r.max.y))).add(mesh);
    if let Some((_, new)) = result {
        if !*dragging {
            *dragging = true;
            let start = match &t {
                Targets::Frame(x) => x.matrix(),
                Targets::Objects(_, xs) => xs[0].matrix(),
            };
            ed.begin_drag(start, shift);
        }
        // 3D Snaps: Move puts the gizmo origin on the vertex / edge / … under the cursor.
        let hit = (ed.snap && ed.mode == TransformMode::Move).then(|| ed.snap_at(view, Vec2::new(cursor.x, cursor.y))).flatten();
        match &t {
            Targets::Objects(ids, xs) => {
                let mut pairs: Vec<(ObjId, Xform)> = ids.iter().copied().zip(new.iter().map(from_g)).collect();
                if let Some(h) = hit {
                    let d = h.pos - xs[0].translation;
                    pairs = ids.iter().copied().zip(xs.iter().map(|x| Xform { translation: x.translation + d, ..*x })).collect();
                }
                ed.drag_objects(&pairs);
            }
            Targets::Frame(x) => {
                let frame = match hit {
                    Some(h) => Mat4::from_translation(h.pos - x.translation) * x.matrix(),
                    None => new.first().map(|n| from_g(n).matrix()).unwrap_or(x.matrix()),
                };
                ed.drag_frame(frame);
            }
        }
    }
    if *dragging && !down {
        *dragging = false;
        ed.end_drag();
    }
    *dragging || gz.is_focused()
}
