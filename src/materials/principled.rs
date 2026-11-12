//! Principled BSDF extras on every Physical / Crystal material: random-walk
//! Subsurface Scattering, Anisotropic GGX, Clearcoat, Cloth Sheen,
//! Beer-Lambert volume absorption and chromatic Dispersion. All default to
//! off, so older scenes render exactly as before. Path-traced in F9; the
//! viewport shows clearcoat / anisotropy / diffuse transmission through
//! Bevy's StandardMaterial.

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Principled {
    /// SSS Weight (0..1): share of diffuse light that enters and scatters.
    pub sss_weight: f32,
    /// SSS Scatter Color (sRGB): the color light picks up inside.
    pub sss_color: [f32; 3],
    /// SSS Radius per channel (world units): mean free path of R, G, B.
    pub sss_radius: [f32; 3],
    /// SSS Scale: multiplies the radius (scene scale).
    pub sss_scale: f32,
    /// Anisotropy (0..1): stretches highlights along the tangent.
    pub anisotropy: f32,
    /// Anisotropy Rotation (degrees, 0..360) around the normal.
    pub aniso_rotation: f32,
    /// Clearcoat (0..1): lacquer layer weight.
    pub clearcoat: f32,
    pub clearcoat_roughness: f32,
    /// Cloth Sheen (0..1) and its tint toward the base color (0..1).
    pub sheen: f32,
    pub sheen_tint: f32,
    /// Beer-Lambert absorption inside transmissive surfaces: the color
    /// thick glass / water fades toward, and its density per world unit.
    pub absorption_color: [f32; 3],
    pub absorption_density: f32,
    /// Dispersion as an Abbe number (0 = off; 20 strong .. 90 faint).
    pub abbe: f32,
}

impl Default for Principled {
    fn default() -> Self {
        Self {
            sss_weight: 0.0,
            sss_color: [0.9, 0.55, 0.45],
            sss_radius: [1.0, 0.35, 0.2],
            sss_scale: 0.1,
            anisotropy: 0.0,
            aniso_rotation: 0.0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.03,
            sheen: 0.0,
            sheen_tint: 0.5,
            absorption_color: [0.8, 0.9, 1.0],
            absorption_density: 0.0,
            abbe: 0.0,
        }
    }
}

/// A named one-click look.
pub type Look = (&'static str, fn(&mut Principled));

/// One-click looks for the rollout.
pub const PRESETS: [Look; 6] = [
    ("Skin", |p| {
        (p.sss_weight, p.sss_color, p.sss_radius, p.sss_scale) = (0.6, [0.95, 0.6, 0.5], [1.0, 0.35, 0.2], 0.08);
    }),
    ("Marble / Jade", |p| {
        (p.sss_weight, p.sss_color, p.sss_radius, p.sss_scale) = (0.5, [0.85, 0.95, 0.85], [0.8, 0.9, 0.7], 0.2);
    }),
    ("Car Paint", |p| (p.clearcoat, p.clearcoat_roughness) = (1.0, 0.03)),
    ("Brushed Metal", |p| (p.anisotropy, p.aniso_rotation) = (0.8, 0.0)),
    ("Velvet", |p| (p.sheen, p.sheen_tint) = (1.0, 0.8)),
    ("Prism Glass", |p| (p.abbe, p.absorption_density, p.absorption_color) = (25.0, 0.3, [0.85, 0.95, 1.0])),
];

impl Principled {
    pub fn has_sss(&self) -> bool {
        self.sss_weight > 0.0
    }

    /// Anything beyond the plain Lambert + GGX + dielectric BSDF.
    pub fn active(&self) -> bool {
        self.has_sss() || self.anisotropy > 0.0 || self.clearcoat > 0.0 || self.sheen > 0.0 || self.absorption_density > 0.0 || self.abbe > 0.0
    }

    /// Single-scattering albedo giving multi-scatter color `c` (van de Hulst
    /// inversion, as in Cycles' random walk).
    pub fn walk_albedo(c: f32) -> f32 {
        let a = c.clamp(0.0, 0.999);
        let s = 4.09712 + 4.20863 * a - (9.59217 + 41.6808 * a + 17.7126 * a * a).sqrt();
        1.0 - s * s
    }

    /// Mean free path (world units) of the random walk.
    pub fn mean_free_path(&self) -> f32 {
        (self.sss_radius.iter().sum::<f32>() / 3.0 * self.sss_scale).max(1e-4)
    }

    /// Cauchy IOR for a wavelength (µm) from base `ior` (at 589 nm) and the
    /// Abbe number: `n(λ) = A + B / λ²`.
    pub fn ior_at(&self, ior: f32, lambda_um: f32) -> f32 {
        if self.abbe <= 0.0 {
            return ior;
        }
        let (f, c, d) = (0.4861f32, 0.6563f32, 0.5893f32);
        let b = (ior - 1.0) / (self.abbe * (1.0 / (f * f) - 1.0 / (c * c)));
        ior - b / (d * d) + b / (lambda_um * lambda_um)
    }
}

/// Representative wavelengths (µm) of the R, G, B channels.
pub const CHANNEL_LAMBDA: [f32; 3] = [0.61, 0.55, 0.465];
