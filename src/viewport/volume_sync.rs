//! Live volumetric godrays in Perspective viewports: mirrors the
//! Environment's Volumetric Godrays & Mist settings onto Bevy's raymarched
//! volumetric fog (a fog volume around the scene, `VolumetricFog` on
//! perspective cameras, `VolumetricLight` on shadow-casting scene lights).

use super::light_sync::SceneLight;
use super::render_sync::ViewportCam;
use crate::editor::Editor;
use bevy::light::{FogVolume, VolumetricFog, VolumetricLight};
use bevy::prelude::*;

#[derive(Component)]
pub struct MistVolume;

pub fn register(app: &mut App) {
    app.add_systems(Update, sync_volume.after(super::light_sync::sync_lights));
}

#[allow(clippy::type_complexity)]
fn sync_volume(
    mut commands: Commands,
    editor: Res<Editor>,
    cams: Query<(Entity, &ViewportCam, Option<&VolumetricFog>)>,
    lights: Query<(Entity, Has<VolumetricLight>), With<SceneLight>>,
    mut vols: Query<(Entity, &mut FogVolume, &mut Transform), With<MistVolume>>,
) {
    let ed = &*editor;
    let v = &ed.scene.environment.volume;
    let on = v.enabled && v.density > 0.0;
    for (e, vc, fog) in &cams {
        let persp = !ed.viewports.views[vc.0].camera.is_ortho();
        match (on && persp, fog) {
            (true, f) if f.is_none_or(|f| f.step_count != v.steps) => {
                commands.entity(e).insert(VolumetricFog { step_count: v.steps.clamp(16, 128), ambient_intensity: 0.0, jitter: 0.0, ..default() });
            }
            (false, Some(_)) => {
                commands.entity(e).remove::<VolumetricFog>();
            }
            _ => {}
        }
    }
    for (e, has) in &lights {
        if on && !has {
            commands.entity(e).insert(VolumetricLight);
        } else if !on && has {
            commands.entity(e).remove::<VolumetricLight>();
        }
    }
    if !on {
        for (e, ..) in &vols {
            commands.entity(e).despawn();
        }
        return;
    }
    let (lo, hi) = ed
        .scene
        .objects
        .values()
        .filter(|o| !o.hidden && !o.is_helper())
        .map(crate::editor::align::world_bounds)
        .fold((Vec3::splat(-10.0), Vec3::splat(10.0)), |(lo, hi), b| (lo.min(b.min), hi.max(b.max)));
    let (c, r) = ((lo + hi) * 0.5, ((hi - lo).length() * 2.0).max(50.0));
    // Height falloff: the box tops out where the mist has thinned to ~5%.
    let top = if v.falloff > 0.0 { v.base_height + 3.0 / v.falloff } else { c.z + r * 0.5 };
    let bottom = (c.z - r * 0.5).min(v.base_height - 1.0);
    let want = FogVolume {
        fog_color: Color::srgb(v.color[0], v.color[1], v.color[2]),
        density_factor: v.density * (1.0 - 0.5 * v.wisps),
        scattering_asymmetry: v.anisotropy.clamp(0.0, 0.95),
        light_intensity: v.intensity,
        absorption: 0.1,
        scattering: 0.5,
        ..default()
    };
    let tf = Transform::from_translation(Vec3::new(c.x, c.y, (top + bottom) * 0.5)).with_scale(Vec3::new(r, r, (top - bottom).max(0.1)));
    match vols.iter_mut().next() {
        Some((_, mut f, mut t)) => {
            if f.density_factor != want.density_factor
                || f.scattering_asymmetry != want.scattering_asymmetry
                || f.light_intensity != want.light_intensity
                || f.fog_color != want.fog_color
            {
                *f = want;
            }
            if *t != tf {
                *t = tf;
            }
        }
        None => {
            commands.spawn((MistVolume, want, tf));
        }
    }
}
