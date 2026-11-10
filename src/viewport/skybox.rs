//! HDRI skybox behind Perspective / camera viewports (Environment `8` ->
//! Show in Perspective Viewports). The equirect image is resampled to a
//! cubemap on the CPU once per image / rotation change.

use super::render_sync::ViewportCam;
use crate::editor::Editor;
use bevy::asset::RenderAssetUsages;
use bevy::light::Skybox;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};

/// Cube face edge in texels.
const FACE: u32 = 256;
/// Skybox brightness (cd/m²) for HDRI radiance 1 at the default exposure.
pub const SKY_NITS: f32 = 1_000.0;

/// The uploaded cubemap and the image / rotation it was built from.
#[derive(Resource, Default)]
pub struct SkyboxCache {
    key: Option<(usize, u32)>,
    handle: Option<Handle<Image>>,
}

fn cubemap(faces: Vec<[f32; 4]>) -> Image {
    let data: Vec<u8> = faces.iter().flat_map(|p| p.map(|c| half::f16::from_f32(c).to_le_bytes())).flatten().collect();
    let mut img = Image::new(
        Extent3d { width: FACE, height: FACE, depth_or_array_layers: 6 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::Cube), ..default() });
    img
}

/// Adds / removes the `Skybox` on viewport cameras to follow the scene's HDRI.
pub fn sync_skybox(
    mut commands: Commands,
    editor: Res<Editor>,
    mut cache: ResMut<SkyboxCache>,
    mut images: ResMut<Assets<Image>>,
    q: Query<(Entity, &ViewportCam, Option<&Skybox>)>,
) {
    let hdri = editor.scene.environment.hdri.as_ref().filter(|h| h.show_in_viewport);
    let key = hdri.and_then(|h| h.key());
    if key != cache.key {
        if let Some(old) = cache.handle.take() {
            images.remove(&old);
        }
        cache.handle =
            hdri.and_then(|h| h.image.as_ref()).map(|i| images.add(cubemap(i.cube_faces(FACE, hdri.map_or(0.0, |h| h.rotation)))));
        cache.key = key;
    }
    let brightness = hdri.map_or(0.0, |h| h.strength * SKY_NITS);
    for (e, vc, sky) in &q {
        let persp = !editor.viewports.views[vc.0].camera.is_ortho();
        match (cache.handle.as_ref().filter(|_| persp), sky) {
            (Some(h), Some(s)) if s.image.as_ref() == Some(h) && s.brightness == brightness => {}
            (Some(h), _) => {
                commands.entity(e).insert(Skybox { image: Some(h.clone()), brightness, ..default() });
            }
            (None, Some(_)) => {
                commands.entity(e).remove::<Skybox>();
            }
            (None, None) => {}
        }
    }
}

pub fn register(app: &mut App) {
    app.init_resource::<SkyboxCache>().add_systems(Update, sync_skybox);
}
