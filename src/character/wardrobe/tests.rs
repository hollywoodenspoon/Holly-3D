use super::*;
use crate::character::makehuman::library::{eye_colors, item, items, skins, texture};
use crate::character::makehuman::obj::base;
use crate::character::params::{Hair, Outfit};
use crate::character::CharacterParams;
use std::collections::HashSet;

fn plain() -> CharacterParams {
    CharacterParams { hair: Hair::Bald, outfit: Outfit::None, ..Default::default() }
}

fn faces_with(m: &EditablePoly, id: u16) -> usize {
    m.polygons.values().filter(|p| p.material_id == id).count()
}

#[test]
fn the_bundled_pack_lists_every_system_asset() {
    let n = |c| items(c).len();
    assert_eq!(
        [n(Category::Eyebrows), n(Category::Eyelashes), n(Category::Teeth), n(Category::Tongue), n(Category::Hair), n(Category::Clothes), n(Category::Proxy)],
        [12, 4, 6, 1, 10, 20, 3]
    );
    assert_eq!(skins().len(), 23, "21 ethnic skins + toon01 + the special suits");
    assert_eq!(eye_colors().len(), 9);
    assert!(items(Category::Hair).contains(&"ponytail01"));
    assert_eq!(library::pretty("male_casualsuit01"), "Male Casualsuit 01");
}

