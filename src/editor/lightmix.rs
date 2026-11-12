//! Frame Window LightMix: re-mixes the finished render's light groups into
//! the frame buffer, and bakes the mix back into the scene lights.

use super::Editor;
use crate::materials::shading::{lin3, linear_to_srgb};
use crate::render::lightmix::weights;

impl Editor {
    /// Re-composites the light groups with the current faders, then
    /// re-applies tone mapping and Color Grading (no re-render).
    pub fn remix_render(&mut self) {
        let (Some(mix), Some(fb)) = (self.rfw.lightmix.as_ref(), self.rfw.fb.as_mut()) else { return };
        let px = mix.composite();
        let [x0, y0, x1, _] = mix.region;
        let (display, hdr): (Vec<_>, Vec<_>) = px.iter().map(|p| (p.display, p.hdr)).unzip();
        fb.put_block(x0, y0, x1 - x0, &display, &hdr);
        fb.put_layers(x0, y0, x1 - x0, &px.iter().map(|p| p.layers).collect::<Vec<_>>());
        self.retone_render();
    }

    /// Resets every fader and shows the original render again.
    pub fn reset_lightmix(&mut self) {
        if let Some(m) = self.rfw.lightmix.as_mut() {
            m.reset();
            self.remix_render();
        }
    }

    /// `[ Apply LightMix to Scene Lights ]`: multiplies each mixed light's
    /// Multiplier and Color by its group's fader (one undo step), so the
    /// next F9 matches the mix. Groups without scene lights (sky, HDRI,
    /// emissive, godrays) stay render-only.
    pub fn apply_lightmix(&mut self) {
        let Some(mix) = self.rfw.lightmix.as_ref() else { return };
        let w = weights(&mix.faders);
        let edits: Vec<_> = mix.groups.iter().zip(&w).filter(|(_, w)| **w != bevy::math::Vec3::ONE).flat_map(|(g, w)| g.lights.iter().map(move |id| (*id, *w))).collect();
        if edits.is_empty() {
            self.status = "Luxon LightMix: no scene light changed".into();
            return;
        }
        self.checkpoint("Apply LightMix");
        let mut n = 0;
        for (id, f) in edits {
            let Some(l) = self.scene.objects.get_mut(id).and_then(|o| o.light_mut()) else { continue };
            let s = f.max_element();
            if s <= 0.0 {
                l.on = false;
            } else {
                let c = lin3(l.color) * (f / s);
                (l.color, l.multiplier) = (c.to_array().map(linear_to_srgb), l.multiplier * s);
            }
            n += 1;
        }
        if let Some(m) = self.rfw.lightmix.as_mut() {
            m.reset();
        }
        self.status = format!("Luxon LightMix applied to {n} scene light(s)");
        self.rfw.message = self.status.clone();
    }
}

#[cfg(test)]
mod tests {
    use crate::editor::Editor;
    use crate::render::{Light, LightKind};
    use bevy::math::Vec3;

    #[test]
    fn apply_lightmix_scales_the_lights_and_undoes() {
        let mut ed = Editor::default();
        let id = ed.scene.add_light(Light::new(LightKind::Omni), Some("Key"), Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO);
        let rs = crate::render::scene_build::RenderScene::build(&ed.scene, ed.render_camera(), false);
        let g = rs.lights[0].group as usize;
        assert_eq!(rs.groups[g].lights, vec![id]);
        let mut mix = crate::render::lightmix::Buffers::blank(rs.groups.clone(), [0, 0, 2, 2], ed.render.clone());
        mix.faders[g].intensity = 2.5;
        mix.faders[g].tint = [1.0, 0.5, 1.0];
        ed.rfw.lightmix = Some(Box::new(mix));
        let before = ed.scene.objects[id].light().unwrap().radiance();
        ed.apply_lightmix();
        let after = ed.scene.objects[id].light().unwrap().radiance();
        let want = before * 2.5 * Vec3::new(1.0, crate::materials::shading::srgb_to_linear(0.5), 1.0);
        assert!((after - want).abs().max_element() < 1e-3, "{after} vs {want}");
        assert!(ed.rfw.lightmix.as_ref().unwrap().faders.iter().all(|f| f.is_neutral()), "faders reset after Apply");
        ed.undo();
        assert!((ed.scene.objects[id].light().unwrap().radiance() - before).abs().max_element() < 1e-6);
    }
}
