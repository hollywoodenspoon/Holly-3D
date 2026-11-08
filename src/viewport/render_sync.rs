//! Bevy side: viewport cameras, lights, and GPU mesh sync from the scene.

use super::camera::ORTHO_DEPTH;
use super::layout::Shading;
use crate::editor::Editor;
use crate::mesh::SubObject;
use crate::scene::ObjId;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ScalingMode, Viewport};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiGlobalSettings, PrimaryEguiContext};
use std::collections::HashMap;

pub const LAYER_MESH: usize = 0;
pub const LAYER_OVERLAY: usize = 5;
pub fn view_layer(i: usize) -> usize {
    1 + i
}

pub const VIEWPORT_BG: Color = Color::srgb(0.235, 0.24, 0.255);

#[derive(Component)]
pub struct ViewportCam(pub usize);

#[derive(Component)]
pub struct SelectionOverlay;

struct ObjRender {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    version: u64,
    look: (u32, bool),
    overlay: Entity,
    overlay_mesh: Handle<Mesh>,
    sel_version: (u64, SubObject, bool),
}

#[derive(Resource, Default)]
pub struct RenderMap {
    objs: HashMap<ObjId, ObjRender>,
}

pub fn setup_scene(mut commands: Commands, mut egui_settings: ResMut<EguiGlobalSettings>) {
    egui_settings.auto_create_primary_context = false;
    for i in 0..4 {
        commands.spawn((
            ViewportCam(i),
            Camera3d::default(),
            Camera { order: i as isize, clear_color: ClearColorConfig::Custom(VIEWPORT_BG), ..default() },
            Msaa::Sample4,
            RenderLayers::from_layers(&[LAYER_MESH, view_layer(i), LAYER_OVERLAY]),
            Transform::default(),
        ));
    }
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        RenderLayers::none(),
        Camera { order: 10, clear_color: ClearColorConfig::None, ..default() },
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9_000.0, shadow_maps_enabled: false, ..default() },
        Transform::from_xyz(-3.0, -5.0, 8.0).looking_at(Vec3::ZERO, Vec3::Z),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 3_500.0, shadow_maps_enabled: false, ..default() },
        Transform::from_xyz(4.0, 6.0, 2.0).looking_at(Vec3::ZERO, Vec3::Z),
    ));
    commands.insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 450.0, affects_lightmapped_meshes: true });
}

/// Mirrors the editor's viewport cameras onto Bevy cameras.
pub fn sync_cameras(
    editor: Res<Editor>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut q: Query<(&ViewportCam, &mut Camera, &mut Transform, &mut Projection, &mut RenderLayers)>,
) {
    let sf = window.scale_factor();
    let win = UVec2::new(window.physical_width(), window.physical_height());
    let first_visible = editor.viewports.views.iter().position(|v| v.visible).unwrap_or(0);
    for (vc, mut cam, mut tf, mut proj, mut layers) in &mut q {
        let v = &editor.viewports.views[vc.0];
        let pos = (v.rect.min * sf).max(Vec2::ZERO).as_uvec2().min(win.saturating_sub(UVec2::ONE));
        let size = (v.rect.size() * sf).max(Vec2::ONE).as_uvec2().min(win.saturating_sub(pos)).max(UVec2::ONE);
        let active = v.visible && v.rect.size().x > 2.0 && v.rect.size().y > 2.0;
        if cam.is_active != active {
            cam.is_active = active;
        }
        cam.viewport = Some(Viewport { physical_position: pos, physical_size: size, ..default() });
        let clear = if vc.0 == first_visible { ClearColorConfig::Custom(VIEWPORT_BG) } else { ClearColorConfig::None };
        if !matches!((&cam.clear_color, &clear), (ClearColorConfig::None, ClearColorConfig::None) | (ClearColorConfig::Custom(_), ClearColorConfig::Custom(_))) {
            cam.clear_color = clear;
        }
        *tf = Transform::from_matrix(v.camera.world_from_view());
        let c = v.camera;
        *proj = if c.is_ortho() {
            let s = c.ortho_size(v.rect.aspect());
            Projection::Orthographic(OrthographicProjection {
                near: 0.0,
                far: ORTHO_DEPTH,
                scaling_mode: ScalingMode::Fixed { width: s.x, height: s.y },
                ..OrthographicProjection::default_3d()
            })
        } else {
            Projection::Perspective(PerspectiveProjection { fov: c.fov, near: 0.02, ..default() })
        };
        let mut want = vec![view_layer(vc.0), LAYER_OVERLAY];
        if v.shading == Shading::Default {
            want.push(LAYER_MESH);
        }
        let want = RenderLayers::from_layers(&want);
        if *layers != want {
            *layers = want;
        }
    }
}

