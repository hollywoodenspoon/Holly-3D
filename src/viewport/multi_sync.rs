//! Viewport display of Multi/Sub-Object materials: an object whose shown
//! material is a Multi/Sub-Object is drawn as one part per sub-material,
//! each holding the triangles of the polygons whose Material ID resolves to
//! it (3ds Max wrap-around rules; switched-off IDs use the default gray).

use super::mat_sync::{MatCache, MatTextures, standard};
use super::render_sync::{LAYER_MESH, build_mesh};
use crate::editor::Editor;
use crate::materials::{Material, MultiSub};
use crate::mesh::EditablePoly;
use crate::mesh::triangulate::TriMesh;
use crate::scene::ObjId;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashMap;

/// Triangle indices grouped by sub-material: `(Some(list index), indices)`
/// for enabled sub-materials, `None` for switched-off ones. Empty groups are
/// dropped; groups come out in list order (off last).
pub fn split_by_id(tri: &TriMesh, mesh: &EditablePoly, ms: &MultiSub) -> Vec<(Option<usize>, Vec<u32>)> {
    let mut groups: Vec<Vec<u32>> = vec![vec![]; ms.subs.len() + 1];
    for (t, poly) in tri.tri_poly.iter().enumerate() {
        let id = mesh.polygons.get(*poly).map_or(1, |p| p.material_id);
        let slot = match ms.index_for(id) {
            Some(i) if ms.subs[i].enabled => i,
            _ => ms.subs.len(),
        };
        groups[slot].extend_from_slice(&tri.indices[t * 3..t * 3 + 3]);
    }
    let off = ms.subs.len();
    groups.into_iter().enumerate().filter(|(_, g)| !g.is_empty()).map(|(i, g)| ((i != off).then_some(i), g)).collect()
}

struct Parts {
    /// (mesh version, showing the end result, material revision, x-ray,
    /// object scale when a sub-material is World / Triplanar mapped).
    key: (u64, bool, u64, bool, (u32, u32, u32)),
    entities: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct MultiParts {
    objs: HashMap<ObjId, Parts>,
}

/// Spawns / refreshes / removes per-ID parts (runs after `sync_meshes`).
#[allow(clippy::too_many_arguments)]
pub fn sync_multi_parts(
    mut commands: Commands,
    editor: Res<Editor>,
    cache: Res<MatCache>,
    mut parts: ResMut<MultiParts>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tfs: Query<(&mut Transform, &mut Visibility)>,
) {
    let scene = &editor.scene;
    let mut keep = vec![];
    for (id, o) in scene.objects.iter().filter(|(_, o)| !o.is_helper()) {
        let Some((rev, m, _)) = o.material.and_then(|m| cache.get(m)).filter(|(_, m, _)| m.show_in_viewport && !o.checker) else {
            continue;
        };
        // A painted Multi/Sub-Object is drawn whole, with its layer composite.
        let Some(ms) = m.multi.as_ref().filter(|_| m.paint.is_none()) else { continue };
        keep.push(id);
        let shown = editor.display_mesh(id);
        let scale = super::triplanar_uv::scale_of(o);
        let scale_key = if ms.subs.iter().any(|s| s.mat.projection.on()) { (scale.x.to_bits(), scale.y.to_bits(), scale.z.to_bits()) } else { (0, 0, 0) };
        let key = (o.version, std::ptr::eq(shown, &*o.evaluated_mesh), rev, o.see_through, scale_key);
        let p = parts.objs.entry(id).or_insert(Parts { key: (u64::MAX, false, 0, false, (0, 0, 0)), entities: vec![] });
        if p.key != key {
            p.key = key;
            p.entities.drain(..).for_each(|e| commands.entity(e).despawn());
            let tri = shown.to_tri_mesh();
            let off = Material::named("Default");
            let none = MatTextures::default();
            for (sub, idx) in split_by_id(&tri, shown, ms) {
                let (sm, tex) = match sub {
                    Some(i) => (&ms.subs[i].mat, o.material.and_then(|mid| cache.sub_textures(mid, i)).unwrap_or(&none)),
                    None => (&off, &none),
                };
                let mut part = tri.clone();
                part.indices = idx;
                if sm.projection.on() {
                    super::triplanar_uv::apply(&mut part, &sm.projection, scale);
                }
                let e = commands
                    .spawn((
                        Mesh3d(meshes.add(build_mesh(&part))),
                        MeshMaterial3d(materials.add(standard(sm, tex, o.see_through))),
                        Transform::from_matrix(o.world_matrix()),
                        RenderLayers::layer(LAYER_MESH),
                    ))
                    .id();
                p.entities.push(e);
            }
        }
        let world = Transform::from_matrix(o.world_matrix());
        let vis = if o.hidden { Visibility::Hidden } else { Visibility::Inherited };
        for e in &p.entities {
            if let Ok((mut tf, mut v)) = tfs.get_mut(*e) {
                if *tf != world {
                    *tf = world;
                }
                if *v != vis {
                    *v = vis;
                }
            }
        }
    }
    parts.objs.retain(|id, p| {
        let alive = keep.contains(id);
        if !alive {
            p.entities.drain(..).for_each(|e| commands.entity(e).despawn());
        }
        alive
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};

    #[test]
    fn triangles_split_by_material_id_with_wrap_and_off() {
        let mut mesh = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
        let ids: Vec<_> = mesh.polygons.keys().collect();
        assert_eq!(ids.len(), 6);
        // IDs 1, 2, 3, 4 (wraps to 1 of 3), 5 (wraps to 2), 3.
        for (p, id) in ids.iter().zip([1u16, 2, 3, 4, 5, 3]) {
            mesh.polygons[*p].material_id = id;
        }
        let mut ms = MultiSub::new(3);
        let tri = mesh.to_tri_mesh();
        let g = split_by_id(&tri, &mesh, &ms);
        let count = |g: &[(Option<usize>, Vec<u32>)], k: Option<usize>| g.iter().find(|x| x.0 == k).map_or(0, |x| x.1.len() / 3);
        assert_eq!((count(&g, Some(0)), count(&g, Some(1)), count(&g, Some(2))), (4, 4, 4), "2 quads (4 tris) each");
        assert_eq!(g.iter().map(|x| x.1.len()).sum::<usize>(), tri.indices.len(), "every triangle drawn once");
        ms.subs[2].enabled = false;
        let g = split_by_id(&tri, &mesh, &ms);
        assert_eq!((count(&g, Some(2)), count(&g, None)), (0, 4), "Off draws the default material");
    }
}
