//! glTF 2.0 export (`.gltf` + `.bin`, or a single `.glb`). glTF only stores
//! triangles, so every primitive also carries `extras.minimax` with the
//! polygon sizes, smoothing groups and source vertex indices; the MiniMax
//! importer uses them to rebuild the original quads / n-gons exactly.

use super::{MatInfo, MeshItem, z_to_y_up};
use crate::mesh::triangulate::triangulate_polygon;
use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Mat4, Vec3};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;

const FLOAT: u32 = 5126;
const UINT: u32 = 5125;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Builder {
    fn push(&mut self, bytes: &[u8], target: u32) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        self.views.push(json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len(), "target": target}));
        self.bin.extend_from_slice(bytes);
        self.views.len() - 1
    }
    fn floats(&mut self, data: &[f32], kind: &str, width: usize, bounds: bool) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.push(&bytes, ARRAY_BUFFER);
        let mut acc = json!({"bufferView": view, "componentType": FLOAT, "count": data.len() / width, "type": kind});
        if bounds {
            let (mut lo, mut hi) = (vec![f32::MAX; width], vec![f32::MIN; width]);
            for c in data.chunks(width) {
                for i in 0..width {
                    lo[i] = lo[i].min(c[i]);
                    hi[i] = hi[i].max(c[i]);
                }
            }
            acc["min"] = json!(lo);
            acc["max"] = json!(hi);
        }
        self.accessors.push(acc);
        self.accessors.len() - 1
    }
    fn indices(&mut self, data: &[u32]) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|i| i.to_le_bytes()).collect();
        let view = self.push(&bytes, ELEMENT_ARRAY_BUFFER);
        self.accessors.push(json!({"bufferView": view, "componentType": UINT, "count": data.len(), "type": "SCALAR"}));
        self.accessors.len() - 1
    }
}

fn material_json(m: &MatInfo) -> Value {
    let [r, g, b] = m.base_color;
    let mut v = json!({
        "name": m.name,
        "pbrMetallicRoughness": {"baseColorFactor": [r, g, b, m.opacity], "metallicFactor": m.metallic, "roughnessFactor": m.roughness},
        "emissiveFactor": m.emissive.map(|c| c.min(1.0)),
        "doubleSided": false,
    });
    if m.opacity < 1.0 {
        v["alphaMode"] = json!("BLEND");
    }
    v
}

/// One primitive per material ID; vertices are split per face corner.
fn primitive(b: &mut Builder, m: &EditablePoly, id: u16, mat: usize, yup: Mat4) -> Value {
    let normals = m.corner_normals();
    let vindex: HashMap<VertexId, u32> = m.vertices.keys().enumerate().map(|(i, k)| (k, i as u32)).collect();
    let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
    let (mut faces, mut sgs, mut vids) = (vec![], vec![], vec![]);
    for (pid, p) in m.polygons.iter().filter(|(_, p)| p.material_id.max(1) == id) {
        let base = (pos.len() / 3) as u32;
        let pts: Vec<Vec3> = p.vertex_ids().map(|v| m.pos(v)).collect();
        for (i, c) in p.corners.iter().enumerate() {
            let v = &m.vertices[c.vertex_id];
            pos.extend(yup.transform_point3(v.position).to_array());
            nrm.extend(yup.transform_vector3(normals[&pid][i]).normalize_or_zero().to_array());
            uv.extend([c.uv.x, 1.0 - c.uv.y]);
            col.extend(v.color.to_array());
            vids.push(vindex[&c.vertex_id]);
        }
        for t in triangulate_polygon(&pts) {
            idx.extend(t.map(|k| base + k as u32));
        }
        faces.push(p.corners.len());
        sgs.push(p.smoothing_group);
    }
    let a_pos = b.floats(&pos, "VEC3", 3, true);
    let a_nrm = b.floats(&nrm, "VEC3", 3, false);
    let a_uv = b.floats(&uv, "VEC2", 2, false);
    let a_col = b.floats(&col, "VEC4", 4, false);
    let a_idx = b.indices(&idx);
    json!({
        "attributes": {"POSITION": a_pos, "NORMAL": a_nrm, "TEXCOORD_0": a_uv, "COLOR_0": a_col},
        "indices": a_idx,
        "material": mat,
        "mode": 4,
        "extras": {"minimax": {"faces": faces, "sg": sgs, "vid": vids, "mat_id": id}},
    })
}

/// The complete glTF document and its binary buffer.
pub fn build(items: &[MeshItem], bin_uri: Option<&str>) -> (Value, Vec<u8>) {
    let yup = z_to_y_up();
    let mut b = Builder::default();
    let (mut meshes, mut nodes, mut materials): (Vec<Value>, Vec<Value>, Vec<Value>) = (vec![], vec![], vec![]);
    let mut mat_index: HashMap<String, usize> = HashMap::new();
    for item in items {
        let mut prims = vec![];
        for id in item.material_ids() {
            let info = MatInfo { name: item.material_name(id), ..item.material_for(id) };
            let mi = *mat_index.entry(info.name.clone()).or_insert_with(|| {
                materials.push(material_json(&info));
                materials.len() - 1
            });
            prims.push(primitive(&mut b, &item.mesh, id, mi, yup));
        }
        meshes.push(json!({"name": item.name, "primitives": prims}));
        let (s, r, t) = (yup * item.transform * yup.inverse()).to_scale_rotation_translation();
        nodes.push(json!({"name": item.name, "mesh": meshes.len() - 1, "translation": t.to_array(), "rotation": r.to_array(), "scale": s.to_array()}));
    }
    let mut buffer = json!({"byteLength": b.bin.len()});
    if let Some(uri) = bin_uri {
        buffer["uri"] = json!(uri);
    }
    let doc = json!({
        "asset": {"version": "2.0", "generator": "MiniMax"},
        "scene": 0,
        "scenes": [{"nodes": (0..nodes.len()).collect::<Vec<_>>()}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "buffers": [buffer],
        "bufferViews": b.views,
        "accessors": b.accessors,
    });
    (doc, b.bin)
}

pub fn export(path: &Path, items: &[MeshItem], binary: bool) -> Result<(), String> {
    if binary {
        let (doc, bin) = build(items, None);
        return std::fs::write(path, glb_bytes(&doc, &bin)).map_err(|e| e.to_string());
    }
    let bin_path = path.with_extension("bin");
    let uri = bin_path.file_name().and_then(|s| s.to_str()).unwrap_or("scene.bin").to_string();
    let (doc, bin) = build(items, Some(&uri));
    std::fs::write(&bin_path, bin).map_err(|e| e.to_string())?;
    std::fs::write(path, serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// GLB container: 12-byte header, JSON chunk, BIN chunk (4-byte aligned).
pub fn glb_bytes(doc: &Value, bin: &[u8]) -> Vec<u8> {
    let mut json = serde_json::to_vec(doc).unwrap_or_default();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut bin = bin.to_vec();
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(bin);
    out
}
