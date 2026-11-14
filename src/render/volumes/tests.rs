use super::density::{Source, Voxels};
use super::*;
use crate::generators::clouds::{CloudParams, CloudType};
use crate::materials::volume::{VolumePreset, VolumeMat};
use crate::primitives::{Primitive, PrimitiveKind};
use crate::render::pathtrace::PathSettings;
use crate::render::pathtrace::integrator::PathTracer;
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::trace::{Rng, Tracer};
use crate::viewport::camera::{ViewCamera, ViewKind};

fn vol_mat(density: f32) -> Material {
    let mut m = Material::named("Vol");
    m.model = ShadingModel::Volume;
    m.volume = VolumeMat { density, ..VolumePreset::CumulusCloud.params() };
    m
}

fn add(s: &mut Scene, base: BaseObject, at: Vec3, m: Option<Material>) -> crate::scene::ObjId {
    let id = s.add(base, at);
    if let Some(m) = m {
        let mid = s.materials.mats.insert(m);
        s.objects[id].material = Some(mid);
    }
    s.objects[id].evaluate();
    id
}

fn box_scene(density: f32) -> Scene {
    let mut s = Scene::default();
    add(&mut s, BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::ZERO, Some(vol_mat(density)));
    s
}

fn cam() -> ViewCamera {
    let mut c = ViewCamera::new(ViewKind::Front);
    c.target = Vec3::ZERO;
    c
}

#[test]
fn volume_objects_become_media_not_triangles() {
    let rs = RenderScene::build(&box_scene(0.5), cam(), true);
    assert!(rs.bvh.tris.is_empty(), "a Volume material makes no surface");
    assert_eq!(rs.volumes.list.len(), 1);
    let reasons = crate::render::gpu_pathtrace::features::reasons(&rs);
    assert!(reasons.contains(&crate::render::gpu_pathtrace::features::CPU_ONLY[2]), "{reasons:?}");
    // Density 0 or a surface material: no volume.
    assert!(RenderScene::build(&box_scene(0.0), cam(), true).volumes.is_empty());
}

#[test]
fn box_transmittance_matches_beer_lambert() {
    let rs = RenderScene::build(&box_scene(0.7), cam(), true);
    let v = &rs.volumes.list[0];
    let (lo, hi) = (v.grid.lo, v.grid.hi);
    let mid = (lo + hi) * 0.5;
    let len = hi.x - lo.x;
    let mut rng = Rng::new(3);
    let t = rs.volumes.transmittance(mid - Vec3::X * 5.0, Vec3::X, 100.0, 1, &mut rng);
    assert!((t - (-0.7 * len).exp()).abs() < 1e-4, "{t} vs {}", (-0.7 * len).exp());
    // A ray that stops halfway sees half the depth; a miss sees none.
    let half = rs.volumes.transmittance(mid - Vec3::X * 5.0, Vec3::X, 5.0, 1, &mut rng);
    assert!((half - (-0.7 * len * 0.5).exp()).abs() < 1e-3);
    assert_eq!(rs.volumes.transmittance(mid + Vec3::Y * 10.0, Vec3::X, 100.0, 1, &mut rng), 1.0);
}

#[test]
fn ratio_and_delta_tracking_are_unbiased_on_a_grid() {
    let (lo, hi) = (Vec3::ZERO, Vec3::splat(2.0));
    let src = Source::Grid(Voxels { dims: [4, 4, 4], data: vec![0.6; 64] });
    let mat = VolumeMat { density: 0.8, ..Default::default() };
    let v = VolumeObj::new(src, lo, hi, Mat4::IDENTITY, mat, 0);
    let want = (-0.6f32 * 0.8 * 2.0).exp();
    let mut rng = Rng::new(11);
    let n = 20_000;
    let o = Vec3::new(-1.0, 1.0, 1.0);
    let ratio: f32 = (0..n).map(|_| v.transmittance(o, Vec3::X, 10.0, &mut rng)).sum::<f32>() / n as f32;
    assert!((ratio - want).abs() < 0.01, "ratio tracking {ratio} vs {want}");
    let passed = (0..n).filter(|_| v.collide(o, Vec3::X, 10.0, &mut rng).is_none()).count() as f32 / n as f32;
    assert!((passed - want).abs() < 0.015, "delta tracking {passed} vs {want}");
    // Collisions land inside the box, with t in world metres.
    let (t, d) = (0..100).find_map(|_| v.collide(o, Vec3::X, 10.0, &mut rng)).unwrap();
    assert!((1.0..=3.0).contains(&t) && (d - 0.6).abs() < 1e-4, "t {t} d {d}");
}

