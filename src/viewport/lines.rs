//! Line overlays drawn with Bevy gizmos: home grid, wireframe / edged faces
//! (per viewport via render layers), selection brackets, sub-object edges and
//! the Slice Plane.

use super::camera::ViewKind;
use super::layout::{Shading, ViewState};
use super::render_sync::{LAYER_OVERLAY, view_layer};
use crate::editor::Editor;
use crate::mesh::SubObject;
use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;

macro_rules! view_group {
    ($($name:ident),*) => {$(
        #[derive(Default, Reflect, GizmoConfigGroup)]
        pub struct $name;
    )*};
}
view_group!(Vp0Lines, Vp1Lines, Vp2Lines, Vp3Lines, OverlayLines);

pub fn register(app: &mut App) {
    app.init_gizmo_group::<Vp0Lines>()
        .init_gizmo_group::<Vp1Lines>()
        .init_gizmo_group::<Vp2Lines>()
        .init_gizmo_group::<Vp3Lines>()
        .init_gizmo_group::<OverlayLines>()
        .add_systems(Startup, configure)
        .add_systems(Update, (draw_lines, super::spline_lines::draw));
}

fn configure(mut store: ResMut<GizmoConfigStore>) {
    fn set<G: GizmoConfigGroup>(store: &mut GizmoConfigStore, layer: usize, width: f32, bias: f32) {
        let (cfg, _) = store.config_mut::<G>();
        cfg.render_layers = RenderLayers::layer(layer);
        cfg.line.width = width;
        cfg.depth_bias = bias;
    }
    set::<Vp0Lines>(&mut store, view_layer(0), 1.0, -0.0008);
    set::<Vp1Lines>(&mut store, view_layer(1), 1.0, -0.0008);
    set::<Vp2Lines>(&mut store, view_layer(2), 1.0, -0.0008);
    set::<Vp3Lines>(&mut store, view_layer(3), 1.0, -0.0008);
    set::<OverlayLines>(&mut store, LAYER_OVERLAY, 2.0, -0.0015);
    let (default, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    default.render_layers = RenderLayers::layer(LAYER_OVERLAY);
}

const GRID: Color = Color::srgb(0.32, 0.33, 0.35);
const GRID_AXIS: Color = Color::srgb(0.12, 0.12, 0.13);

fn draw_grid<G: GizmoConfigGroup>(g: &mut Gizmos<G>, v: &ViewState) {
    let cam = &v.camera;
    let (u, w) = match cam.kind {
        ViewKind::Front => (Vec3::X, Vec3::Z),
        ViewKind::Left => (Vec3::Y, Vec3::Z),
        _ => (Vec3::X, Vec3::Y),
    };
    let extent = if cam.is_ortho() { cam.ortho_height * v.rect.aspect().max(1.0) } else { cam.distance * 2.5 };
    let step = 10f32.powf((extent / 4.0).log10().floor()).max(0.01);
    let n = ((extent / step).ceil() as i32).min(200);
    let center = cam.target;
    let cu = (center.dot(u) / step).round() as i32;
    let cw = (center.dot(w) / step).round() as i32;
    for i in -n..=n {
        let a = (cu + i) as f32 * step;
        let b = (cw + i) as f32 * step;
        let c1 = if cu + i == 0 { GRID_AXIS } else { GRID };
        let c2 = if cw + i == 0 { GRID_AXIS } else { GRID };
        let (lo_w, hi_w) = ((cw - n) as f32 * step, (cw + n) as f32 * step);
        let (lo_u, hi_u) = ((cu - n) as f32 * step, (cu + n) as f32 * step);
        g.line(u * a + w * lo_w, u * a + w * hi_w, c1);
        g.line(w * b + u * lo_u, w * b + u * hi_u, c2);
    }
}

fn draw_view<G: GizmoConfigGroup>(g: &mut Gizmos<G>, ed: &Editor, i: usize) {
    let v = &ed.viewports.views[i];
    if !v.visible {
        return;
    }
    if v.grid {
        draw_grid(g, v);
    }
    let wire = v.shading == Shading::Wireframe;
    if !(wire || v.edged_faces) {
        return;
    }
    for (id, o) in ed.scene.objects.iter().filter(|(_, o)| !o.hidden) {
        let w = o.world_matrix();
        let [r, gg, b] = o.wire_color;
        let color = if wire && o.selected {
            Color::WHITE
        } else if wire {
            Color::srgb(r, gg, b)
        } else {
            Color::srgb(r * 0.35, gg * 0.35, b * 0.35)
        };
        let m = ed.display_mesh(id);
        for e in m.edges.values().filter(|e| !e.hidden) {
            g.line(w.transform_point3(m.pos(e.v0)), w.transform_point3(m.pos(e.v1)), color);
        }
    }
}

fn brackets(g: &mut Gizmos<OverlayLines>, w: Mat4, b: crate::mesh::Aabb) {
    let s = b.size() * 0.25;
    for corner in 0..8 {
        let pick = |bit: usize, axis: usize| {
            if corner >> bit & 1 == 1 { b.max[axis] } else { b.min[axis] }
        };
        let c = Vec3::new(pick(0, 0), pick(1, 1), pick(2, 2));
        for axis in 0..3 {
            let mut d = Vec3::ZERO;
            d[axis] = if corner >> axis & 1 == 1 { -s[axis] } else { s[axis] };
            g.line(w.transform_point3(c), w.transform_point3(c + d), Color::WHITE);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_lines(
    ed: Res<Editor>,
    mut g0: Gizmos<Vp0Lines>,
    mut g1: Gizmos<Vp1Lines>,
    mut g2: Gizmos<Vp2Lines>,
    mut g3: Gizmos<Vp3Lines>,
    mut ov: Gizmos<OverlayLines>,
    mut box_cache: Local<super::mod_gizmos::BoxCache>,
) {
    draw_view(&mut g0, &ed, 0);
    draw_view(&mut g1, &ed, 1);
    draw_view(&mut g2, &ed, 2);
    draw_view(&mut g3, &ed, 3);

    let current = ed.current();
    for (id, o) in ed.scene.objects.iter().filter(|(_, o)| o.selected && !o.hidden) {
        let w = o.world_matrix();
        if !(ed.in_sub_object() && Some(id) == current)
            && let Some(b) = o.bounds()
        {
            brackets(&mut ov, w, b);
        }
    }
    if ed.in_sub_object()
        && let Some(o) = ed.current_obj()
    {
        let w = o.world_matrix();
        let m = &o.level_mesh;
        let red = Color::srgb(1.0, 0.18, 0.12);
        let dim = Color::srgba(0.75, 0.78, 0.85, 0.35);
        let sel_polys = ed.sub.is_face_like();
        let poly_edges: std::collections::HashSet<_> = if sel_polys {
            m.selected_polygons()
                .iter()
                .flat_map(|&p| m.polygons[p].half_edges().map(|(a, b)| crate::mesh::edge_key(a, b)).collect::<Vec<_>>())
                .collect()
        } else {
            Default::default()
        };
        for e in m.edges.values() {
            let (a, b) = (w.transform_point3(m.pos(e.v0)), w.transform_point3(m.pos(e.v1)));
            let hot = (ed.sub.is_edge_like() && e.selected) || poly_edges.contains(&(e.v0, e.v1));
            ov.line(a, b, if hot { red } else { dim });
        }
        let _ = SubObject::Vertex;
    }
    super::mod_gizmos::draw(&mut ov, &ed, &mut box_cache);
    super::joint_gizmos::draw(&mut ov, &ed);
    if let (Some(sp), true) = (&ed.slice_plane, ed.in_sub_object()) {
        let m = sp.xform.matrix();
        let size = ed.current_obj().and_then(|o| o.evaluated_mesh.bounds()).map(|b| b.size().length() * 0.75).unwrap_or(1.0);
        let yellow = Color::srgb(1.0, 0.86, 0.1);
        let c = [Vec3::new(-1., -1., 0.), Vec3::new(1., -1., 0.), Vec3::new(1., 1., 0.), Vec3::new(-1., 1., 0.)]
            .map(|p| m.transform_point3(p * size));
        for k in 0..4 {
            ov.line(c[k], c[(k + 1) % 4], yellow);
        }
        ov.line(c[0], c[2], yellow.with_alpha(0.5));
        ov.line(c[1], c[3], yellow.with_alpha(0.5));
    }
}
