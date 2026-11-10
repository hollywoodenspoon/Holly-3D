//! Mirrors scene lights onto Bevy lights, drives the Default Modeling
//! Lights headlight rig (Ctrl+L), ambient light / Skylights and fog.

use super::render_sync::ViewportCam;
use crate::editor::Editor;
use crate::render::lighting::headlight_rig;
use crate::render::{Light, LightKind};
use crate::scene::ObjId;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use std::collections::HashMap;

/// Illuminance of a multiplier-1 directional light (lux).
pub const DIRECT_LUX: f32 = 9_000.0;
/// Luminous power of a multiplier-1 omni / spot (lumens): matches a
/// directional key at ~4 units.
pub const POINT_LM: f32 = 1_800_000.0;
/// Ambient brightness at level 1 (cd/m²).
pub const AMBIENT_NITS: f32 = 450.0;

#[derive(Component)]
pub struct RigLight(usize);

#[derive(Component)]
pub struct SceneLight;

/// Which Bevy light component an entity carries.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Point,
    Spot,
    Directional,
}

fn shape(k: LightKind) -> Option<Shape> {
    match k {
        LightKind::Omni => Some(Shape::Point),
        LightKind::TargetSpot | LightKind::FreeSpot => Some(Shape::Spot),
        LightKind::TargetDirect | LightKind::FreeDirect => Some(Shape::Directional),
        LightKind::Skylight => None,
    }
}

#[derive(Resource, Default)]
pub struct LightMap {
    lights: HashMap<ObjId, (Entity, Shape)>,
}

pub fn register(app: &mut App) {
    app.init_resource::<LightMap>().add_systems(Startup, spawn_rig);
}

fn spawn_rig(mut commands: Commands) {
    for i in 0..2 {
        commands.spawn((RigLight(i), DirectionalLight { shadow_maps_enabled: false, ..default() }, Transform::default()));
    }
    commands.insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: AMBIENT_NITS, affects_lightmapped_meshes: true });
}

fn color(l: &Light) -> Color {
    Color::srgb(l.color[0], l.color[1], l.color[2])
}

fn spawn(commands: &mut Commands, s: Shape) -> Entity {
    let mut e = commands.spawn((SceneLight, Transform::default()));
    match s {
        Shape::Point => e.insert(PointLight::default()),
        Shape::Spot => e.insert(SpotLight::default()),
        Shape::Directional => e.insert(DirectionalLight::default()),
    };
    e.id()
}

