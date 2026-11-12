//! Round trips for the scene file and every exchange format.

use super::*;
use crate::mesh::EditablePoly;
use crate::modifiers::{ModifierEntry, make_modifier};
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::{BaseObject, Scene};
use bevy::math::{Quat, Vec2, Vec4};

/// Box with mixed smoothing groups (incl. a multi-group face), two material
/// IDs, coloured vertices and distinct UVs.
fn sample_box() -> EditablePoly {
    let mut m = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    for (i, p) in m.polygons.values_mut().enumerate() {
        p.material_id = 1 + (i % 2) as u16;
        p.smoothing_group = match i {
            0 => 0,
            1 => 0b101,
            _ => 1 << i,
        };
        for (k, c) in p.corners.iter_mut().enumerate() {
            c.uv = Vec2::new(0.1 * k as f32, 0.05 * i as f32);
        }
    }
    for (i, v) in m.vertices.values_mut().enumerate() {
        v.color = Vec4::new(i as f32 / 8.0, 0.5, 1.0 - i as f32 / 8.0, 1.0);
    }
    m
}

fn item() -> MeshItem {
    MeshItem {
        name: "Box001".into(),
        mesh: sample_box(),
        transform: Mat4::from_scale_rotation_translation(Vec3::new(1.0, 2.0, 1.0), Quat::from_rotation_z(0.7), Vec3::new(3.0, -1.0, 2.0)),
        materials: vec![
            MatInfo { name: "Red".into(), base_color: [0.9, 0.1, 0.1], ..Default::default() },
            MatInfo { name: "Steel".into(), metallic: 0.9, roughness: 0.25, ..Default::default() },
        ],
    }
}

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ilex-io-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

type Signature = (Vec<(u32, u16, usize)>, Vec<[i32; 3]>);

/// Sorted (sg, mat, corner count) per polygon plus vertex colours.
fn signature(m: &EditablePoly) -> Signature {
    let mut f: Vec<_> = m.polygons.values().map(|p| (p.smoothing_group, p.material_id, p.corners.len())).collect();
    f.sort();
    let mut c: Vec<[i32; 3]> = m.vertices.values().map(|v| (v.color.truncate() * 1000.0).round().as_ivec3().to_array()).collect();
    c.sort();
    (f, c)
}

fn same_world(a: &MeshItem, b: &MeshItem) {
    let (wa, wb) = (a.world_mesh(), b.world_mesh());
    assert_eq!(wb.counts(), wa.counts(), "counts");
    assert!(wb.validate().is_ok());
    let ba = crate::mesh::Aabb::from_points(wa.vertices.values().map(|v| v.position)).unwrap();
    let bb = crate::mesh::Aabb::from_points(wb.vertices.values().map(|v| v.position)).unwrap();
    assert!((ba.min - bb.min).length() < 1e-3 && (ba.max - bb.max).length() < 1e-3, "{ba:?} vs {bb:?}");
    let mut ua: Vec<[i32; 2]> =
        wa.polygons.values().flat_map(|p| p.corners.iter().map(|c| (c.uv * 1000.0).round().as_ivec2().to_array())).collect();
    let mut ub: Vec<[i32; 2]> =
        wb.polygons.values().flat_map(|p| p.corners.iter().map(|c| (c.uv * 1000.0).round().as_ivec2().to_array())).collect();
    ua.sort();
    ub.sort();
    assert_eq!(ua, ub, "uvs");
}

#[test]
fn obj_round_trip_keeps_ngons_groups_colors_and_ids() {
    let it = item();
    let (obj, mtl) = obj::write_strings(std::slice::from_ref(&it), "x.mtl");
    let back = obj::parse(&obj, &obj::parse_mtl(&mtl));
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].mesh.counts(), (8, 12, 6));
    same_world(&it, &back[0]);
    assert_eq!(signature(&back[0].mesh), signature(&it.world_mesh()));
    assert_eq!(back[0].materials[1].metallic, 0.9);
}

