//! Panel Groove modifier: non-destructive Panel Line / Seam Groove along a
//! filtered edge set (by default the edges selected below it in the stack).

use super::chamfer::{ChamferFilter, ChamferMod};
use crate::mesh::EditablePoly;
use crate::mesh::ops::groove::panel_groove;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PanelGrooveMod {
    pub width: f32,
    pub depth: f32,
    pub lip: f32,
    pub material_id: u16,
    pub filter: ChamferFilter,
}

impl Default for PanelGrooveMod {
    fn default() -> Self {
        Self { width: 0.04, depth: 0.03, lip: 0.008, material_id: 2, filter: ChamferFilter::Selected }
    }
}

impl PanelGrooveMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        // The Chamfer modifier's edge filter picks the seams.
        let picked = ChamferMod { filter: self.filter, ..ChamferMod::default() }.edges(m);
        if picked.is_empty() {
            return;
        }
        let polys: Vec<_> = m.selected_polygons();
        m.edges.values_mut().for_each(|e| e.selected = false);
        for e in picked {
            m.edges[e].selected = true;
        }
        panel_groove(m, self.width, self.depth, self.lip, self.material_id);
        // Keep the incoming polygon selection flowing up the stack.
        m.polygons.values_mut().for_each(|p| p.selected = false);
        for p in polys {
            if let Some(p) = m.polygons.get_mut(p) {
                p.selected = true;
            }
        }
    }
}
