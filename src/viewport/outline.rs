//! Universal ink outline in the viewport: an inverted-hull mesh (back faces of
//! a normal-pushed copy) plus sharp-angle crease lines drawn as gizmos.

use super::render_sync::LAYER_MESH;
use crate::editor::Editor;
use crate::mesh::EditablePoly;
use crate::mesh::adjacency::Adjacency;
use crate::mesh::triangulate::TriMesh;
use crate::scene::ObjId;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use std::collections::HashMap;

/// World size of one pixel at `p` in the active viewport, so Outline Width
/// reads as (roughly) pixels. The hull is rebuilt when this changes by >12%.
fn world_per_px(ed: &Editor, p: Vec3) -> f32 {
    let v = ed.viewports.active_view();
    let h = v.rect.size().y.max(1.0);
    let c = &v.camera;
    if c.is_ortho() {
        c.ortho_height / h
    } else {
        let eye = c.world_from_view().w_axis.truncate();
        eye.distance(p) * 2.0 * (c.fov * 0.5).tan() / h
    }
}

/// Positions pushed out along smooth (position-welded) normals, so the hull
/// stays closed across hard edges and UV seams.
pub fn hull_positions(t: &TriMesh, offset: f32) -> Vec<[f32; 3]> {
    let key = |p: &[f32; 3]| p.map(|c| (c * 1e4).round() as i64);
    let mut acc: HashMap<[i64; 3], Vec3> = HashMap::new();
    for tri in t.indices.as_chunks::<3>().0 {
        let [a, b, c] = tri.map(|i| Vec3::from_array(t.positions[i as usize]));
        let n = (b - a).cross(c - a); // area-weighted
        for &i in tri.iter() {
            *acc.entry(key(&t.positions[i as usize])).or_default() += n;
        }
    }
    t.positions.iter().map(|p| (Vec3::from_array(*p) + acc[&key(p)].normalize_or_zero() * offset).to_array()).collect()
}

/// Edges whose dihedral angle exceeds `angle_deg`, plus open borders (mesh space).
pub fn crease_edges(m: &EditablePoly, angle_deg: f32) -> Vec<(Vec3, Vec3)> {
    let adj = Adjacency::build(m);
    let cos_limit = angle_deg.to_radians().cos();
    m.edges
        .values()
        .filter(|e| match adj.faces_of_edge(e.v0, e.v1) {
            [a, b] => m.poly_normal(*a).dot(m.poly_normal(*b)) < cos_limit,
            [_] => true,
            _ => false,
        })
        .map(|e| (m.pos(e.v0), m.pos(e.v1)))
        .collect()
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct InkLines;

struct Hull {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    key: (u64, u32, [u32; 3], i32),
}

/// Crease lines cached per (mesh version, angle bits).
type Creases = ((u64, u32), Vec<(Vec3, Vec3)>);

#[derive(Resource, Default)]
pub struct OutlineMap {
    hulls: HashMap<ObjId, Hull>,
    creases: HashMap<ObjId, Creases>,
}

pub fn register(app: &mut App) {
    app.init_resource::<OutlineMap>().init_gizmo_group::<InkLines>().add_systems(Startup, configure);
}

fn configure(mut store: ResMut<GizmoConfigStore>) {
    let (cfg, _) = store.config_mut::<InkLines>();
    cfg.render_layers = RenderLayers::layer(LAYER_MESH);
    cfg.line.width = 2.0;
    cfg.depth_bias = -0.0004;
}

fn outline_of<'a>(ed: &'a Editor, o: &crate::scene::SceneObject) -> Option<&'a crate::materials::OutlineParams> {
    let m = ed.scene.materials.mats.get(o.material?)?;
    (m.show_in_viewport && m.outline.enabled && !o.hidden).then_some(&m.outline)
}

fn quant(c: [f32; 3]) -> [u32; 3] {
    c.map(|x| (x.clamp(0.0, 1.0) * 255.0) as u32)
}

