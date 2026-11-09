//! Bevy side: viewport cameras and GPU mesh sync from the scene.

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
    look: (u32, bool, bool, Option<u64>),
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
    let [r, g, b] = editor.scene.environment.background;
    let bg = Color::srgb(r, g, b);
    for (vc, mut cam, mut tf, mut proj, mut layers) in &mut q {
        let v = &editor.viewports.views[vc.0];
        let pos = (v.rect.min * sf).max(Vec2::ZERO).as_uvec2().min(win.saturating_sub(UVec2::ONE));
        let size = (v.rect.size() * sf).max(Vec2::ONE).as_uvec2().min(win.saturating_sub(pos)).max(UVec2::ONE);
        let active = v.visible && v.rect.size().x > 2.0 && v.rect.size().y > 2.0;
        if cam.is_active != active {
            cam.is_active = active;
        }
        cam.viewport = Some(Viewport { physical_position: pos, physical_size: size, ..default() });
        // Environment background (8) clears the frame once, under every view.
        let clear = if vc.0 == first_visible { ClearColorConfig::Custom(bg) } else { ClearColorConfig::None };
        let same = match (&cam.clear_color, &clear) {
            (ClearColorConfig::None, ClearColorConfig::None) => true,
            (ClearColorConfig::Custom(a), ClearColorConfig::Custom(b)) => a == b,
            _ => false,
        };
        if !same {
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

/// 8x8 UV checker (tiles with the mesh's UVs; Repeat sampler so tiling > 1 works).
fn checker_image() -> Image {
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const N: u32 = 8;
    const PX: u32 = 16;
    let size = N * PX;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dark = ((x / PX) + (y / PX)).is_multiple_of(2);
            let c: [u8; 4] = if dark { [70, 72, 80, 255] } else { [225, 225, 230, 255] };
            data.extend_from_slice(&c);
        }
    }
    let mut img = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        ..default()
    });
    img
}

fn color_key(c: [f32; 3]) -> u32 {
    let q = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u32;
    q(c[0]) << 16 | q(c[1]) << 8 | q(c[2])
}

/// Default look: wire color (or the UV checker), like 3ds Max without a material.
fn wire_material(o: &crate::scene::SceneObject, checker_tex: &Handle<Image>) -> StandardMaterial {
    let [cr, cg, cb] = o.wire_color;
    let alpha = if o.see_through { 0.35 } else { 1.0 };
    let base =
        if o.checker { Color::srgba(1.0, 1.0, 1.0, alpha) } else { Color::srgba(cr * 0.85 + 0.1, cg * 0.85 + 0.1, cb * 0.85 + 0.1, alpha) };
    StandardMaterial {
        base_color: base,
        base_color_texture: o.checker.then(|| checker_tex.clone()),
        perceptual_roughness: 0.62,
        reflectance: 0.35,
        double_sided: true,
        cull_mode: None,
        alpha_mode: if o.see_through { AlphaMode::Blend } else { AlphaMode::Opaque },
        ..default()
    }
}

/// Uploads changed meshes, transforms and selection overlays.
#[allow(clippy::too_many_arguments)]
pub fn sync_meshes(
    mut commands: Commands,
    editor: Res<Editor>,
    mut map: ResMut<RenderMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut checker: Local<Option<Handle<Image>>>,
    cache: Res<super::mat_sync::MatCache>,
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
    let checker_tex = checker.get_or_insert_with(|| images.add(checker_image())).clone();
    let overlay_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.15, 0.12, 0.38),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    // Lights and targets have no geometry (see light_icons / light_sync).
    for (id, o) in scene.objects.iter().filter(|(_, o)| !o.is_helper()) {
        let r = map.objs.entry(id).or_insert_with(|| {
            let mesh = meshes.add(build_mesh(&o.evaluated_mesh.to_tri_mesh()));
            let material = materials.add(StandardMaterial::default());
            let overlay_mesh = meshes.add(selected_faces_mesh(&crate::mesh::EditablePoly::new()));
            let entity = commands
                .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::default(), RenderLayers::layer(LAYER_MESH)))
                .id();
            let overlay = commands
                .spawn((
                    SelectionOverlay,
                    Mesh3d(overlay_mesh.clone()),
                    MeshMaterial3d(overlay_mat.clone()),
                    Transform::default(),
                    RenderLayers::layer(LAYER_MESH),
                ))
                .id();
            ObjRender {
                entity,
                mesh,
                material,
                version: o.version,
                look: (u32::MAX, false, false, None),
                overlay,
                overlay_mesh,
                sel_version: (u64::MAX, SubObject::Object, false),
            }
        });
        if r.version != o.version {
            r.version = o.version;
            if let Some(mut m) = meshes.get_mut(&r.mesh) {
                *m = build_mesh(&o.evaluated_mesh.to_tri_mesh());
            }
        }
        // Assigned material (when "Show Shaded Material in Viewport" is on);
        // the UV checker overrides it while unwrapping.
        let shown = o.material.and_then(|m| cache.get(m)).filter(|(_, m, _)| m.show_in_viewport && !o.checker);
        let look = (color_key(o.wire_color), o.see_through, o.checker, shown.map(|(rev, _, _)| rev));
        if r.look != look {
            r.look = look;
            if let Some(mut mat) = materials.get_mut(&r.material) {
                *mat = match shown {
                    Some((_, m, tex)) => super::mat_sync::standard(m, tex, o.see_through),
                    None => wire_material(o, &checker_tex),
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
