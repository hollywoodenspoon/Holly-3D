//! 3ds Max-style light objects (Omni, Target / Free Spot, Target / Free
//! Directional, Skylight) and the Default Modeling Lights headlight rig.
//! Pure data + math; the Bevy side lives in `viewport::light_sync`.

use crate::scene::ObjId;
use bevy::math::{Mat3, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightKind {
    Omni,
    TargetSpot,
    FreeSpot,
    TargetDirect,
    FreeDirect,
    Skylight,
}

impl LightKind {
    pub const ALL: [LightKind; 6] = [Self::Omni, Self::TargetSpot, Self::FreeSpot, Self::TargetDirect, Self::FreeDirect, Self::Skylight];

    pub fn label(self) -> &'static str {
        match self {
            Self::Omni => "Omni",
            Self::TargetSpot => "Target Spot",
            Self::FreeSpot => "Free Spot",
            Self::TargetDirect => "Target Direct",
            Self::FreeDirect => "Free Direct",
            Self::Skylight => "Skylight",
        }
    }
    pub fn has_target(self) -> bool {
        matches!(self, Self::TargetSpot | Self::TargetDirect)
    }
    pub fn is_spot(self) -> bool {
        matches!(self, Self::TargetSpot | Self::FreeSpot)
    }
    pub fn is_directional(self) -> bool {
        matches!(self, Self::TargetDirect | Self::FreeDirect)
    }
    /// Omni and spots fall off with distance.
    pub fn has_range(self) -> bool {
        matches!(self, Self::Omni | Self::TargetSpot | Self::FreeSpot)
    }
}

/// Kelvin presets for the color swatch dropdown.
pub const KELVIN_PRESETS: [(&str, f32); 8] = [
    ("Candle (1900K)", 1900.0),
    ("Tungsten (2700K)", 2700.0),
    ("Halogen (3200K)", 3200.0),
    ("Fluorescent (4100K)", 4100.0),
    ("Daylight (5600K)", 5600.0),
    ("Neutral (6500K)", 6500.0),
    ("Overcast (7500K)", 7500.0),
    ("Blue Sky (10000K)", 10000.0),
];

/// Black-body color (sRGB-ish, normalised so the brightest channel is 1).
pub fn kelvin_to_rgb(k: f32) -> [f32; 3] {
    // Tanner Helland's fit, in hundreds of kelvin.
    let t = k.clamp(1000.0, 40000.0) / 100.0;
    let r = if t <= 66.0 { 255.0 } else { 329.698_73 * (t - 60.0).powf(-0.133_204_76) };
    let g = if t <= 66.0 { 99.470_8 * t.ln() - 161.119_57 } else { 288.122_16 * (t - 60.0).powf(-0.075_514_85) };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_73 * (t - 10.0).ln() - 305.044_8
    };
    let c = [r, g, b].map(|x: f32| x.clamp(0.0, 255.0) / 255.0);
    let m = c.iter().cloned().fold(1e-6, f32::max);
    c.map(|x| x / m)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Light {
    pub kind: LightKind,
    pub on: bool,
    pub cast_shadows: bool,
    pub color: [f32; 3],
    pub multiplier: f32,
    /// Far attenuation end (world units) for Omni / Spot; `use_atten` off
    /// means the light reaches the whole scene.
    pub use_atten: bool,
    pub atten_start: f32,
    pub atten_end: f32,
    pub show_atten: bool,
    /// Spot cone angles in degrees (full angle, like 3ds Max).
    pub hotspot: f32,
    pub falloff: f32,
    /// Directional beam radius (display + F9 renderer).
    pub beam_radius: f32,
    pub shadow_bias: f32,
    /// Shadow softness / light radius.
    pub softness: f32,
    pub shadow_color: [f32; 3],
    pub shadow_density: f32,
    /// Target helper object (Target Spot / Target Direct).
    pub target: Option<ObjId>,
    /// Created by an Environment lighting preset (replaced by the next one).
    pub from_preset: bool,
}

impl Light {
    pub fn new(kind: LightKind) -> Self {
        Self {
            kind,
            on: true,
            cast_shadows: !matches!(kind, LightKind::Skylight),
            color: [1.0; 3],
            multiplier: 1.0,
            use_atten: false,
            atten_start: 0.0,
            atten_end: 20.0,
            show_atten: false,
            hotspot: 43.0,
            falloff: 45.0,
            beam_radius: 2.0,
            shadow_bias: 0.02,
            softness: 0.0,
            shadow_color: [0.0; 3],
            shadow_density: 1.0,
            target: None,
            from_preset: false,
        }
    }

    /// Linear RGB times multiplier (what the renderers use).
    pub fn radiance(&self) -> Vec3 {
        if !self.on {
            return Vec3::ZERO;
        }
        Vec3::from_array(self.color.map(crate::materials::shading::srgb_to_linear)) * self.multiplier
    }

    /// Keeps hotspot inside falloff (3ds Max pushes one against the other).
    pub fn clamp_cone(&mut self) {
        self.falloff = self.falloff.clamp(1.0, 179.0);
        self.hotspot = self.hotspot.clamp(0.5, self.falloff - 0.5);
    }

    /// Viewport range: attenuation end, or "the whole scene".
    pub fn range(&self) -> f32 {
        if self.use_atten { self.atten_end.max(0.01) } else { 200.0 }
    }
}

/// Rotation pointing a light's emission axis (local -Z, like 3ds Max) from
/// `from` towards `to`, keeping world Z as up when possible.
pub fn aim(from: Vec3, to: Vec3) -> Quat {
    let dir = (to - from).normalize_or_zero();
    if dir == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    let up = if dir.cross(Vec3::Z).length_squared() < 1e-6 { Vec3::Y } else { Vec3::Z };
    // Camera-style basis: X right, Y up, Z back (so -Z looks along `dir`).
    let right = dir.cross(up).normalize();
    let up = right.cross(dir);
    Quat::from_mat3(&Mat3::from_cols(right, up, -dir))
}

/// Emission direction of a light with node rotation `rot`.
pub fn emit_dir(rot: Quat) -> Vec3 {
    rot * Vec3::NEG_Z
}

/// One light of the Default Modeling Lights rig (world space, linear color).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigLight {
    /// Direction the light travels (towards the scene).
    pub dir: Vec3,
    pub color: Vec3,
    /// Relative to a standard key (1.0).
    pub strength: f32,
}

/// 3ds Max 2-light headlight rig for a camera: a key slightly above and to
/// the left of the eye, plus a cooler fill from the opposite side.
pub fn headlight_rig(cam_forward: Vec3, cam_right: Vec3, cam_up: Vec3) -> [RigLight; 2] {
    let key = (cam_forward + cam_right * 0.35 - cam_up * 0.45).normalize();
    let fill = (cam_forward * 0.3 - cam_right * 0.8 + cam_up * 0.3).normalize();
    [
        RigLight { dir: key, color: Vec3::new(1.0, 0.97, 0.92), strength: 1.0 },
        RigLight { dir: fill, color: Vec3::new(0.78, 0.85, 1.0), strength: 0.4 },
    ]
}
