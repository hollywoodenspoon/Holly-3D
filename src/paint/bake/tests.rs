use super::*;
use crate::mesh::EditablePoly;
use crate::modifiers::smart_uv::smart_uv_project;
use crate::primitives::{Primitive, PrimitiveKind};

fn cfg() -> BakeSettings {
    BakeSettings { size: 128, ao_rays: 16, edge_radius: 0.08 }
}

/// Smart UV Project: non-overlapping UVs.
fn unwrap(m: &mut EditablePoly) {
    let (uvs, _) = smart_uv_project(m, 66.0, 0.02, true).expect("unwraps");
    for (p, uv) in m.polygons.values_mut().zip(uvs) {
        p.corners.iter_mut().zip(uv).for_each(|(c, uv)| c.uv = uv);
    }
}

fn unwrapped_box() -> EditablePoly {
    let mut m = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    unwrap(&mut m);
    m
}

fn mean(maps: &MeshMaps, v: &[f32], keep: impl Fn(usize) -> bool) -> f32 {
    let picked: Vec<f32> = (0..v.len()).filter(|&i| maps.valid[i] && keep(i)).map(|i| v[i]).collect();
    picked.iter().sum::<f32>() / picked.len().max(1) as f32
}

#[test]
fn box_maps_have_convex_edges_open_faces_and_axis_normals() {
    let m = unwrapped_box();
    let (v, e, p) = m.counts();
    let maps = MeshMaps::bake(&m, Mat4::IDENTITY, &cfg());
    assert_eq!(m.counts(), (v, e, p), "baking reads the mesh only");
    let n = (maps.size * maps.size) as usize;
    assert_eq!((maps.curvature.len(), maps.ao.len(), maps.normal.len(), maps.height.len(), maps.pos.len()), (n, n, n, n, n));
    assert!(maps.coverage() > 0.3, "the six faces fill a good part of the UV square: {}", maps.coverage());
    // Every value in range; normals are unit and (on a box) axis aligned.
    for i in (0..n).filter(|&i| maps.valid[i]) {
        assert!((0.0..=1.0).contains(&maps.curvature[i]) && (0.0..=1.0).contains(&maps.ao[i]) && (0.0..=1.0).contains(&maps.height[i]));
        assert!((maps.normal[i].length() - 1.0).abs() < 1e-3);
    }
    // A convex box: edges read above 0.5, face centres are flat, nothing is concave.
    let max = maps.curvature.iter().cloned().fold(0.0, f32::max);
    let min = (0..n).filter(|&i| maps.valid[i]).map(|i| maps.curvature[i]).fold(1.0, f32::min);
    assert!(max > 0.9 && min >= 0.5 - 1e-4, "edges {max}, least {min}");
    let flat = (0..n).filter(|&i| maps.valid[i] && (maps.curvature[i] - 0.5).abs() < 1e-4).count();
    assert!(flat > 500, "{flat} flat texels in the middle of faces");
    // Nothing shadows a convex box.
    assert!(mean(&maps, &maps.ao, |_| true) > 0.97);
    // Height follows world Z: the top face is 1, the bottom face 0.
    let top = mean(&maps, &maps.height, |i| maps.normal[i].z > 0.9);
    let bottom = mean(&maps, &maps.height, |i| maps.normal[i].z < -0.9);
    assert!(top > 0.95 && bottom < 0.05, "top {top}, bottom {bottom}");
}

#[test]
fn a_box_on_a_floor_bakes_cavity_occlusion_and_a_concave_crease() {
    // A box standing on a wide plane, attached into one mesh.
    let mut m = Primitive::default_of(PrimitiveKind::Plane).to_editable_poly();
    for v in m.vertices.values_mut() {
        v.position *= 4.0;
    }
    let lowest = Primitive::default_of(PrimitiveKind::Box).to_editable_poly().bounds().unwrap().min.z;
    let mut b = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    for v in b.vertices.values_mut() {
        v.position.z -= lowest;
    }
    m.append(&b, Mat4::IDENTITY);
    unwrap(&mut m);
    let maps = MeshMaps::bake(&m, Mat4::IDENTITY, &BakeSettings { size: 256, ao_rays: 32, edge_radius: 0.08 });
    // The floor right next to the box sees less sky than the open floor.
    let floor = |i: usize| maps.normal[i].z > 0.9 && maps.pos[i].z.abs() < 1e-3;
    let d = |i: usize| maps.pos[i].truncate().abs().max_element();
    let near = mean(&maps, &maps.ao, |i| floor(i) && d(i) > 1.0 && d(i) < 1.2);
    let far = mean(&maps, &maps.ao, |i| floor(i) && d(i) > 3.0);
    assert!(near < far - 0.1, "near the wall {near}, open floor {far}");
}

