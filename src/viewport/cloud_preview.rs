//! Clouds V2b: soft volume preview of clouds in Perspective viewports.
//! Each cloud's eroded density is baked on a worker thread into a small 3D
//! texture and drawn by Bevy's raymarched fog volumes, lit by the scene's
//! lights and a sky ambient. The solid preview puffs move to their own
//! render layer, which only views without the preview draw.

use super::layout::{Shading, ViewState};
use super::render_sync::RenderMap;
use crate::editor::Editor;
use crate::materials::volume::VolumeMat;
use crate::render::volumes::bake::{Baked, bake, preview_volume};
use crate::render::volumes::volume_of;
use crate::scene::{ObjId, Scene, SceneObject};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::image::ImageSampler;
use bevy::light::FogVolume;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use std::collections::HashMap;

/// Render layer of cloud preview puffs while their volume preview shows.
pub const LAYER_PUFFS: usize = 6;
/// Voxels per baked cloud (about 48³).
const BUDGET: u32 = 110_000;

/// True where view `v` draws cloud volumes instead of the puffs.
pub fn volumes_in(on: bool, v: &ViewState) -> bool {
    on && v.shading == Shading::Default && !v.camera.is_ortho()
}

/// A cloud object the preview shows as a volume.
pub fn previewed(scene: &Scene, o: &SceneObject) -> bool {
    !o.hidden && o.is_cloud() && volume_of(scene, o).is_some()
}

/// True when any viewport has cloud volumes to draw (cameras need fog on).
pub fn any_volumes(ed: &Editor) -> bool {
    ed.prefs.viewport.preview_volumes && ed.scene.objects.values().any(|o| previewed(&ed.scene, o))
}

#[derive(Component)]
struct CloudVolume;

type Key = (u64, VolumeMat);

#[derive(Default)]
struct Entry {
    key: Option<Key>,
    task: Option<(Key, Task<Baked>)>,
    fog: Option<Entity>,
    image: Option<Handle<Image>>,
    baked: Option<(Vec3, Vec3)>,
}

#[derive(Resource, Default)]
struct Previews(HashMap<ObjId, Entry>);

pub fn register(app: &mut App) {
    app.init_resource::<Previews>().add_systems(Update, sync_clouds.after(super::render_sync::sync_meshes));
}

/// Peak optical depth across the box radius. Bevy's fog self-shadows every
/// sample by `exp(-density * radius)`, so a real cloud's extinction would
/// turn the preview black; capping it keeps the billows readable.
const PREVIEW_DEPTH: f32 = 2.0;
/// Makes up for the isotropic phase spreading the headlight over the whole
/// sphere (the solid puffs it replaces read about this bright).
const PREVIEW_LIGHT: f32 = 8.0;

/// The fog volume for material `m` over a box of world half-diagonal `radius`.
fn fog_of(m: &VolumeMat, image: Handle<Image>, radius: f32) -> FogVolume {
    let a = m.absorption.clamp(0.0, 1.0);
    // The modelling headlight shines from behind the camera, so a cloud's
    // strong forward scattering would leave it black: the preview scatters
    // nearly evenly, keeping only a hint of the material's lobe.
    let g = ((m.g_forward * (1.0 - m.blend) + m.g_back * m.blend) * 0.15).clamp(-0.2, 0.2);
    FogVolume {
        fog_color: Color::srgb(m.scatter[0], m.scatter[1], m.scatter[2]),
        density_factor: m.density.clamp(0.0, PREVIEW_DEPTH / radius.max(1e-3)),
        light_intensity: PREVIEW_LIGHT,
        density_texture: Some(image),
        absorption: a,
        scattering: 1.0 - a,
        scattering_asymmetry: g,
        ..default()
    }
}

fn image_of(b: &Baked) -> Image {
    let [w, h, d] = b.dims;

    let size = Extent3d { width: w, height: h, depth_or_array_layers: d };
    let mut img = Image::new(size, TextureDimension::D3, b.data.clone(), TextureFormat::R8Unorm, RenderAssetUsages::RENDER_WORLD);
    img.sampler = ImageSampler::linear();
    img
}

#[allow(clippy::too_many_arguments)]
fn sync_clouds(
    mut commands: Commands,
    editor: Res<Editor>,
    map: Res<RenderMap>,
    mut prev: ResMut<Previews>,
    mut images: ResMut<Assets<Image>>,
    mut layers: Query<&mut RenderLayers>,
    mut fogs: Query<&mut Transform, With<CloudVolume>>,
) {
    let ed = &*editor;
    let on = ed.prefs.viewport.preview_volumes;
    let live: Vec<ObjId> = ed.scene.objects.iter().filter(|(_, o)| on && previewed(&ed.scene, o)).map(|(id, _)| id).collect();
    // Retire previews of deleted, hidden or switched-off clouds.
    prev.0.retain(|id, e| {
        let keep = live.contains(id);
        if !keep && let Some(f) = e.fog.take() {
            commands.entity(f).despawn();
        }
        keep
    });
    // Puffs: on their own layer while a preview exists, else back to meshes.
    for (id, _) in ed.scene.objects.iter().filter(|(_, o)| o.is_cloud()) {
        let Some(ent) = map.entity(id) else { continue };
        let want = RenderLayers::layer(if live.contains(&id) { LAYER_PUFFS } else { super::render_sync::LAYER_MESH });
        if let Ok(mut l) = layers.get_mut(ent)
            && *l != want
        {
            *l = want;
        }
    }
    for id in live {
        let o = &ed.scene.objects[id];
        let e = prev.0.entry(id).or_default();
        let Some(mat) = volume_of(&ed.scene, o) else { continue };
        let key: Key = (o.version, mat);
        // Start a bake when the cloud changed and none is running for it.
        let stale = e.key.as_ref() != Some(&key) && e.task.as_ref().is_none_or(|(k, _)| *k != key);
        if stale && let Some(v) = preview_volume(&ed.scene, o) {
            e.task = Some((key.clone(), AsyncComputeTaskPool::get().spawn(async move { bake(v, BUDGET) })));
        }
        let done = e.task.as_mut().and_then(|(k, task)| block_on(poll_once(task)).map(|b| (k.clone(), b)));
        if let Some((k, b)) = done {
            e.key = Some(k);
            e.task = None;
            let img = images.add(image_of(&b));
            e.baked = Some((b.lo, b.hi));
            let scale = o.world_matrix().to_scale_rotation_translation().0.abs().max_element();
            let fog = fog_of(&b.mat, img.clone(), (b.hi - b.lo).length() * 0.5 * scale);
            match e.fog {
                Some(f) => {
                    commands.entity(f).insert(fog);
                }
                None => e.fog = Some(commands.spawn((CloudVolume, fog, Transform::default())).id()),
            }
            if let Some(old) = e.image.replace(img) {
                images.remove(&old);
            }
        }
        // Follow the object's transform every frame (moves need no bake).
        if let (Some(f), Some((lo, hi))) = (e.fog, e.baked) {
            let local = Mat4::from_scale_rotation_translation((hi - lo).max(Vec3::splat(1e-3)), Quat::IDENTITY, (lo + hi) * 0.5);
            let tf = Transform::from_matrix(o.world_matrix() * local);
            if let Ok(mut t) = fogs.get_mut(f)
                && *t != tf
            {
                *t = tf;
            }
        }
    }
}
