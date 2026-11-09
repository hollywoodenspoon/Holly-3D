//! File menu actions: New / Open / Save (`.mmax`) and Import / Export of
//! OBJ, glTF / GLB and FBX.

use super::Editor;
use crate::io::{self, MatInfo, MeshItem, scene_file};
use crate::mesh::SubObject;
use crate::scene::{BaseObject, ObjId, Xform};
use bevy::math::Vec3;
use std::path::{Path, PathBuf};

impl Editor {
    pub fn new_scene(&mut self) {
        self.checkpoint("New Scene");
        self.scene = Default::default();
        self.sub = SubObject::Object;
        self.scene_path = None;
        self.status = "New scene".into();
    }

    /// Open Scene (`Ctrl+O`): replaces the scene and clears the undo history.
    pub fn open_scene(&mut self, path: &Path) -> Result<(), String> {
        let scene = scene_file::load(path)?;
        self.scene = scene;
        self.sub = SubObject::Object;
        self.caddy = None;
        self.drag = None;
        self.mod_sub = None;
        self.history = Default::default();
        self.scene_path = Some(path.to_path_buf());
        self.status = format!("Opened {}", path.display());
        Ok(())
    }

    /// Save Scene (`Ctrl+S`) / Save As.
    pub fn save_scene(&mut self, path: &Path) -> Result<(), String> {
        let path = with_ext(path, scene_file::EXT);
        scene_file::save(&self.scene, &path)?;
        self.status = format!("Saved {}", path.display());
        self.scene_path = Some(path);
        Ok(())
    }

    /// Opens a `.mmax` scene or imports any exchange file.
    pub fn open_any(&mut self, path: &Path) -> Result<(), String> {
        let is_scene = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(scene_file::EXT));
        if is_scene { self.open_scene(path) } else { self.import_file(path).map(|_| ()) }
    }

    /// Import: every mesh becomes an Editable Poly with its pivot transform
    /// and first material; the imported objects end up selected.
    pub fn import_file(&mut self, path: &Path) -> Result<Vec<ObjId>, String> {
        let items = io::import(path)?;
        if items.is_empty() {
            return Err(format!("No meshes in {}", path.display()));
        }
        self.checkpoint("Import");
        let ids = self.add_items(items);
        self.status = format!("Imported {} object(s) from {}", ids.len(), path.display());
        Ok(ids)
    }

    pub fn add_items(&mut self, items: Vec<MeshItem>) -> Vec<ObjId> {
        for o in self.scene.objects.values_mut() {
            o.selected = false;
        }
        let mut ids = vec![];
        for item in items {
            let id = self.scene.add(BaseObject::EditablePoly(item.mesh), Vec3::ZERO);
            let taken = self.scene.objects.values().any(|o| o.name == item.name);
            let name = if item.name.trim().is_empty() || taken { self.scene.unique_name("Mesh") } else { item.name };
            let mat = item.materials.first().map(|m| self.scene.materials.mats.insert(m.to_material()));
            let o = &mut self.scene.objects[id];
            o.name = name;
            o.transform = Xform::from_matrix(item.transform);
            o.material = mat;
            o.selected = true;
            o.version = crate::scene::next_version();
            ids.push(id);
        }
        self.sub = SubObject::Object;
        ids
    }

    /// Geometry items for export (all visible, or the selection only).
    pub fn export_items(&self, selected_only: bool) -> Vec<MeshItem> {
        self.scene
            .objects
            .values()
            .filter(|o| !o.is_helper() && !o.hidden && (!selected_only || o.selected) && !o.evaluated_mesh.polygons.is_empty())
            .map(|o| {
                let mut mesh = o.evaluated_mesh.clone();
                mesh.transform(o.offset);
                let materials = o.material.and_then(|m| self.scene.materials.mats.get(m)).map(MatInfo::from_material).into_iter().collect();
                MeshItem { name: o.name.clone(), mesh, transform: o.transform.matrix(), materials }
            })
            .collect()
    }

    pub fn export_file(&mut self, path: &Path, selected_only: bool) -> Result<usize, String> {
        let items = self.export_items(selected_only);
        if items.is_empty() {
            return Err(if selected_only { "Nothing selected to export".into() } else { "Scene has no geometry to export".into() });
        }
        io::export(path, &items)?;
        self.status = format!("Exported {} object(s) to {}", items.len(), path.display());
        Ok(items.len())
    }

    /// Reports a failed file action in the status bar.
    pub fn report<T>(&mut self, r: Result<T, String>) {
        if let Err(e) = r {
            self.status = format!("Error: {e}");
        }
    }
}

fn with_ext(path: &Path, ext: &str) -> PathBuf {
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) { path.to_path_buf() } else { path.with_extension(ext) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::PrimitiveKind;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minimax-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn export_selected_then_import_keeps_world_geometry() {
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Box, Vec3::new(2.0, 0.0, 0.0));
        ed.create_primitive(PrimitiveKind::Sphere, Vec3::ZERO);
        let boxid = ed.scene.objects.iter().find(|(_, o)| o.name.starts_with("Box")).unwrap().0;
        ed.scene.select_only(Some(boxid));
        assert_eq!(ed.export_items(true).len(), 1);
        assert_eq!(ed.export_items(false).len(), 2);
        for ext in ["obj", "glb", "fbx"] {
            let p = tmp(&format!("sel.{ext}"));
            ed.export_file(&p, true).unwrap();
            let mut fresh = Editor::default();
            let ids = fresh.import_file(&p).unwrap();
            assert_eq!(ids.len(), 1, "{ext}");
            let o = &fresh.scene.objects[ids[0]];
            assert_eq!(o.evaluated_mesh.counts(), (8, 12, 6), "{ext}");
            assert!(o.selected && o.material.is_some());
            let (a, b) = (ed.selection_bounds().unwrap(), fresh.selection_bounds().unwrap());
            assert!((a.min - b.min).length() < 1e-3 && (a.max - b.max).length() < 1e-3, "{ext}: {a:?} vs {b:?}");
            assert!(fresh.history.can_undo(), "import is undoable");
        }
    }

    #[test]
    fn save_and_open_scene_round_trip() {
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Torus, Vec3::new(1.0, 2.0, 0.0));
        let p = tmp("scene");
        ed.save_scene(&p).unwrap();
        assert_eq!(ed.scene_path.as_deref().unwrap().extension().unwrap(), "mmax");
        let mut other = Editor::default();
        other.open_any(&p.with_extension("mmax")).unwrap();
        assert_eq!(other.scene.objects.len(), 1);
        assert!(!other.history.can_undo());
        assert!(other.open_any(Path::new("missing.mmax")).is_err());
    }
}