/// Keeps target lights aimed, then syncs every light-related Bevy entity.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_lights(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    mut map: ResMut<LightMap>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut rig: Query<(&RigLight, &mut DirectionalLight, &mut Transform, &mut Visibility), Without<SceneLight>>,
    mut points: Query<&mut PointLight>,
    mut spots: Query<&mut SpotLight>,
    mut directs: Query<&mut DirectionalLight, (With<SceneLight>, Without<RigLight>)>,
    mut tfs: Query<(&mut Transform, &mut Visibility), (With<SceneLight>, Without<RigLight>)>,
    cams: Query<(Entity, &ViewportCam, Option<&DistanceFog>)>,
) {
    if editor.bypass_change_detection().scene.aim_targets() {
        editor.set_changed();
    }
    let ed = &*editor;
    let env = &ed.scene.environment;
    // Scene lights -> Bevy lights (Skylights only feed the ambient term).
    let wanted: HashMap<ObjId, Shape> =
        ed.scene.objects.iter().filter_map(|(k, o)| o.light().and_then(|l| shape(l.kind)).map(|s| (k, s))).collect();
    map.lights.retain(|k, (e, s)| {
        let keep = wanted.get(k) == Some(s);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    let use_rig = ed.uses_default_lights();
    for (k, s) in &wanted {
        let o = &ed.scene.objects[*k];
        let l = o.light().unwrap();
        let (e, _) = *map.lights.entry(*k).or_insert_with(|| (spawn(&mut commands, *s), *s));
        let on = l.on && !o.hidden && !use_rig;
        let col = color(l);
        if let Ok(mut p) = points.get_mut(e) {
            let v = PointLight {
                color: col,
                intensity: POINT_LM * l.multiplier,
                range: l.range(),
                radius: l.softness,
                shadow_maps_enabled: l.cast_shadows,
                shadow_depth_bias: l.shadow_bias * 4.0,
                ..default()
            };
            set_if(&mut p, v, |a, b| {
                a.color == b.color
                    && a.intensity == b.intensity
                    && a.range == b.range
                    && a.radius == b.radius
                    && a.shadow_maps_enabled == b.shadow_maps_enabled
                    && a.shadow_depth_bias == b.shadow_depth_bias
            });
        }
        if let Ok(mut p) = spots.get_mut(e) {
            let v = SpotLight {
                color: col,
                intensity: POINT_LM * l.multiplier,
                range: l.range(),
                radius: l.softness,
                shadow_maps_enabled: l.cast_shadows,
                shadow_depth_bias: l.shadow_bias * 4.0,
                outer_angle: (l.falloff * 0.5).to_radians(),
                inner_angle: (l.hotspot * 0.5).to_radians(),
                ..default()
            };
            set_if(&mut p, v, |a, b| {
                a.color == b.color
                    && a.intensity == b.intensity
                    && a.range == b.range
                    && a.radius == b.radius
                    && a.shadow_maps_enabled == b.shadow_maps_enabled
                    && a.shadow_depth_bias == b.shadow_depth_bias
                    && a.outer_angle == b.outer_angle
                    && a.inner_angle == b.inner_angle
            });
        }
        if let Ok(mut p) = directs.get_mut(e) {
            let v = DirectionalLight {
                color: col,
                illuminance: DIRECT_LUX * l.multiplier,
                shadow_maps_enabled: l.cast_shadows,
                shadow_depth_bias: l.shadow_bias,
                ..default()
            };
            set_if(&mut p, v, |a, b| {
                a.color == b.color
                    && a.illuminance == b.illuminance
                    && a.shadow_maps_enabled == b.shadow_maps_enabled
                    && a.shadow_depth_bias == b.shadow_depth_bias
            });
        }
        if let Ok((mut tf, mut vis)) = tfs.get_mut(e) {
            let t = Transform::from_translation(o.transform.translation).with_rotation(o.transform.rotation);
            if *tf != t {
                *tf = t;
            }
            let want = if on { Visibility::Inherited } else { Visibility::Hidden };
            if *vis != want {
                *vis = want;
            }
        }
    }
    // Headlight rig follows the active viewport camera.
    let cam = &ed.viewports.active_view().camera;
    let rig_lights = headlight_rig(cam.forward(), cam.right(), cam.up());
    for (r, mut d, mut tf, mut vis) in &mut rig {
        let rl = rig_lights[r.0];
        let want = if use_rig { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
        let t = Transform::from_rotation(crate::render::lighting::aim(Vec3::ZERO, rl.dir));
        if *tf != t {
            *tf = t;
        }
        let illum = DIRECT_LUX * rl.strength;
        let c = Color::linear_rgb(rl.color.x, rl.color.y, rl.color.z);
        if d.illuminance != illum || d.color != c {
            (d.illuminance, d.color) = (illum, c);
        }
    }
    // Ambient = environment tint * level + every Skylight that is on.
    let tint = Vec3::from_array(env.ambient.map(crate::materials::shading::srgb_to_linear)) * env.ambient_level;
    let sky: Vec3 =
        ed.scene.active_lights().filter(|(_, _, l)| l.kind == LightKind::Skylight && !use_rig).map(|(_, _, l)| l.radiance()).sum();
    let mut amb = tint + sky;
    if use_rig {
        amb = amb.max(Vec3::splat(0.25));
    }
    let peak = amb.max_element().max(1e-4);
    let (c, b) = (Color::linear_rgb(amb.x / peak, amb.y / peak, amb.z / peak), AMBIENT_NITS * peak);
    if ambient.color != c || ambient.brightness != b {
        (ambient.color, ambient.brightness) = (c, b);
    }
    // Exponential fog in perspective viewports.
    for (e, vc, fog) in &cams {
        let persp = !ed.viewports.views[vc.0].camera.is_ortho();
        let want = (env.fog && persp).then(|| DistanceFog {
            color: Color::srgb(env.fog_color[0], env.fog_color[1], env.fog_color[2]),
            falloff: FogFalloff::Exponential { density: env.fog_density },
            ..default()
        });
        match (want, fog) {
            (Some(w), Some(f))
                if f.color == w.color && matches!(f.falloff, FogFalloff::Exponential { density } if density == env.fog_density) => {}
            (Some(w), _) => {
                commands.entity(e).insert(w);
            }
            (None, Some(_)) => {
                commands.entity(e).remove::<DistanceFog>();
            }
            (None, None) => {}
        }
    }
}

/// Writes `v` only when it differs (avoids needless change detection).
fn set_if<T>(dst: &mut Mut<T>, v: T, same: impl Fn(&T, &T) -> bool) {
    if !same(dst, &v) {
        **dst = v;
    }
}
