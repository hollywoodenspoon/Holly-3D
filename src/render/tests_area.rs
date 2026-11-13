//! Area lights and IES profiles: far-field power matches an Omni, shadows
//! soften with size, Visible to Camera, and the analytic photometric webs.

use super::area_lights::{Area, AreaShape, IesProfile};
use super::scene_build::{RShape, RenderScene};
use super::trace::{Rng, Tracer};
use super::volumetrics::incident;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::render::{Light, LightKind, RenderSettings};
use crate::scene::{BaseObject, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::{Quat, Vec3};

fn scene_with(light: Light, at: Vec3) -> RenderScene {
    let mut s = Scene::default();
    s.add_light(light, None, at, Vec3::ZERO);
    RenderScene::build(&s, ViewCamera::new(ViewKind::Perspective), false)
}

/// Mean intensity factor at the origin over many light samples.
fn mean_k(rs: &RenderScene, p: Vec3) -> f32 {
    let mut rng = Rng::new(7);
    let n = 20_000;
    (0..n).filter_map(|_| incident(&rs.lights[0], p, &mut rng)).map(|(_, _, k)| k).sum::<f32>() / n as f32
}

#[test]
fn small_area_lights_match_an_omni_far_away() {
    let at = Vec3::new(0.0, 0.0, 10.0);
    let omni = mean_k(&scene_with(Light::new(LightKind::Omni), at), Vec3::ZERO);
    assert!((omni - 0.01).abs() < 1e-4, "inverse square: {omni}");
    for kind in [LightKind::RectArea, LightKind::DiscArea, LightKind::SphereArea, LightKind::TubeArea] {
        let rs = scene_with(Light::new(kind), at);
        assert!(matches!(rs.lights[0].shape, RShape::Area(_)), "{kind:?} renders as an area light");
        let k = mean_k(&rs, Vec3::ZERO);
        assert!((k - omni).abs() < 0.04 * omni, "{kind:?}: {k} vs omni {omni}");
    }
}

#[test]
fn panels_emit_from_one_side_only() {
    let rs = scene_with(Light::new(LightKind::RectArea), Vec3::new(0.0, 0.0, 4.0));
    let mut rng = Rng::new(1);
    assert!(incident(&rs.lights[0], Vec3::ZERO, &mut rng).is_some(), "lit below a downward panel");
    assert!(incident(&rs.lights[0], Vec3::new(0.0, 0.0, 8.0), &mut rng).is_none(), "dark behind it");
}

#[test]
fn bigger_lights_cast_softer_shadows() {
    // A 1×1 occluder at z = 2 between a light at z = 4 and a floor point
    // just past the occluder's shadow edge.
    let share = |size_per_hx: f32| {
        let mut s = Scene::default();
        let id = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Plane)), Vec3::new(0.0, 0.0, 2.0));
        s.objects[id].transform.scale = Vec3::splat(0.1);
        s.objects[id].evaluate();
        let hx = s.objects[id].evaluated_mesh.vertices.values().map(|v| v.position.x.abs()).fold(0.0f32, f32::max) * 0.1;
        let mut l = Light::new(LightKind::RectArea);
        l.area_size = [size_per_hx * hx; 2];
        s.add_light(l, None, Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO);
        let rs = RenderScene::build(&s, ViewCamera::new(ViewKind::Perspective), false);
        let cfg = RenderSettings { shadow_samples: 64, ..Default::default() };
        let t = Tracer::new(&rs, &cfg);
        // Inside the hard umbra of a point light, near its edge.
        let edge = Vec3::new(hx * 1.6, 0.0, 0.0);
        let mut rng = Rng::new(5);
        let lit: f32 = (0..64)
            .filter_map(|_| incident(&rs.lights[0], edge, &mut rng))
            .map(|(dir, dist, _)| t.transmittance(edge + Vec3::Z * 1e-3, dir, dist - 1e-2))
            .sum();
        lit / 64.0
    };
    let (tiny, big) = (share(0.01), share(2.0));
    assert!(tiny < 0.05, "a tiny light leaves a hard umbra: {tiny}");
    assert!(big > 0.2 && big < 0.95, "a big softbox leaves a penumbra: {big}");
}