fn build_mesh(t: &crate::mesh::triangulate::TriMesh) -> Mesh {
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, t.positions.clone());
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, t.normals.clone());
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, t.uvs.clone());
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, t.colors.clone());
    m.insert_indices(Indices::U32(t.indices.clone()));
    m
}

fn selected_faces_mesh(mesh: &crate::mesh::EditablePoly) -> Mesh {
    let mut pos = vec![];
    let lift = mesh.bounds().map(|b| b.size().length() * 0.002).unwrap_or(0.001);
    for pid in mesh.selected_polygons() {
        let n = mesh.poly_normal(pid) * lift;
        for tri in mesh.poly_triangles(pid) {
            pos.extend(tri.iter().map(|p| (*p + n).to_array()));
        }
    }
    let n = pos.len();
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
    m.insert_indices(Indices::U32((0..n as u32).collect()));
    m
}

fn color_key(c: [f32; 3]) -> u32 {
    let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u32;
    q(c[0]) << 16 | q(c[1]) << 8 | q(c[2])
}

/// Uploads changed meshes, transforms and selection overlays.
pub fn sync_meshes(
    mut commands: Commands,
    editor: Res<Editor>,
    mut map: ResMut<RenderMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tfs: Query<(&mut Transform, &mut Visibility), Without<ViewportCam>>,
) {
    let scene = &editor.scene;
    let dead: Vec<ObjId> = map.objs.keys().filter(|k| !scene.objects.contains_key(**k)).copied().collect();
    for k in dead {
        if let Some(r) = map.objs.remove(&k) {
            commands.entity(r.entity).despawn();
            commands.entity(r.overlay).despawn();
        }
    }
    let current = editor.current();
    let overlay_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.15, 0.12, 0.38),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    for (id, o) in &scene.objects {
        let r = map.objs.entry(id).or_insert_with(|| {
            let mesh = meshes.add(build_mesh(&o.evaluated_mesh.to_tri_mesh()));
            let material = materials.add(StandardMaterial::default());
            let overlay_mesh = meshes.add(selected_faces_mesh(&crate::mesh::EditablePoly::new()));
            let entity = commands
                .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::default(), RenderLayers::layer(LAYER_MESH)))
                .id();
            let overlay = commands
                .spawn((SelectionOverlay, Mesh3d(overlay_mesh.clone()), MeshMaterial3d(overlay_mat.clone()), Transform::default(), RenderLayers::layer(LAYER_MESH)))
                .id();
            ObjRender { entity, mesh, material, version: o.version, look: (u32::MAX, false), overlay, overlay_mesh, sel_version: (u64::MAX, SubObject::Object, false) }
        });
        if r.version != o.version {
            r.version = o.version;
            if let Some(mut m) = meshes.get_mut(&r.mesh) {
                *m = build_mesh(&o.evaluated_mesh.to_tri_mesh());
            }
        }
        let look = (color_key(o.wire_color), o.see_through);
        if r.look != look {
            r.look = look;
            if let Some(mut mat) = materials.get_mut(&r.material) {
                let [cr, cg, cb] = o.wire_color;
                let alpha = if o.see_through { 0.35 } else { 1.0 };
                *mat = StandardMaterial {
                    base_color: Color::srgba(cr * 0.85 + 0.1, cg * 0.85 + 0.1, cb * 0.85 + 0.1, alpha),
                    perceptual_roughness: 0.62,
                    reflectance: 0.35,
                    double_sided: true,
                    cull_mode: None,
                    alpha_mode: if o.see_through { AlphaMode::Blend } else { AlphaMode::Opaque },
                    ..default()
                };
            }
        }
        let is_current = Some(id) == current && editor.in_sub_object() && editor.sub.is_face_like();
        let sv = (o.selection_version, editor.sub, is_current);
        if r.sel_version != sv {
            r.sel_version = sv;
            if let Some(mut m) = meshes.get_mut(&r.overlay_mesh) {
                *m = if is_current { selected_faces_mesh(&o.level_mesh) } else { selected_faces_mesh(&crate::mesh::EditablePoly::new()) };
            }
        }
        let world = Transform::from_matrix(o.world_matrix());
        let vis = if o.hidden { Visibility::Hidden } else { Visibility::Inherited };
        for e in [r.entity, r.overlay] {
            if let Ok((mut tf, mut v)) = tfs.get_mut(e) {
                if *tf != world {
                    *tf = world;
                }
                if *v != vis {
                    *v = vis;
                }
            }
        }
    }
}
