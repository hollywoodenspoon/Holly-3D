//! Builds a Character: morphs the hm08 base mesh (macros, micro sliders,
//! anime ratio), keeps the `body` faces (helpers and joint cubes are
//! masked away), fits the eye proxies, then adds hair, outfit and weapon
//! pieces fitted to the morphed landmarks. Feet stand on Z = 0.

use super::landmarks::Landmarks;
use super::makehuman::mhclo::Proxy;
use super::makehuman::obj::{base, to_scene, BaseMesh, Part};
use super::makehuman::target::pack;
use super::makehuman::{assets, modifiers};
use super::params::{ids, CharacterParams, Eyes, Outfit};
use super::{anime, outfit};
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};
use std::sync::OnceLock;

/// The morphed and posed hm08 coordinates (decimetres, Y-up) and the head sphere.
pub fn morph(p: &CharacterParams) -> (Vec<Vec3>, (Vec3, f32)) {
    let b = base();
    let mut c = b.coords.clone();
    p.macros.apply(pack(), &mut c);
    modifiers::apply(&anime::micro(p).micro, pack(), &mut c);
    let head = anime::head_sphere(&c, body_mask());
    anime::grow_head(&mut c, head, p.anime);
    let head = anime::head_sphere(&c, body_mask());
    if p.pose.is_rest() {
        return (c, head);
    }
    // The head sphere rides on the head bone (raised hands would fool a
    // fresh top-of-body search).
    let follow = super::rig::bone_matrix(&c, &p.pose, "head");
    super::rig::apply(&p.pose, &mut c);
    (c, (follow.transform_point3(head.0), head.1))
}

pub fn body_mask() -> &'static [bool] {
    static CELL: OnceLock<Vec<bool>> = OnceLock::new();
    CELL.get_or_init(|| base().vertex_mask(Part::Body))
}

/// Area-weighted vertex normals of the faces of `part`.
fn normals(b: &BaseMesh, coords: &[Vec3], group: &str) -> Vec<Vec3> {
    let mut n = vec![Vec3::ZERO; coords.len()];
    for f in b.faces.iter().filter(|f| b.groups[f.group as usize] == group) {
        let v = f.verts();
        let (a, bb, c) = (coords[v[0] as usize], coords[v[1] as usize], coords[v[2] as usize]);
        let mut fn_ = (bb - a).cross(c - a);
        if v.len() == 4 {
            fn_ += (c - a).cross(coords[v[3] as usize] - a);
        }
        for &i in v {
            n[i as usize] += fn_;
        }
    }
    n.iter().map(|v| v.normalize_or(Vec3::Y)).collect()
}

/// Shared hm08 -> scene placement (feet on Z = 0).
pub struct Place {
    pub lift: f32,
}

impl Place {
    pub fn at(&self, p: Vec3) -> Vec3 {
        to_scene(p) + Vec3::Z * self.lift
    }
}

/// Adds the faces of face group `group` (offset `push` dm along the normal).
#[allow(clippy::too_many_arguments)]
fn add_group(m: &mut EditablePoly, b: &BaseMesh, coords: &[Vec3], group: &str, mat: u16, push: f32, place: &Place, toon: Option<((Vec3, f32), f32)>) {
    let nrm = normals(b, coords, group);
    let mut map = vec![None; coords.len()];
    for f in b.faces.iter().filter(|f| b.groups[f.group as usize] == group) {
        let ids: Vec<_> = f
            .verts()
            .iter()
            .map(|&i| *map[i as usize].get_or_insert_with(|| m.add_vertex(place.at(coords[i as usize] + nrm[i as usize] * push))))
            .collect();
        let uvs: Vec<Vec2> = f.uvs().iter().map(|&t| b.uvs.get(t as usize).copied().unwrap_or(Vec2::ZERO)).collect();
        let pid = m.add_polygon_uv(&ids, &uvs);
        let poly = &mut m.polygons[pid];
        (poly.material_id, poly.smoothing_group) = (mat, 1);
        if let Some((head, a)) = toon {
            for (c, &i) in poly.corners.iter_mut().zip(f.verts()) {
                let n = anime::toon_normal(coords[i as usize], nrm[i as usize], head, a);
                c.explicit_normal = n.map(|n| to_scene(n).normalize_or(Vec3::Z));
            }
        }
    }
}

fn add_eyes(m: &mut EditablePoly, coords: &[Vec3], eyes: Eyes, place: &Place) {
    let (clo, obj) = match eyes {
        Eyes::None => return,
        Eyes::Low => (assets::eyes_low_mhclo(), assets::eyes_low_obj()),
        Eyes::High => (assets::eyes_high_mhclo(), assets::eyes_high_obj()),
    };
    let proxy = Proxy::parse(assets::text(clo));
    let mesh = BaseMesh::parse(assets::text(obj));
    let fitted = proxy.fit(coords);
    let ids: Vec<_> = fitted.iter().map(|p| m.add_vertex(place.at(*p))).collect();
    for f in &mesh.faces {
        let v: Vec<_> = f.verts().iter().filter_map(|&i| ids.get(i as usize).copied()).collect();
        if v.len() != f.verts().len() {
            continue;
        }
        let uvs: Vec<Vec2> = f.uvs().iter().map(|&t| mesh.uvs.get(t as usize).copied().unwrap_or(Vec2::ZERO)).collect();
        let pid = m.add_polygon_uv(&v, &uvs);
        (m.polygons[pid].material_id, m.polygons[pid].smoothing_group) = (ids::EYES, 1);
    }
}

/// The full humanoid character mesh.
pub fn build_human(p: &CharacterParams) -> EditablePoly {
    let b = base();
    let (coords, head) = morph(p);
    let min_y = coords.iter().zip(body_mask()).filter(|(_, m)| **m).map(|(c, _)| c.y).fold(f32::MAX, f32::min);
    let max_y = coords.iter().zip(body_mask()).filter(|(_, m)| **m).map(|(c, _)| c.y).fold(f32::MIN, f32::max);
    let place = Place { lift: -min_y * 0.1 };
    let mut m = EditablePoly::new();
    let toon = (p.anime > 0.0).then_some((head, p.anime));
    add_group(&mut m, b, &coords, "body", ids::SKIN, 0.0, &place, toon);
    if p.outfit != Outfit::None {
        add_group(&mut m, b, &coords, "helper-tights", outfit::tights_material(p.outfit), 0.04, &place, None);
    }
    if outfit::wants_skirt(p.outfit) {
        add_group(&mut m, b, &coords, "helper-skirt", ids::CLOTH, 0.06, &place, None);
    }
    add_eyes(&mut m, &coords, p.eyes, &place);
    let lm = Landmarks::of(&coords, head, (max_y - min_y) * 0.1, |q| place.at(q));
    let mut kit = crate::generators::kit::Kit::new();
    super::hair::build(&mut kit, p, &lm);
    outfit::build(&mut kit, p, &lm);
    kit.m.finalize();
    m.append(&kit.m, bevy::math::Mat4::IDENTITY);
    m.finalize();
    m
}
