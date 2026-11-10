//! Bound hoses: each frame the ends of a hose bound to Top / Bottom End
//! Objects are moved to those objects' pivots (in the hose's own space), so
//! the hose flexes live as the parts move.

use super::MechPart;
use crate::scene::{BaseObject, ObjId, Scene};
use bevy::math::Vec3;

impl Scene {
    /// World pivot and travel direction (down the object's -Z) of a bound end.
    fn hose_end(&self, id: ObjId) -> Option<(Vec3, Vec3)> {
        let x = &self.objects.get(id)?.transform;
        Some((x.translation, x.rotation * -Vec3::Z))
    }

    /// Re-fits every bound hose; true when one changed.
    pub fn update_hoses(&mut self) -> bool {
        let ids: Vec<ObjId> =
            self.objects.iter().filter(|(_, o)| matches!(o.base, BaseObject::Mech(MechPart::Hose(_)))).map(|(k, _)| k).collect();
        let mut changed = false;
        for id in ids {
            let Some(MechPart::Hose(h)) = self.objects[id].mech() else { continue };
            let mut h = h.clone();
            let before = h.clone();
            let inv = self.objects[id].world_matrix().inverse();
            for (bound, top) in [(h.top, true), (h.bottom, false)] {
                let Some(b) = bound else { continue };
                let Some((p, d)) = self.hose_end(b).filter(|_| b != id) else {
                    // The end object is gone: that end stays where it was, free.
                    if top {
                        h.top = None
                    } else {
                        h.bottom = None
                    }
                    continue;
                };
                let (lp, ld) = (inv.transform_point3(p), inv.transform_vector3(d).normalize_or(Vec3::Z));
                if top {
                    (h.start, h.start_dir) = (lp, ld);
                } else {
                    (h.end, h.end_dir) = (lp, ld);
                }
            }
            let moved = |a: Vec3, b: Vec3| a.distance(b) > 1e-5;
            let differs = h.top != before.top
                || h.bottom != before.bottom
                || moved(h.start, before.start)
                || moved(h.end, before.end)
                || moved(h.start_dir, before.start_dir)
                || moved(h.end_dir, before.end_dir);
            if differs {
                let o = &mut self.objects[id];
                if let Some(MechPart::Hose(dst)) = o.mech_mut() {
                    *dst = h;
                }
                o.evaluate();
                changed = true;
            }
        }
        changed
    }
}
