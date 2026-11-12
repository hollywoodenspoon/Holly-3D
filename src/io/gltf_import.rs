//! glTF / GLB import. Files written by Ilex are rebuilt into their
//! original polygons from `extras.ilex` (or `extras.minimax` from before
//! the rename); other files come in as
//! triangles welded by position (one material ID per primitive).

use super::{MatInfo, MeshItem, y_to_z_up, z_to_y_up};
use crate::mesh::{EditablePoly, FaceCorner, Polygon, VertexId};
use bevy::math::{Mat4, Vec2, Vec3, Vec4};
use std::collections::HashMap;
use std::path::Path;

#[derive(serde::Deserialize, Default)]
struct Extra {
    faces: Vec<usize>,
    sg: Vec<u32>,
    vid: Vec<u32>,
    mat_id: u16,
}

#[derive(serde::Deserialize)]
struct Extras {
    #[serde(alias = "minimax")]
    ilex: Extra,
}

pub fn import(path: &Path) -> Result<Vec<MeshItem>, String> {
    let (doc, buffers, _) = gltf::import(path).map_err(|e| e.to_string())?;
    let scene = doc.default_scene().or_else(|| doc.scenes().next()).ok_or("glTF file has no scene")?;
    let mut out = vec![];
    for node in scene.nodes() {
        visit(&node, Mat4::IDENTITY, &buffers, &mut out);
    }
    Ok(out)
}

fn visit(node: &gltf::Node, parent: Mat4, buffers: &[gltf::buffer::Data], out: &mut Vec<MeshItem>) {
    let world = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        let name = node.name().or(mesh.name()).unwrap_or("Mesh").to_string();
        let (mesh_z, materials) = read_mesh(&mesh, buffers);
        let transform = y_to_z_up() * world * z_to_y_up();
        if !mesh_z.polygons.is_empty() {
            out.push(MeshItem { name, mesh: mesh_z, transform, materials });
        }
    }
    for c in node.children() {
        visit(&c, world, buffers, out);
    }
}

fn material_info(m: &gltf::Material) -> MatInfo {
    let pbr = m.pbr_metallic_roughness();
    let [r, g, b, a] = pbr.base_color_factor();
    MatInfo {
        name: m.name().unwrap_or("Material").to_string(),
        base_color: [r, g, b],
        opacity: a,
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
        emissive: m.emissive_factor(),
    }
}

/// Mesh (Z-up, own space) and its material list (`[id - 1]`).
fn read_mesh(mesh: &gltf::Mesh, buffers: &[gltf::buffer::Data]) -> (EditablePoly, Vec<MatInfo>) {
    let zup = y_to_z_up();
    let mut out = EditablePoly::new();
    let mut materials: Vec<MatInfo> = vec![];
    let mut by_vid: HashMap<u32, VertexId> = HashMap::new();
    let mut by_pos: HashMap<[u32; 3], VertexId> = HashMap::new();
    for (pi, prim) in mesh.primitives().enumerate() {
        if prim.mode() != gltf::mesh::Mode::Triangles {
            continue;
        }
        let r = prim.reader(|b| buffers.get(b.index()).map(|d| &d.0[..]));
        let Some(pos) = r.read_positions() else { continue };
        let pos: Vec<Vec3> = pos.map(|p| zup.transform_point3(Vec3::from(p))).collect();
        let uvs: Vec<Vec2> = r.read_tex_coords(0).map_or(vec![], |t| t.into_f32().map(|[u, v]| Vec2::new(u, 1.0 - v)).collect());
        let cols: Vec<Vec4> = r.read_colors(0).map_or(vec![], |c| c.into_rgba_f32().map(Vec4::from).collect());
        let idx: Vec<u32> = r.read_indices().map_or_else(|| (0..pos.len() as u32).collect(), |i| i.into_u32().collect());
        let extra: Option<Extra> = prim.extras().as_ref().and_then(|raw| serde_json::from_str::<Extras>(raw.get()).ok()).map(|e| e.ilex);
        let id = extra.as_ref().map_or(pi as u16 + 1, |e| e.mat_id.max(1));
        let slot = id as usize - 1;
        if materials.len() <= slot {
            materials.resize(slot + 1, MatInfo::default());
        }
        materials[slot] = material_info(&prim.material());
        let uv = |i: usize| uvs.get(i).copied().unwrap_or(Vec2::ZERO);
        let mut vert = |i: usize, key: Option<u32>| -> VertexId {
            let make = |out: &mut EditablePoly| {
                let v = out.add_vertex(pos[i]);
                out.vertices[v].color = cols.get(i).copied().unwrap_or(Vec4::ONE);
                v
            };
            match key {
                Some(k) => *by_vid.entry(k).or_insert_with(|| make(&mut out)),
                None => *by_pos.entry(pos[i].to_array().map(f32::to_bits)).or_insert_with(|| make(&mut out)),
            }
        };
        let mut template = Polygon::new(vec![]);
        template.material_id = id;
        let mut polys: Vec<(Vec<FaceCorner>, u32)> = vec![];
        match &extra {
            Some(e) if e.faces.iter().sum::<usize>() == pos.len() && e.vid.len() == pos.len() => {
                let mut base = 0;
                for (f, &n) in e.faces.iter().enumerate() {
                    let corners = (base..base + n).map(|i| FaceCorner::new(vert(i, Some(e.vid[i])), uv(i))).collect();
                    polys.push((corners, e.sg.get(f).copied().unwrap_or(1)));
                    base += n;
                }
            }
            _ => {
                for t in idx.as_chunks::<3>().0 {
                    let corners = t.iter().map(|&i| FaceCorner::new(vert(i as usize, None), uv(i as usize))).collect();
                    polys.push((corners, 1));
                }
            }
        }
        for (corners, sg) in polys {
            template.smoothing_group = sg;
            out.add_polygon_corners(corners, &template);
        }
    }
    out.finalize();
    (out, materials)
}
