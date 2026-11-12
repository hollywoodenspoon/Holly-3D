//! Live Light-Mixer: the path tracer keeps one radiance buffer per light
//! group (sun & sky, environment, emissive materials, godrays, and up to
//! eight scene-light groups). After F9 finishes, the Frame Window's LightMix
//! tab re-weights them as `Σ group × intensity × tint` and re-runs the post
//! chain and Color Grading — relighting without re-rendering.

use super::post::PixelLayers;
use super::postfx::{PostCtx, run_ctx};
use super::settings::RenderSettings;
use super::trace::{GSample, Pixel};
use crate::materials::Material;
use crate::scene::ObjId;
use bevy::math::{Vec3, Vec4};

/// Most groups a render keeps: 4 fixed + 8 scene-light groups.
pub const GROUPS: usize = 12;
pub type Groups = [Vec3; GROUPS];

pub const SUN_SKY: u8 = 0;
pub const ENV: u8 = 1;
pub const EMISSIVE: u8 = 2;
pub const VOLUME: u8 = 3;
pub const FIRST_LIGHT: u8 = 4;

/// One group: its name and the scene lights that feed it.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupInfo {
    pub name: String,
    pub lights: Vec<ObjId>,
}

impl GroupInfo {
    fn named(name: &str) -> Self {
        Self { name: name.into(), lights: vec![] }
    }
}

/// The four fixed groups.
pub fn fixed_groups() -> Vec<GroupInfo> {
    ["Sun & Dynamic Sky", "Environment / HDRI & Ambient", "Emissive Materials", "Volumetric Godrays & Mist"].map(GroupInfo::named).to_vec()
}

/// Group of the next scene light: one group each, the ninth and later
/// lights share the last.
pub fn light_group(groups: &mut Vec<GroupInfo>, id: Option<ObjId>, name: &str) -> u8 {
    let n = groups.len();
    if n < GROUPS {
        groups.push(GroupInfo { name: name.into(), lights: id.into_iter().collect() });
        return n as u8;
    }
    let last = groups.last_mut().expect("groups");
    if last.lights.len() == 1 {
        last.name = "Other Scene Lights".into();
    }
    last.lights.extend(id);
    (GROUPS - 1) as u8
}

/// One group's fader in the LightMix tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fader {
    pub on: bool,
    pub solo: bool,
    pub intensity: f32,
    pub tint: [f32; 3],
}

impl Default for Fader {
    fn default() -> Self {
        Self { on: true, solo: false, intensity: 1.0, tint: [1.0; 3] }
    }
}

impl Fader {
    pub fn is_neutral(&self) -> bool {
        self.on && self.intensity == 1.0 && self.tint == [1.0; 3]
    }
}

/// Linear weight of each group; any Solo hides the groups not soloed.
pub fn weights(f: &[Fader]) -> Vec<Vec3> {
    let solo = f.iter().any(|g| g.solo);
    f.iter()
        .map(|g| {
            let live = g.on && (!solo || g.solo);
            if live { Vec3::from_array(g.tint.map(crate::materials::shading::srgb_to_linear)) * g.intensity } else { Vec3::ZERO }
        })
        .collect()
}

/// The raw (pre-post) path-traced image, split by light group.
#[derive(Clone, Debug)]
pub struct Buffers {
    /// Frame-buffer region `[x0, y0, x1, y1]` these pixels cover.
    pub region: crate::render::job::Region,
    pub groups: Vec<GroupInfo>,
    /// `w × h × groups.len()` linear radiance, pixel-major.
    pub data: Vec<Vec3>,
    pub alpha: Vec<f32>,
    pub back: Vec<Vec3>,
    pub g: Vec<GSample>,
    pub faders: Vec<Fader>,
    pub cfg: RenderSettings,
    pub materials: Vec<Material>,
    pub noisy: bool,
    pub coc: Option<f32>,
    pub plain_bg: bool,
    /// À-Trous denoiser guides (empty: the bilateral denoiser).
    pub aux: Vec<super::denoise_atrous::Aux>,
}

impl Buffers {
    pub fn size(&self) -> (usize, usize) {
        let [x0, y0, x1, y1] = self.region;
        ((x1 - x0) as usize, (y1 - y0) as usize)
    }

    /// Black buffers for `groups` over `region`.
    pub fn blank(groups: Vec<GroupInfo>, region: crate::render::job::Region, cfg: RenderSettings) -> Self {
        let n = ((region[2] - region[0]) * (region[3] - region[1])) as usize;
        Self {
            data: vec![Vec3::ZERO; n * groups.len()],
            faders: vec![Fader::default(); groups.len()],
            groups,
            region,
            alpha: vec![0.0; n],
            back: vec![Vec3::ZERO; n],
            g: vec![GSample::EMPTY; n],
            cfg,
            materials: vec![],
            noisy: false,
            coc: None,
            plain_bg: true,
            aux: vec![],
        }
    }

    /// Mixes the groups with the faders and runs the post chain.
    pub fn composite(&self) -> Vec<Pixel> {
        let (k, (w, h)) = (self.groups.len(), self.size());
        let wts = weights(&self.faders);
        let mut px: Vec<Pixel> = (0..w * h)
            .map(|i| {
                let c: Vec3 = self.data[i * k..(i + 1) * k].iter().zip(&wts).map(|(c, w)| *c * *w).sum();
                let layers = PixelLayers { geom: Vec4::from((c, self.alpha[i])), back: self.back[i], ..PixelLayers::ZERO };
                Pixel { g: self.g[i], layers, ..Pixel::EMPTY }
            })
            .collect();
        let ctx = PostCtx { noisy: self.noisy, coc: self.coc, materials: &self.materials, plain_bg: self.plain_bg, aux: (!self.aux.is_empty()).then_some(&self.aux[..]) };
        run_ctx(&mut px, w, h, &ctx, &self.cfg);
        px
    }

    /// Sum of one group over the image (UI meters, tests).
    pub fn energy(&self, group: usize) -> f32 {
        let k = self.groups.len();
        self.data.iter().skip(group).step_by(k.max(1)).map(|c| c.x + c.y + c.z).sum()
    }

    /// Resets every fader.
    pub fn reset(&mut self) {
        self.faders.iter_mut().for_each(|f| *f = Fader::default());
    }
}

#[cfg(test)]
#[path = "tests_lightmix.rs"]
mod tests;