/// Keeps one hull entity per outlined object and draws crease lines.
pub fn sync_outlines(
    mut commands: Commands,
    editor: Res<Editor>,
    mut map: ResMut<OutlineMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tfs: Query<&mut Transform>,
    mut ink: Gizmos<InkLines>,
) {
    let map = &mut *map;
    let wanted: HashMap<ObjId, crate::materials::OutlineParams> =
        editor.scene.objects.iter().filter_map(|(k, o)| outline_of(&editor, o).map(|p| (k, p.clone()))).collect();
    map.hulls.retain(|k, h| {
        let keep = wanted.get(k).is_some_and(|p| p.width > 0.0);
        if !keep {
            commands.entity(h.entity).despawn();
        }
        keep
    });
    map.creases.retain(|k, _| wanted.get(k).is_some_and(|p| p.crease_lines));
    for (id, p) in &wanted {
        let o = &editor.scene.objects[*id];
        let world = o.world_matrix();
        if p.width > 0.0 {
            // Hull offset in mesh space (world pixels / object scale).
            let center = o.evaluated_mesh.bounds().map_or(Vec3::ZERO, |b| world.transform_point3(b.center()));
            let scale = o.transform.scale.abs().max_element().max(1e-4);
            let offset = p.width * world_per_px(&editor, center) / scale;
            let key = (o.version, p.width.to_bits(), quant(p.color), (offset.max(1e-6).ln() * 8.0).round() as i32);
            let h = map.hulls.entry(*id).or_insert_with(|| {
                let mesh = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default()));
                let material = materials.add(StandardMaterial::default());
                let entity = commands
                    .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::default(), RenderLayers::layer(LAYER_MESH)))
                    .id();
                Hull { entity, mesh, material, key: (u64::MAX, 0, [0; 3], 0) }
            });
            if h.key != key {
                h.key = key;
                let t = o.evaluated_mesh.to_tri_mesh();
                let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
                let n = t.positions.len();
                m.insert_attribute(Mesh::ATTRIBUTE_POSITION, hull_positions(&t, offset));
                m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
                m.insert_indices(Indices::U32(t.indices.clone()));
                if let Some(mut dst) = meshes.get_mut(&h.mesh) {
                    *dst = m;
                }
                if let Some(mut mat) = materials.get_mut(&h.material) {
                    *mat = StandardMaterial {
                        base_color: Color::srgb(p.color[0], p.color[1], p.color[2]),
                        unlit: true,
                        cull_mode: Some(Face::Front),
                        ..default()
                    };
                }
            }
            if let Ok(mut tf) = tfs.get_mut(h.entity) {
                let w = Transform::from_matrix(world);
                if *tf != w {
                    *tf = w;
                }
            }
        }
        if p.crease_lines {
            let ckey = (o.version, p.crease_angle.to_bits());
            let entry = map.creases.entry(*id).or_insert_with(|| ((u64::MAX, 0), vec![]));
            if entry.0 != ckey {
                *entry = (ckey, crease_edges(&o.evaluated_mesh, p.crease_angle));
            }
            let c = Color::srgb(p.color[0], p.color[1], p.color[2]);
            for (a, b) in &entry.1 {
                ink.line(world.transform_point3(*a), world.transform_point3(*b), c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};

    #[test]
    fn hull_pushes_every_corner_outward() {
        let m = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
        let t = m.to_tri_mesh();
        let c = m.bounds().unwrap().center();
        let hull = hull_positions(&t, 0.1);
        assert_eq!(hull.len(), t.positions.len());
        for (p, h) in t.positions.iter().zip(&hull) {
            let (p, h) = (Vec3::from_array(*p), Vec3::from_array(*h));
            assert!((h - c).length() > (p - c).length() + 0.05, "hull vertex moved outward");
        }
        // Coincident corners (split for hard normals) move to the same spot.
        let mut seen: HashMap<[i64; 3], Vec3> = HashMap::new();
        for (p, h) in t.positions.iter().zip(&hull) {
            let k = p.map(|x| (x * 1e4).round() as i64);
            let prev = *seen.entry(k).or_insert(Vec3::from_array(*h));
            assert!(prev.distance(Vec3::from_array(*h)) < 1e-5);
        }
    }

    #[test]
    fn creases_follow_the_angle_threshold() {
        let cube = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
        assert_eq!(crease_edges(&cube, 45.0).len(), cube.edges.len(), "every box edge is 90°");
        assert_eq!(crease_edges(&cube, 95.0).len(), 0);
        let sphere = Primitive::default_of(PrimitiveKind::Sphere).to_editable_poly();
        assert!(crease_edges(&sphere, 40.0).is_empty(), "smooth sphere has no creases");
        let plane = Primitive::default_of(PrimitiveKind::Plane).to_editable_poly();
        assert!(!crease_edges(&plane, 40.0).is_empty(), "open borders are always inked");
    }
}
