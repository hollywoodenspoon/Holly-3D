//! Dynamic sky (`8`): `[✓] Link Main Directional Sun to Sky Time` keeps the
//! scene's first directional light (created as "Sun" when missing) aimed
//! along the sky's sun / moon / star, with matching color and Multiplier.

use super::*;
use crate::render::lighting::aim;
use crate::render::{Light, LightKind};

/// Distance from its aim point the linked sun is placed at.
const SUN_DIST: f32 = 20.0;

impl Editor {
    /// The first directional light (the "main sun").
    pub fn main_sun(&self) -> Option<ObjId> {
        self.scene.objects.iter().find(|(_, o)| o.light().is_some_and(|l| l.kind.is_directional())).map(|(k, _)| k)
    }

    /// Re-aims the main sun at the sky's key light; true if anything moved.
    pub fn link_sun_to_sky(&mut self) -> bool {
        let sky = &self.scene.environment.sky;
        let Some((dir, color, mult)) = sky.key_light().filter(|_| sky.link_sun) else { return false };
        let id = match self.main_sun() {
            Some(id) => id,
            None => {
                let l = Light { cast_shadows: true, ..Light::new(LightKind::FreeDirect) };
                self.scene.add_light(l, Some("Sun"), dir * SUN_DIST, Vec3::ZERO)
            }
        };
        let aim_at =
            self.scene.objects[id].target_id().and_then(|t| self.scene.objects.get(t)).map_or(Vec3::ZERO, |t| t.transform.translation);
        let pos = aim_at + dir * SUN_DIST;
        let rot = aim(pos, aim_at);
        let col = color.to_array();
        let o = &self.scene.objects[id];
        let same = o.transform.translation.distance(pos) < 1e-4
            && o.light().is_some_and(|l| l.on && l.color == col && (l.multiplier - mult).abs() < 1e-5);
        if same {
            return false;
        }
        let l = self.scene.objects[id].light_mut().expect("directional light");
        (l.color, l.multiplier, l.on) = (col, mult, true);
        let o = &mut self.scene.objects[id];
        (o.transform.translation, o.transform.rotation) = (pos, rot);
        o.version = crate::scene::next_version();
        true
    }
}

impl Editor {
    /// Applies a one-click sky preset (one undo step) and re-links the sun.
    pub fn apply_sky_preset(&mut self, p: crate::render::sky_presets::SkyPreset) {
        self.checkpoint("Sky Preset");
        p.apply(&mut self.scene.environment.sky);
        self.link_sun_to_sky();
        self.status = format!("Sky: {}", p.label());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sky::SkyMode;

    #[test]
    fn linked_sun_follows_time_of_day() {
        let mut ed = Editor::default();
        ed.scene.environment.sky.mode = SkyMode::Earth;
        ed.scene.environment.sky.time_of_day = 12.0;
        assert!(ed.link_sun_to_sky());
        let id = ed.main_sun().expect("a Sun was created");
        let dir = |ed: &Editor| -crate::render::lighting::emit_dir(ed.scene.objects[id].transform.rotation);
        assert!(dir(&ed).z > 0.7, "noon sun is high: {}", dir(&ed));
        let noon = ed.scene.objects[id].light().unwrap().color;
        ed.scene.environment.sky.time_of_day = 17.6;
        assert!(ed.link_sun_to_sky());
        assert!(dir(&ed).z.abs() < 0.2, "evening sun near the horizon");
        let dusk = ed.scene.objects[id].light().unwrap().color;
        assert!(dusk[2] < noon[2] - 0.2, "golden hour is warmer: {dusk:?} vs {noon:?}");
        ed.scene.environment.sky.time_of_day = 0.0;
        ed.link_sun_to_sky();
        let l = ed.scene.objects[id].light().unwrap();
        assert!(l.multiplier < 0.3 && l.color[2] > l.color[0], "moonlight at midnight");
        assert!(!ed.link_sun_to_sky(), "nothing changes without a new time");
        assert_eq!(ed.scene.objects.values().filter(|o| o.light().is_some()).count(), 1, "one sun, reused");
        ed.scene.environment.sky.link_sun = false;
        ed.scene.environment.sky.time_of_day = 12.0;
        assert!(!ed.link_sun_to_sky());
    }
}