#[test]
fn gltf_and_glb_round_trip_rebuild_quads() {
    for ext in ["gltf", "glb"] {
        let p = tmp(&format!("box.{ext}"));
        export(&p, &[item()]).unwrap();
        let back = import(&p).unwrap();
        assert_eq!(back.len(), 1, "{ext}");
        assert_eq!(back[0].mesh.counts(), (8, 12, 6), "{ext}: quads rebuilt");
        same_world(&item(), &back[0]);
        assert_eq!(signature(&back[0].mesh), signature(&item().mesh));
        assert_eq!(back[0].materials[0].base_color, [0.9, 0.1, 0.1]);
        let t = back[0].transform.to_scale_rotation_translation().2;
        assert!((t - Vec3::new(3.0, -1.0, 2.0)).length() < 1e-4, "{ext}: pivot kept");
    }
}

#[test]
fn fbx_round_trip_through_ufbx() {
    let p = tmp("box.fbx");
    export(&p, &[item()]).unwrap();
    let back = import(&p).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].name, "Box001");
    assert_eq!(back[0].mesh.counts(), (8, 12, 6));
    same_world(&item(), &back[0]);
    assert_eq!(signature(&back[0].mesh), signature(&item().mesh), "3ds Max smoothing masks kept");
    assert_eq!(back[0].materials.len(), 2);
    assert!((back[0].materials[1].base_color[0] - 0.8).abs() < 1e-4);
}

#[test]
fn scene_file_round_trip_restores_stack_and_materials() {
    let mut scene = Scene::default();
    let a = scene.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Sphere)), Vec3::new(1.0, 2.0, 3.0));
    let md: ModifierEntry = make_modifier("TurboSmooth").unwrap();
    scene.objects[a].add_modifier(md);
    scene.objects[a].material = Some(scene.materials.slot(2));
    let b = scene.add(BaseObject::EditablePoly(sample_box().into()), Vec3::ZERO);
    let light = crate::render::Light::new(crate::render::LightKind::Omni);
    let l = scene.add_light(light, None, Vec3::new(5.0, 5.0, 5.0), Vec3::ZERO);
    let text = scene_file::to_string(&scene).unwrap();
    let back = scene_file::from_str(&text).unwrap();
    assert_eq!(back.objects.len(), scene.objects.len());
    let (oa, ob) = (&back.objects[a], &back.objects[b]);
    assert_eq!(oa.modifiers.len(), 1);
    assert_eq!(oa.evaluated_mesh.counts(), scene.objects[a].evaluated_mesh.counts(), "stack re-evaluated");
    assert_eq!(oa.transform.translation, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(back.materials.mats[oa.material.unwrap()].name, scene.materials.mats[scene.objects[a].material.unwrap()].name);
    assert_eq!(signature(&ob.evaluated_mesh), signature(&sample_box()));
    assert!(back.objects[l].light().is_some());
    // names keep counting where the saved scene left off
    let mut back = back;
    assert_eq!(back.unique_name("Sphere"), "Sphere002");
}

#[test]
fn painted_texture_survives_scene_file() {
    let mut scene = Scene::default();
    let id = scene.materials.slot(0);
    let mut t = crate::materials::texture::Texture::new(4, 4, [255, 255, 255, 255]);
    t.data[5] = [12, 34, 56, 255];
    let tex = crate::materials::texture::TexHandle { tex: std::sync::Arc::new(t.clone()), rev: 1 };
    scene.materials.mats[id].maps[0].node = Some(crate::materials::maps::MapNode::Bitmap { path: String::new(), tex: Some(tex) });
    let back = scene_file::from_str(&scene_file::to_string(&scene).unwrap()).unwrap();
    let Some(crate::materials::maps::MapNode::Bitmap { tex: Some(t), .. }) = &back.materials.mats[id].maps[0].node else {
        panic!("bitmap lost")
    };
    assert_eq!(t.tex.data[5], [12, 34, 56, 255]);
}
