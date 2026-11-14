//! Clouds V2b: the viewport preview bake matches the render density.

use super::bake::{bake, dims_for, preview_volume};
use super::*;
use crate::generators::clouds::{CloudLook, CloudParams, RenderAs};
use crate::primitives::Primitive;

fn cloud_scene(p: CloudParams) -> (Scene, crate::scene::ObjId) {
    let mut s = Scene::default();
    let id = s.add(BaseObject::World(WorldGen::Cloud(Box::new(p))), Vec3::ZERO);
    s.objects[id].evaluate();
    (s, id)
}

#[test]
fn dims_follow_the_box_shape_and_budget() {
    let d = dims_for(Vec3::new(400.0, 200.0, 100.0), 64_000);
    assert!(d[0] > d[1] && d[1] > d[2], "{d:?}");
    let n = d.iter().product::<u32>();
    assert!((40_000..=90_000).contains(&n), "{n}");
    assert_eq!(dims_for(Vec3::new(1e5, 1.0, 1.0), 1000), [192, 4, 4]);
}

#[test]
fn bake_samples_the_render_density() {
    let (s, id) = cloud_scene(CloudParams { detail: 2, ..Default::default() });
    let v = preview_volume(&s, &s.objects[id]).expect("a cloud previews as a volume");
    let mat = v.mat.clone();
    let reference = preview_volume(&s, &s.objects[id]).unwrap();
    let b = bake(v, 20_000);
    let [w, h, d] = b.dims;
    assert_eq!(b.data.len() as u32, w * h * d);
    assert_eq!(b.mat, mat);
    assert!(b.data.iter().any(|&x| x > 200), "a dense core");
    assert!(b.data.iter().filter(|&&x| x == 0).count() > b.data.len() / 10, "empty space around the puffs");
    // Voxel centres hold the renderer's density (Low noise detail).
    let mut r = reference;
    if let density::Source::Puffs(c) = &mut r.src {
        c.octaves = NoiseDetail::Low.octaves();
    }
    let cell = (b.hi - b.lo) / Vec3::new(w as f32, h as f32, d as f32);
    assert_eq!(b.voxel(0, h / 2, d / 2), 0, "the shell fades the box edge");
    assert_eq!(b.voxel(w / 2, h / 2, d - 1), 0);
    // The padded box keeps the puffs clear of every face.
    let face_max = |f: &dyn Fn(u32, u32) -> u8, n: u32, m: u32| (0..n).flat_map(|i| (0..m).map(move |j| (i, j))).map(|(i, j)| f(i, j)).max().unwrap();
    assert!(face_max(&|y, z| b.voxel(1, y, z), h, d) < 40);
    assert!(face_max(&|y, z| b.voxel(w - 2, y, z), h, d) < 40);
    assert!(face_max(&|x, y| b.voxel(x, y, d - 2), w, h) < 40, "the crown clears the top");
    for (x, y, z) in [(w / 2, h / 2, d / 2), (w / 3, h / 2, d / 4), (1, 1, 1)] {
        let p = b.lo + (Vec3::new(x as f32, y as f32, z as f32) + 0.5) * cell;
        let want = (r.density(p) * 255.0).round() as u8;
        assert_eq!(b.voxel(x, y, z), want);
    }
}

#[test]
fn only_volume_clouds_get_a_preview() {
    let (s, id) = cloud_scene(CloudParams { look: CloudLook { render_as: RenderAs::Mesh, ..Default::default() }, ..Default::default() });
    assert!(preview_volume(&s, &s.objects[id]).is_none());
    let mut s = Scene::default();
    let b = s.add(BaseObject::Primitive(Primitive::Box { length: 2.0, width: 2.0, height: 2.0, length_segs: 1, width_segs: 1, height_segs: 1 }), Vec3::ZERO);
    s.objects[b].evaluate();
    assert!(preview_volume(&s, &s.objects[b]).is_none());
}

