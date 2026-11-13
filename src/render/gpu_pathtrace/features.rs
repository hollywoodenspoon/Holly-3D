//! What a scene asks of the GPU kernel: the per-scene feature switches
//! (WGSL `override` constants, one compiled pipeline per mask), the list of
//! reasons a scene still needs the CPU tracer, and the time-based batch size
//! that keeps each GPU submit short.

use crate::materials::ShadingModel;
use crate::materials::proc_engine::Pattern;
use crate::render::scene_build::{RShape, RenderScene};

/// Feature switches, bit `i` = `NAMES[i]` (the WGSL override's name).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Features(pub u32);

impl Features {
    pub const TEXTURES: u32 = 1;
    pub const PRINCIPLED: u32 = 2;
    pub const SSS: u32 = 4;
    pub const VOLUME: u32 = 8;
    pub const ENV_MAP: u32 = 16;
    pub const AREA: u32 = 32;
    pub const OPACITY: u32 = 64;
    pub const NAMES: [&'static str; 7] = ["FEAT_TEXTURES", "FEAT_PRINCIPLED", "FEAT_SSS", "FEAT_VOLUME", "FEAT_ENV_MAP", "FEAT_AREA", "FEAT_OPACITY"];
    /// Every mask the kernel can be built with.
    pub const COUNT: u32 = 1 << Self::NAMES.len();

    /// The switches this scene would turn on.
    pub fn of(rs: &RenderScene) -> Self {
        let mut f = 0;
        let mut set = |on: bool, bit: u32| f |= if on { bit } else { 0 };
        set(rs.materials.iter().any(mapped), Self::TEXTURES);
        set(rs.materials.iter().any(|m| m.principled.active()), Self::PRINCIPLED);
        set(rs.materials.iter().any(|m| m.principled.has_sss()), Self::SSS);
        set(rs.volume.is_some(), Self::VOLUME);
        set(has_env_map(rs), Self::ENV_MAP);
        set(rs.lights.iter().any(|l| matches!(l.shape, RShape::Area(_))), Self::AREA);
        set(rs.translucent, Self::OPACITY);
        Self(f)
    }

    /// Pipeline-overridable constants for this mask.
    pub fn constants(self) -> Vec<(&'static str, f64)> {
        Self::NAMES.iter().enumerate().map(|(i, n)| (*n, f64::from(self.0 >> i & 1))).collect()
    }
}

/// The scene shows a procedural sky or a loaded HDRI (what the CPU's
/// `Tracer::environment` looks up instead of the flat colours).
pub fn has_env_map(rs: &RenderScene) -> bool {
    rs.procedural.is_some() || rs.hdri.as_ref().is_some_and(|h| h.image.is_some())
}

/// The material has maps, a procedural pattern, paint layers or a projection.
pub fn mapped(m: &crate::materials::Material) -> bool {
    let maps = m.maps.iter().any(|s| s.enabled && s.node.is_some()) || !m.slate_nodes.is_empty();
    maps || m.proc.pattern != Pattern::None || m.paint.is_some() || m.projection.on()
}

/// Everything that still sends a scene to the CPU tracer, in the order
/// [`reasons`] lists it (the Render tab's tooltip shows this list).
pub const CPU_ONLY: [&str; 8] = [
    "volumetric godrays / mist",
    "area lights",
    "IES light profiles",
    "Shadow & Reflection Catcher surfaces",
    "see-through (opacity) surfaces",
    "cel / toon materials",
    "maps the GPU cannot bake (3D procedural, or triplanar with paint or non-repeating maps)",
    "more than 1.5 GB of baked textures",
];

/// Every reason this scene renders on the CPU instead (empty = the GPU
/// handles it), in a fixed order without repeats.
pub fn reasons(rs: &RenderScene) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut add = |on: bool, why: &'static str| {
        if on {
            out.push(why);
        }
    };
    let mats = &rs.materials;
    add(rs.volume.is_some(), CPU_ONLY[0]);
    add(rs.lights.iter().any(|l| matches!(l.shape, RShape::Area(_))), CPU_ONLY[1]);
    add(rs.lights.iter().any(|l| l.ies != crate::render::IesProfile::None), CPU_ONLY[2]);
    add(mats.iter().any(|m| m.shadow_catcher), CPU_ONLY[3]);
    add(rs.translucent, CPU_ONLY[4]);
    add(mats.iter().any(|m| m.model == ShadingModel::Toon), CPU_ONLY[5]);
    add(mats.iter().any(|m| mapped(m) && super::bake::why_not(m).is_some()), CPU_ONLY[6]);
    add(super::textures::planned_bytes(rs) > super::bake::CAP_BYTES, CPU_ONLY[7]);
    out
}

/// The fallback label's text: every reason, joined ("a, b and c").
pub fn join(reasons: &[&str]) -> String {
    match reasons {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Fewest and most samples per pixel one GPU submit traces.
pub const SPP_RANGE: (u32, u32) = (1, 8);
/// Wall time one submit aims for: short enough that the desktop stays
/// responsive and the driver watchdog never resets the card.
pub const TARGET_MS: f64 = 50.0;

/// The next submit's samples per pixel from how long `cur` samples took:
/// scales towards [`TARGET_MS`], at most doubling or halving per step.
pub fn next_spp(cur: u32, elapsed_ms: f64, target_ms: f64) -> u32 {
    let (lo, hi) = SPP_RANGE;
    let cur = cur.clamp(lo, hi);
    if !elapsed_ms.is_finite() || elapsed_ms <= 0.0 {
        return (cur * 2).min(hi);
    }
    let want = (f64::from(cur) * target_ms / elapsed_ms).clamp(f64::from(cur) * 0.5, f64::from(cur) * 2.0);
    (want.floor() as u32).clamp(lo, hi)
}
