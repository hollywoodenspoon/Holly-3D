//! File I/O: the native `.ilex` (and legacy `.mmax`) scene format and OBJ / glTF / FBX
//! import + export. Exchange formats carry polygons (quads and n-gons),
//! 3ds Max smoothing groups, UVs, vertex colours and material IDs.

pub mod fbx_export;
pub mod fbx_import;
pub mod fbx_smoothing;
pub mod gltf_export;
pub mod gltf_import;
pub mod obj;
pub mod scene_file;
#[cfg(test)]
mod tests;

use crate::materials::Material;
use crate::mesh::EditablePoly;
use bevy::math::{Mat4, Quat, Vec3};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Obj,
    Gltf,
    Glb,
    Fbx,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Obj, Format::Gltf, Format::Glb, Format::Fbx];

    pub fn from_path(p: &Path) -> Option<Self> {
        match p.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "obj" => Some(Format::Obj),
            "gltf" => Some(Format::Gltf),
            "glb" => Some(Format::Glb),
            "fbx" => Some(Format::Fbx),
            _ => None,
        }
    }
    pub fn ext(self) -> &'static str {
        match self {
            Format::Obj => "obj",
            Format::Gltf => "gltf",
            Format::Glb => "glb",
            Format::Fbx => "fbx",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Format::Obj => "Wavefront OBJ",
            Format::Gltf => "glTF 2.0",
            Format::Glb => "glTF Binary",
            Format::Fbx => "Autodesk FBX",
        }
    }
}

/// Plain material description exchanged with other packages.
#[derive(Clone, Debug, PartialEq)]
pub struct MatInfo {
    pub name: String,
    pub base_color: [f32; 3],
    pub opacity: f32,
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
}

impl MatInfo {
    pub fn from_material(m: &Material) -> Self {
        Self {
            name: m.name.clone(),
            base_color: m.base_color,
            opacity: m.opacity,
            metallic: m.metallic,
            roughness: m.roughness,
            emissive: m.emissive.map(|c| c * m.emissive_strength.max(0.0)),
        }
    }
    pub fn to_material(&self) -> Material {
        let mut m = Material::named(self.name.clone());
        m.base_color = self.base_color;
        m.opacity = self.opacity;
        m.metallic = self.metallic;
        m.roughness = self.roughness;
        if self.emissive.iter().any(|&c| c > 0.0) {
            m.emissive = self.emissive;
            m.emissive_strength = 1.0;
        }
        m
    }
}

impl Default for MatInfo {
    fn default() -> Self {
        Self { name: "Material".into(), base_color: [0.8; 3], opacity: 1.0, metallic: 0.0, roughness: 0.5, emissive: [0.0; 3] }
    }
}

/// One exchanged mesh object: geometry in its own (pivot) space plus the
/// node transform. `materials[id - 1]` describes material ID `id`.
#[derive(Clone, Debug)]
pub struct MeshItem {
    pub name: String,
    pub mesh: EditablePoly,
    pub transform: Mat4,
    pub materials: Vec<MatInfo>,
}

impl MeshItem {
    /// Mesh baked into world space.
    pub fn world_mesh(&self) -> EditablePoly {
        let mut m = self.mesh.clone();
        m.transform(self.transform);
        m
    }
    /// Material IDs used by the mesh, sorted.
    pub fn material_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self.mesh.polygons.values().map(|p| p.material_id.max(1)).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
    /// Material used for ID `id` (falls back to the first, then a default).
    pub fn material_for(&self, id: u16) -> MatInfo {
        let i = id.max(1) as usize - 1;
        self.materials.get(i).or(self.materials.first()).cloned().unwrap_or_default()
    }
    /// Name written for material ID `id`.
    pub fn material_name(&self, id: u16) -> String {
        let base = self.material_for(id).name;
        if self.material_ids().len() > 1 || self.materials.len() > 1 { format!("{base}_ID{id}") } else { base }
    }
}

/// Z-up (Ilex / 3ds Max) to Y-up (glTF, OBJ convention): rotate -90° about X.
pub fn z_to_y_up() -> Mat4 {
    Mat4::from_quat(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
}

pub fn y_to_z_up() -> Mat4 {
    z_to_y_up().inverse()
}

/// Imports any supported exchange file.
pub fn import(path: &Path) -> Result<Vec<MeshItem>, String> {
    match Format::from_path(path) {
        Some(Format::Obj) => obj::import(path),
        Some(Format::Gltf | Format::Glb) => gltf_import::import(path),
        Some(Format::Fbx) => fbx_import::import(path),
        None => Err(format!("Unsupported file type: {}", path.display())),
    }
}

/// Exports mesh items in the format given by the file extension.
pub fn export(path: &Path, items: &[MeshItem]) -> Result<(), String> {
    match Format::from_path(path) {
        Some(Format::Obj) => obj::export(path, items),
        Some(Format::Gltf) => gltf_export::export(path, items, false),
        Some(Format::Glb) => gltf_export::export(path, items, true),
        Some(Format::Fbx) => fbx_export::export(path, items),
        None => Err(format!("Unsupported file type: {}", path.display())),
    }
}

/// Centre of an item's world bounding box (where imported pivots land).
pub fn bounds_center(m: &EditablePoly) -> Vec3 {
    crate::mesh::Aabb::from_points(m.vertices.values().map(|v| v.position)).map_or(Vec3::ZERO, |b| b.center())
}