#[test]
fn smart_masks_follow_the_shape() {
    let m = unwrapped_box();
    let maps = MeshMaps::bake(&m, Mat4::IDENTITY, &cfg());
    let n = maps.size as usize;
    let level = |mask: &crate::paint::texel::Tiles<u8>, keep: &dyn Fn(usize) -> bool| {
        let v: Vec<f32> = (0..n * n).filter(|&i| maps.valid[i] && keep(i)).map(|i| mask.get((i % n) as i64, (i / n) as i64) as f32 / 255.0).collect();
        v.iter().sum::<f32>() / v.len().max(1) as f32
    };
    let plain = |kind| SmartMask { breakup: 0.0, ..SmartMask::new(kind) };
    // Edge wear: on the edges, not in the middle of faces.
    let wear = plain(SmartKind::EdgeWear).build(&maps);
    assert_eq!(wear.size(), maps.size);
    assert!(level(&wear, &|i| maps.curvature[i] > 0.9) > 0.9 && level(&wear, &|i| maps.curvature[i] < 0.51) < 0.05);
    // Top-down dust: on the top face only.
    let dust = plain(SmartKind::TopDown).build(&maps);
    assert!(level(&dust, &|i| maps.normal[i].z > 0.9) > 0.9 && level(&dust, &|i| maps.normal[i].z < 0.1) < 0.05);
    // Bottom soot: strongest at the foot of the walls, gone at the top.
    let soot = plain(SmartKind::BottomUp).build(&maps);
    let wall = |i: usize| maps.normal[i].z.abs() < 0.1;
    assert!(level(&soot, &|i| wall(i) && maps.height[i] < 0.15) > level(&soot, &|i| wall(i) && maps.height[i] > 0.85) + 0.5);
    // Cavity dirt finds nothing on a convex box; more Amount spreads any mask.
    assert!(level(&plain(SmartKind::CavityDirt).build(&maps), &|_| true) < 0.05);
    let more = SmartMask { amount: 0.9, ..plain(SmartKind::EdgeWear) }.build(&maps);
    assert!(level(&more, &|_| true) > level(&wear, &|_| true));
    // Breakup noise changes the mask, and the seed changes the noise.
    let noisy = |seed| SmartMask { seed, ..SmartMask::new(SmartKind::TopDown) }.build(&maps);
    let differ = (0..n * n).filter(|&i| noisy(1).get((i % n) as i64, (i / n) as i64) != noisy(2).get((i % n) as i64, (i / n) as i64)).count();
    assert!(differ > 50, "{differ} texels differ between seeds");
    // Every kind offers ready-made looks with colour, roughness and height.
    for k in SmartKind::ALL {
        assert!(!k.looks().is_empty() && k.looks().iter().all(|(n, f)| !n.is_empty() && f.channels[0]));
    }
    assert_eq!(SmartMask { look: 1, ..SmartMask::new(SmartKind::CavityDirt) }.look_name(), "Rust");
}

#[test]
fn geometry_maps_record_material_ids_and_projection_space() {
    let mut m = unwrapped_box();
    let ids: Vec<_> = m.polygons.keys().collect();
    for (k, p) in ids.iter().enumerate() {
        m.polygons[*p].material_id = k as u16 + 1;
    }
    let tris = crate::paint::paint_tris(&m, Mat4::IDENTITY, false);
    let plain = GeoMap::build(&tris, 64, false);
    assert!(plain.pos.is_empty() && plain.space(0).is_none());
    let mut seen: Vec<u16> = plain.ids.iter().copied().filter(|i| *i > 0).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen, vec![1, 2, 3, 4, 5, 6], "every face's ID lands in the map");
    let space = GeoMap::build(&tris, 64, true);
    let i = space.ids.iter().position(|i| *i > 0).unwrap();
    let (p, n) = space.space(i).unwrap();
    assert!((n.length() - 1.0).abs() < 1e-3 && n.abs().max_element() > 0.99, "a box face normal: {n}");
    assert!(m.bounds().unwrap().size().max_element() >= p.abs().max_element());
    // Images for export.
    let maps = MeshMaps::bake(&m, Mat4::IDENTITY, &cfg());
    let names: Vec<_> = maps.images().iter().map(|(n, t)| (*n, t.width)).collect();
    assert_eq!(names, [("curvature", 128), ("ao", 128), ("world_normal", 128), ("height", 128)]);
}