#[test]
fn majorant_bounds_every_cloud_density() {
    let mut s = Scene::default();
    let p = CloudParams { detail: 2, base_flatness: 0.6, ..CloudParams::of(CloudType::Cumulus) };
    add(&mut s, BaseObject::World(WorldGen::Cloud(Box::new(p))), Vec3::ZERO, Some(vol_mat(0.05)));
    let rs = RenderScene::build(&s, cam(), true);
    let v = &rs.volumes.list[0];
    assert!(matches!(v.src, Source::Puffs(ref b) if b.len() > 10));
    let mut rng = Rng::new(5);
    let (lo, hi) = (v.grid.lo, v.grid.hi);
    let mut dense = 0;
    for _ in 0..20_000 {
        let p = lo + (hi - lo) * Vec3::new(rng.f(), rng.f(), rng.f());
        let d = v.density(p);
        assert!((0.0..=1.0).contains(&d));
        assert!(d <= v.grid.max[v.grid.index(v.grid.cell_of(p))] + 1e-5, "majorant bound at {p}");
        dense += (d > 0.5) as u32;
    }
    assert!(dense > 500, "the cloud has a dense core ({dense})");
    // Base Flatness cuts the underside: little density below the base.
    let below = (0..4000).filter(|_| v.density(Vec3::new(lo.x + (hi.x - lo.x) * rng.f(), lo.y + (hi.y - lo.y) * rng.f(), lo.z * rng.f())) > 0.05).count();
    assert!(below < 400, "{below}");
}

#[test]
fn clouds_shadow_the_ground_and_scale_with_the_object() {
    let mut s = box_scene(2.0);
    let id = s.objects.keys().next().unwrap();
    s.objects[id].transform.translation = Vec3::new(0.0, 0.0, 3.0);
    s.objects[id].transform.scale = Vec3::splat(2.0);
    let rs = RenderScene::build(&s, cam(), true);
    let cfg = RenderSettings::default();
    let t = Tracer::new(&rs, &cfg);
    let under = t.transmittance(Vec3::ZERO, Vec3::Z, 100.0);
    let open = t.transmittance(Vec3::new(10.0, 0.0, 0.0), Vec3::Z, 100.0);
    assert_eq!(open, 1.0);
    let v = &rs.volumes.list[0];
    let thick = (v.grid.hi.z - v.grid.lo.z) * 2.0;
    assert!((under - (-2.0 * thick).exp()).abs() < 1e-3, "shadow {under}, scaled thickness {thick}");
}

#[test]
fn camera_inside_a_volume_scatters_and_renders() {
    let rs = RenderScene::build(&box_scene(3.0), cam(), true);
    let v = &rs.volumes.list[0];
    let mid = (v.grid.lo + v.grid.hi) * 0.5;
    let mut rng = Rng::new(2);
    let c = rs.volumes.collide(mid, Vec3::Y, 100.0, &mut rng).expect("dense box collides from inside");
    assert!(c.t < 1.0);
    let cfg = RenderSettings::default();
    let t = Tracer::new(&rs, &cfg);
    let (col, a, _) = t.trace(mid, Vec3::Y, 0, &mut rng);
    assert!(a == 1.0 && col.is_finite() && col.max_element() > 0.0, "{col} {a}");
    let ps = PathSettings::default();
    let pt = PathTracer::new(Tracer::new(&rs, &cfg), &ps);
    let mut sum = Vec3::ZERO;
    for _ in 0..200 {
        let s = pt.radiance(mid, Vec3::Y, &mut rng);
        assert!(s.radiance.is_finite());
        sum += s.radiance;
    }
    assert!(sum.max_element() > 0.0, "lit by the headlight rig");
}

#[test]
fn white_cloud_conserves_energy_and_more_bounces_brighten() {
    let rs = RenderScene::build(&box_scene(4.0), cam(), true);
    let v = &rs.volumes.list[0];
    let mid = (v.grid.lo + v.grid.hi) * 0.5;
    let o = mid - Vec3::Y * 5.0;
    let mean = |bounces: u32, boost: f32| {
        let mut cfg = RenderSettings::default();
        cfg.volumes.max_bounces = bounces;
        cfg.volumes.boost = boost;
        let ps = PathSettings::default();
        let pt = PathTracer::new(Tracer::new(&rs, &cfg), &ps);
        let mut rng = Rng::new(9);
        (0..3000).map(|_| pt.radiance(o, Vec3::Y, &mut rng).radiance).sum::<Vec3>() / 3000.0
    };
    let (one, many) = (mean(1, 0.0), mean(16, 0.0));
    assert!(many.x > one.x, "multiple scattering adds light ({one} -> {many})");
    assert!(mean(1, 1.0).x > one.x, "the boost brightens single scattering");
    let lights: f32 = rs.lights.iter().map(|l| l.radiance.max_element()).sum::<f32>() + rs.ambient.max_element() + 1.0;
    assert!(many.max_element() < lights, "no energy gain: {many} vs {lights}");
}

#[test]
fn presets_and_old_files_load() {
    for p in VolumePreset::ALL {
        let m = p.params();
        assert!(m.albedo().max_element() <= 1.0 && m.density > 0.0, "{p:?}");
        let back: VolumeMat = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back, m);
    }
    assert!(VolumePreset::Fire.params().glow(1.0).max_element() > 0.0);
    assert_eq!(VolumePreset::CumulusCloud.params().glow(1.0), Vec3::ZERO);
    let r: VolumeRender = serde_json::from_str("{}").unwrap();
    assert_eq!(r, VolumeRender::default());
    let old: RenderSettings = serde_json::from_str(r#"{"width": 640}"#).unwrap();
    assert_eq!(old.volumes.max_bounces, 8);
}