#[test]
fn area_shapes_intersect_rays() {
    let down = Quat::IDENTITY; // emission along -Z
    let c = Vec3::new(0.0, 0.0, 5.0);
    let rect = Area::new(AreaShape::Rect, down, [2.0, 1.0], true);
    assert_eq!(rect.hit(c, Vec3::ZERO, Vec3::Z), Some(5.0), "seen from the lit side");
    assert_eq!(rect.hit(c, Vec3::new(0.0, 0.0, 9.0), Vec3::NEG_Z), None, "back face is not drawn");
    assert_eq!(rect.hit(c, Vec3::new(0.0, 0.8, 0.0), Vec3::Z), None, "outside the 1-unit length");
    let disc = Area::new(AreaShape::Disc, down, [0.5, 0.5], true);
    assert!(disc.hit(c, Vec3::new(0.4, 0.0, 0.0), Vec3::Z).is_some() && disc.hit(c, Vec3::new(0.6, 0.0, 0.0), Vec3::Z).is_none());
    let sphere = Area::new(AreaShape::Sphere, down, [0.5, 0.5], true);
    assert!((sphere.hit(c, Vec3::ZERO, Vec3::Z).unwrap() - 4.5).abs() < 1e-4);
    let tube = Area::new(AreaShape::Tube, down, [0.1, 2.0], true);
    assert!((tube.hit(c, Vec3::new(0.5, 0.0, 0.0), Vec3::Z).unwrap() - 4.9).abs() < 1e-3, "along the tube");
    assert!(tube.hit(c, Vec3::new(1.5, 0.0, 0.0), Vec3::Z).is_none(), "past its end");
}

#[test]
fn visible_to_camera_shows_the_emitter() {
    for visible in [true, false] {
        let mut l = Light::new(LightKind::RectArea);
        l.visible_to_camera = visible;
        let rs = scene_with(l, Vec3::new(0.0, 0.0, 5.0));
        let cfg = RenderSettings::default();
        let t = Tracer::new(&rs, &cfg);
        let seen = t.area_seen(Vec3::ZERO, Vec3::Z, f32::INFINITY);
        assert_eq!(seen.is_some(), visible);
        let mut rng = Rng::new(2);
        let (c, a, _) = t.trace(Vec3::ZERO, Vec3::Z, 0, &mut rng);
        assert_eq!(a > 0.0, visible, "camera ray coverage");
        assert_eq!(c.max_element() > 1.0, visible, "bright softbox: {c}");
    }
    assert_eq!(crate::render::gpu_pathtrace::pack::unsupported(&scene_with(Light::new(LightKind::DiscArea), Vec3::Z)), None, "area lights render on the GPU");
}

#[test]
fn ies_profiles_shape_the_beam() {
    let at = |p: IesProfile, deg: f32| p.factor(deg.to_radians().cos());
    for p in IesProfile::ALL {
        assert!((0..=90).all(|d| (0.0..=1.0).contains(&at(p, d as f32))), "{p:?} stays in 0..1");
    }
    assert_eq!(at(IesProfile::None, 80.0), 1.0);
    assert!(at(IesProfile::StageNarrow, 0.0) > 0.99 && at(IesProfile::StageNarrow, 20.0) == 0.0, "narrow stage beam");
    assert!(at(IesProfile::StreetlampWide, 62.0) > 2.0 * at(IesProfile::StreetlampWide, 0.0), "batwing throws sideways");
    assert!(at(IesProfile::TacticalFlashlight, 11.0) > 1.5 * at(IesProfile::TacticalFlashlight, 7.5), "inner reflector ring");
    assert!(at(IesProfile::WallScallop, 30.0) > at(IesProfile::WallScallop, 5.0) && at(IesProfile::WallScallop, 70.0) == 0.0, "scallop arc then cut");
    assert!(at(IesProfile::HighBay, 0.0) > at(IesProfile::HighBay, 45.0) && at(IesProfile::HighBay, 80.0) == 0.0);
    // Applied to a spot in the renderer.
    let mut l = Light::new(LightKind::FreeSpot);
    (l.ies, l.hotspot, l.falloff) = (IesProfile::StageNarrow, 170.0, 175.0);
    let rs = scene_with(l, Vec3::new(0.0, 0.0, 5.0));
    let mut rng = Rng::new(1);
    let on = incident(&rs.lights[0], Vec3::ZERO, &mut rng).map_or(0.0, |x| x.2);
    let off = incident(&rs.lights[0], Vec3::new(3.0, 0.0, 0.0), &mut rng).map_or(0.0, |x| x.2);
    assert!(on > 0.03 && off == 0.0, "IES narrows a wide spot: {on} / {off}");
}
