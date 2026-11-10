//! Battlefield `Blast Zones` sub-object: click / marquee selection of the
//! zone spheres (moved with W and scaled with R through `drag_mod_sub`).

use super::Editor;
use super::select::{SelectOp, VERTEX_PICK_PX};
use crate::modifiers::ModifierKind;
use bevy::math::{Vec2, Vec3};

impl Editor {
    /// Screen centre and screen radius of every zone of the active Battlefield.
    fn zone_screen(&self, view: usize) -> Vec<(usize, Vec2, f32)> {
        let Some(o) = self.current_obj() else { return vec![] };
        let Some(ModifierKind::Battlefield(b)) = o.active_modifier() else { return vec![] };
        let (w, v) = (o.world_matrix(), &self.viewports.views[view]);
        let right = v.camera.right();
        b.zones
            .iter()
            .enumerate()
            .filter_map(|(i, z)| {
                let c = w.transform_point3(z.center);
                let s = v.to_screen(c)?;
                let edge = c + right * w.transform_vector3(Vec3::X * z.radius).length();
                let r = v.to_screen(edge).map_or(0.0, |e| e.distance(s));
                Some((i, s, r))
            })
            .collect()
    }

    fn set_zone_selection(&mut self, hits: &[usize], op: SelectOp) {
        let Some(id) = self.current() else { return };
        let o = &mut self.scene.objects[id];
        if let Some(ModifierKind::Battlefield(b)) = o.active_modifier_mut() {
            if op == SelectOp::Replace {
                b.zones.iter_mut().for_each(|z| z.selected = false);
            }
            for &i in hits {
                if let Some(z) = b.zones.get_mut(i) {
                    z.selected = op != SelectOp::Subtract;
                }
            }
        }
        o.selection_version = crate::scene::next_version();
    }

    /// Click: the zone whose centre (or sphere outline) is under the cursor.
    pub(crate) fn zones_click(&mut self, view: usize, cursor: Vec2, op: SelectOp) {
        let hit = self
            .zone_screen(view)
            .into_iter()
            .map(|(i, s, r)| (i, s.distance(cursor), r))
            .filter(|&(_, d, r)| d <= VERTEX_PICK_PX.max(r))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let hits: Vec<usize> = hit.map(|h| h.0).into_iter().collect();
        self.set_zone_selection(&hits, op);
    }

    /// Marquee: zones whose centre falls inside the region.
    pub(crate) fn zones_marquee(&mut self, view: usize, region: &crate::viewport::region::Region, op: SelectOp) {
        let hits: Vec<usize> = self.zone_screen(view).into_iter().filter(|(_, s, _)| region.contains(*s)).map(|x| x.0).collect();
        self.set_zone_selection(&hits, op);
    }
}
