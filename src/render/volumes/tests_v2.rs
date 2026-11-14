//! Clouds V2: clouds render as volumes by default, with erosion that only
//! ever lowers density, Render As: Mesh, presets and the Density multiplier.

use super::density::Source;
use super::erode::{NoiseDetail, noise};
use super::*;
use crate::generators::clouds::{CloudFieldParams, CloudLook, CloudParams, CloudType, RenderAs};
use crate::render::scene_build::RenderScene;
use crate::render::trace::Rng;
use crate::viewport::camera::{ViewCamera, ViewKind};

fn cloud(p: CloudParams) -> Scene {
    let mut s = Scene::default();
    let id = s.add(BaseObject::World(WorldGen::Cloud(Box::new(p))), Vec3::ZERO);
    s.objects[id].evaluate();
    s
}

fn build(s: &Scene) -> RenderScene {
    RenderScene::build(s, ViewCamera::new(ViewKind::Front), true)
}

fn with_look(look: CloudLook) -> CloudParams {
    CloudParams { detail: 2, look, ..CloudParams::of(CloudType::Cumulus) }
}

#[test]
fn a_cloud_renders_as_a_volume_without_a_volume_material() {
    let rs = build(&cloud(CloudParams::default()));
    assert!(rs.bvh.tris.is_empty(), "no preview mesh in the render");
    let v = &rs.volumes.list[0];
    assert_eq!(v.mat, VolumePreset::CumulusCloud.params());
    // A storm tower gets the Storm Cloud preset; Density scales it.
    let look = CloudLook { density: 2.0, ..Default::default() };
    let rs = build(&cloud(CloudParams { look, ..CloudParams::of(CloudType::Cumulonimbus) }));
    assert!((rs.volumes.list[0].sigma() - 2.0 * VolumePreset::StormCloud.params().density).abs() < 1e-6);
    // Render As: Mesh keeps the preview mesh and makes no volume.
    let rs = build(&cloud(with_look(CloudLook { render_as: RenderAs::Mesh, ..Default::default() })));
    assert!(!rs.bvh.tris.is_empty() && rs.volumes.is_empty());
    // A Cloud Field renders as one volume holding every cloud's puffs.
    let mut s = Scene::default();
    let f = CloudFieldParams { width: 1500.0, depth: 1500.0, detail: 1, ..Default::default() };
    let id = s.add(BaseObject::World(WorldGen::CloudField(Box::new(f))), Vec3::ZERO);
    s.objects[id].evaluate();
    let rs = build(&s);
    assert!(rs.bvh.tris.is_empty() && rs.volumes.list.len() == 1);
}

#[test]
fn erosion_only_lowers_density_and_keeps_the_core() {
    let smooth = build(&cloud(with_look(CloudLook { erosion: 0.0, ..Default::default() })));
    let worn = build(&cloud(with_look(CloudLook { erosion: 1.0, wispiness: 1.0, ..Default::default() })));
    let (a, b) = (&smooth.volumes.list[0], &worn.volumes.list[0]);
    assert_eq!(a.grid.max, b.grid.max, "erosion leaves the majorants alone");
    let mut rng = Rng::new(4);
    let (lo, hi) = (a.grid.lo, a.grid.hi);
    let (mut eroded, mut core) = (0, 0);
    for _ in 0..20_000 {
        let p = lo + (hi - lo) * Vec3::new(rng.f(), rng.f(), rng.f());
        let (d0, d1) = (a.density(p), b.density(p));
        assert!((0.0..=1.0).contains(&d1) && d1 <= d0 + 1e-6, "erosion raised density at {p}: {d0} -> {d1}");
        assert!(d1 <= b.grid.max[b.grid.index(b.grid.cell_of(p))] + 1e-5);
        eroded += (d0 - d1 > 0.05) as u32;
        core += (d0 >= 1.0 && d1 >= 1.0) as u32;
    }
    assert!(eroded > 500, "erosion carves the edges ({eroded})");
    assert!(core > 100, "fully dense cores stay ({core})");
}

#[test]
fn softer_edges_spread_density_outward() {
    let at = |edge: f32| {
        let rs = build(&cloud(with_look(CloudLook { edge_softness: edge, erosion: 0.0, ..Default::default() })));
        let v = &rs.volumes.list[0];
        let Source::Puffs(c) = &v.src else { panic!("puffs") };
        let b = c.blobs[0];
        // Half-way out along the first puff: a harder edge is denser there.
        v.density(b.c + Vec3::X * b.r * 0.8)
    };
    assert!(at(0.1) > at(0.9), "{} vs {}", at(0.1), at(0.9));
}

#[test]
fn noise_is_bounded_deterministic_and_detail_sets_octaves() {
    let mut rng = Rng::new(8);
    for _ in 0..2000 {
        let q = Vec3::new(rng.f(), rng.f(), rng.f()) * 50.0 - 25.0;
        let n = noise(q, 3);
        assert!((0.0..=1.0).contains(&n) && n == noise(q, 3));
    }
    let mut rs = build(&cloud(CloudParams::default()));
    rs.volumes.set_detail(NoiseDetail::Ultra);
    let Source::Puffs(c) = &rs.volumes.list[0].src else { panic!("puffs") };
    assert_eq!(c.octaves, 4);
    // Old scenes without a look load as volume clouds.
    let p: CloudParams = serde_json::from_str(r#"{"kind": "Congestus", "size": 400.0}"#).unwrap();
    assert_eq!(p.look, CloudLook::default());
}