#[test]
fn every_item_binds_each_vertex_to_the_base_mesh() {
    let nb = base().coords.len() as u32;
    for cat in Category::ALL {
        for name in items(cat) {
            let it = item(cat, name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(it.proxy.bindings.len(), it.mesh.coords.len(), "{name}: one binding per vertex");
            assert!(!it.mesh.faces.is_empty(), "{name}");
            assert!(it.proxy.bindings.iter().all(|b| b.v.iter().all(|&v| v < nb)), "{name}");
            assert!(it.proxy.scale.iter().all(Option::is_some), "{name}: x/y/z scale refs");
            let nv = it.mesh.coords.len() as u32;
            assert!(it.mesh.faces.iter().all(|f| f.verts().iter().all(|&v| v < nv)), "{name}");
        }
    }
}

#[test]
fn face_kit_adds_brows_lashes_teeth_and_tongue() {
    let bare = plain().build();
    let p = CharacterParams { mh: MhAssets::face_kit(), ..plain() };
    let m = p.build();
    assert!(m.validate().is_ok(), "{:?}", m.validate());
    let count = |c: Category, n: &str| item(c, n).unwrap().mesh.faces.len();
    for (id, c, n) in [(ids::EYEBROWS, Category::Eyebrows, "eyebrow001"), (ids::EYELASHES, Category::Eyelashes, "eyelashes01"), (ids::TEETH, Category::Teeth, "teeth_base"), (ids::TONGUE, Category::Tongue, "tongue01")] {
        assert_eq!(faces_with(&m, id), count(c, n), "{n}");
    }
    let extra: usize = MhAssets::face_kit().pieces().iter().map(|(c, n, _)| count(*c, n)).sum();
    assert_eq!(m.counts().2, bare.counts().2 + extra);
    // The brows sit on the face: above the eyes, inside the head.
    let eye_z = m.polygons.values().filter(|p| p.material_id == ids::EYES).flat_map(|p| p.corners.iter().map(|c| m.vertices[c.vertex_id].position.z)).fold(0.0, f32::max);
    let brow = m.polygons.values().filter(|p| p.material_id == ids::EYEBROWS).flat_map(|p| p.corners.iter().map(|c| m.vertices[c.vertex_id].position)).collect::<Vec<_>>();
    let top = m.bounds().unwrap().max.z;
    assert!(brow.iter().all(|b| b.z > eye_z - 0.02 && b.z < top), "brows above the eyes");
}

#[test]
fn proxies_follow_the_morphs() {
    let brow_top = |height: f32| {
        let mut p = CharacterParams { mh: MhAssets { eyebrows: Some("eyebrow001".into()), ..Default::default() }, ..plain() };
        p.macros.height = height;
        let m = p.build();
        m.polygons.values().filter(|q| q.material_id == ids::EYEBROWS).flat_map(|q| q.corners.iter().map(|c| m.vertices[c.vertex_id].position.z)).fold(0.0, f32::max)
    };
    assert!(brow_top(1.0) > brow_top(0.0) + 0.1);
}

#[test]
fn clothes_hide_the_body_they_cover_and_get_their_own_ids() {
    let bare = plain().build();
    let mut mh = MhAssets::default();
    mh.toggle_clothes("male_casualsuit01");
    mh.toggle_clothes("shoes01");
    let m = CharacterParams { mh: mh.clone(), ..plain() }.build();
    assert!(m.validate().is_ok());
    assert!(faces_with(&m, ids::SKIN) < faces_with(&bare, ids::SKIN) - 3000, "the suit hides the torso and legs");
    assert_eq!(faces_with(&m, ids::CLOTHES), item(Category::Clothes, "male_casualsuit01").unwrap().mesh.faces.len());
    assert!(faces_with(&m, ids::CLOTHES + 1) > 0, "shoes");
    mh.toggle_clothes("male_casualsuit01");
    assert_eq!(mh.clothes, ["shoes01"]);
}

#[test]
fn low_poly_topology_replaces_the_body() {
    let mh = MhAssets { topology: Some("proxy741".into()), ..Default::default() };
    let m = CharacterParams { mh, ..plain() }.build();
    assert!(m.validate().is_ok());
    assert_eq!(faces_with(&m, ids::SKIN), item(Category::Proxy, "proxy741").unwrap().mesh.faces.len());
    let b = m.bounds().unwrap();
    assert!((1.4..2.0).contains(&b.max.z), "still full height: {}", b.max.z);
}

#[test]
fn makehuman_hair_replaces_the_procedural_hair() {
    let p = CharacterParams { hair: Hair::Long, mh: MhAssets { hair: Some("ponytail01".into()), ..Default::default() }, ..plain() };
    let m = p.build();
    assert_eq!(faces_with(&m, ids::HAIR), 0);
    assert!(faces_with(&m, ids::MH_HAIR) > 500);
}

#[test]
fn textures_decode_with_their_alpha() {
    let brow = texture("eyebrows/eyebrow001/eyebrow001.png").unwrap();
    assert_eq!((brow.tex.width, brow.tex.height), (256, 256));
    assert!(brow.tex.data.iter().any(|p| p[3] < 10) && brow.tex.data.iter().any(|p| p[3] > 200));
    // The six teeth share one texture through a link record.
    let a = texture("teeth/teeth_base/teeth.png").unwrap();
    let b = texture("teeth/teeth_shape03/teeth.png").unwrap();
    assert!(std::sync::Arc::ptr_eq(&a.tex, &b.tex));
    let skin = item(Category::Hair, "afro01").unwrap().texture().unwrap();
    assert_eq!(skin.tex.width, 1024);
}

#[test]
fn materials_follow_the_assets_and_keep_user_edits() {
    use crate::materials::{MapChannel, MultiPreset};
    let mut ms = MultiPreset::Character.build();
    let mut mh = MhAssets::face_kit();
    mh.toggle_clothes("female_casualsuit01");
    (mh.skin, mh.eye_color) = (Some("young_asian_female".into()), Some("green".into()));
    materials::sync(&mh, None, &mut ms);
    let get = |ms: &crate::materials::multi::MultiSub, id: u16| ms.subs.iter().find(|s| s.id == id).cloned().unwrap();
    for id in [ids::EYEBROWS, ids::EYELASHES, ids::TEETH, ids::TONGUE, ids::CLOTHES, ids::SKIN, ids::EYES] {
        assert!(get(&ms, id).mat.active_map(MapChannel::BaseColor).is_some(), "ID {id} is textured");
    }
    assert_eq!(get(&ms, ids::SKIN).name, "Skin: Young Asian Female");
    assert!(get(&ms, ids::SKIN).mat.principled.sss_weight > 0.0, "skin keeps its SSS");
    let brows = get(&ms, ids::EYEBROWS).mat;
    assert!(!brows.alpha_cutout && brows.opacity < 1.0, "brows blend softly");
    assert!(get(&ms, ids::CLOTHES).mat.alpha_cutout || get(&ms, ids::CLOTHES).mat.opacity == 1.0, "clothes never haze");
    assert!(brows.roughness >= 0.6, "brows are not glossy");
    assert!(!get(&ms, ids::EYES).mat.alpha_cutout, "eyes have no holes");
    // A user edit survives a re-sync.
    ms.subs.iter_mut().find(|s| s.id == ids::CLOTHES).unwrap().mat.roughness = 0.123;
    materials::sync(&mh, None, &mut ms);
    assert_eq!(get(&ms, ids::CLOTHES).mat.roughness, 0.123);
    // Taking things off cleans up.
    let ids_before: HashSet<u16> = ms.subs.iter().map(|s| s.id).collect();
    assert!(ids_before.contains(&ids::CLOTHES));
    let bare = MhAssets::default();
    materials::sync(&bare, None, &mut ms);
    assert_eq!(ms.subs.len(), 6, "back to the six pack materials");
    assert_eq!(get(&ms, ids::SKIN).name, "Skin");
    assert!(get(&ms, ids::SKIN).mat.active_map(MapChannel::BaseColor).is_none());
}

#[test]
fn a_photo_skin_takes_id_1_and_a_rebake_keeps_user_edits() {
    use crate::materials::texture::{TexHandle, Texture};
    use crate::materials::{MapChannel, MapNode, MultiPreset};
    let mut ms = MultiPreset::Character.build();
    let mh = MhAssets { skin: Some("young_asian_female".into()), ..MhAssets::default() };
    let skin = |ms: &crate::materials::multi::MultiSub| ms.subs.iter().find(|s| s.id == ids::SKIN).cloned().unwrap();
    let tex_of = |m: &crate::materials::Material| match m.active_map(MapChannel::BaseColor).unwrap().0 {
        MapNode::Bitmap { tex: Some(t), .. } => t.clone(),
        _ => panic!("no bitmap"),
    };
    let a = TexHandle::new(Texture::new(4, 4, [200, 150, 120, 255]));
    materials::sync(&mh, Some(&a), &mut ms);
    assert_eq!(skin(&ms).name, materials::CLONED_SKIN);
    assert!(tex_of(&skin(&ms).mat) == a);
    ms.subs.iter_mut().find(|s| s.id == ids::SKIN).unwrap().mat.roughness = 0.321;
    let b = TexHandle::new(Texture::new(4, 4, [190, 140, 110, 255]));
    materials::sync(&mh, Some(&b), &mut ms);
    assert!(tex_of(&skin(&ms).mat) == b, "a re-bake swaps the bitmap");
    assert_eq!(skin(&ms).mat.roughness, 0.321, "and keeps the user's edit");
    // Switching the photo skin off goes back to the MakeHuman skin.
    materials::sync(&mh, None, &mut ms);
    assert_eq!(skin(&ms).name, "Skin: Young Asian Female");
}

#[test]
fn old_characters_load_without_assets() {
    let p = CharacterParams { mh: MhAssets::face_kit(), ..Default::default() };
    let s = ron::to_string(&p).unwrap();
    assert_eq!(ron::from_str::<CharacterParams>(&s).unwrap(), p);
    let old: CharacterParams = ron::from_str("(hair: Long)").unwrap();
    assert_eq!(old.mh, MhAssets::default());
}
