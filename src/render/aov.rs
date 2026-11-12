//! AOV render passes kept by the CPU path tracer and shown by the F9
//! Frame Window's pass selector: Beauty, Direct, Indirect GI, Volumetric,
//! Specular / Reflection, Subsurface / Transmission, Albedo, World Normal,
//! Linear Depth, Cavity / AO and the Adaptive Sample Count heatmap.

use super::job::Region;
use super::post::Tone;
use bevy::math::{Vec3, Vec4};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Aov {
    #[default]
    Beauty,
    Direct,
    Indirect,
    Volumetric,
    Specular,
    Subsurface,
    Albedo,
    Normal,
    Depth,
    Ao,
    Samples,
}

impl Aov {
    pub const ALL: [Aov; 11] = [
        Aov::Beauty,
        Aov::Direct,
        Aov::Indirect,
        Aov::Volumetric,
        Aov::Specular,
        Aov::Subsurface,
        Aov::Albedo,
        Aov::Normal,
        Aov::Depth,
        Aov::Ao,
        Aov::Samples,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Aov::Beauty => "Beauty (Composite)",
            Aov::Direct => "Direct Lighting",
            Aov::Indirect => "Indirect GI",
            Aov::Volumetric => "Volumetric Godrays",
            Aov::Specular => "Specular / Reflection",
            Aov::Subsurface => "Subsurface / Transmission",
            Aov::Albedo => "Albedo (Unlit Texture)",
            Aov::Normal => "World Normal",
            Aov::Depth => "Linear Depth (Z)",
            Aov::Ao => "Cavity / Ambient Occlusion",
            Aov::Samples => "Adaptive Sample Count Heatmap",
        }
    }
}

/// One pixel's passes (radiance passes premultiplied by coverage).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AovPixel {
    pub direct: Vec3,
    pub indirect: Vec3,
    pub volume: Vec3,
    pub specular: Vec3,
    pub sss: Vec3,
    pub albedo: Vec3,
    pub normal: Vec3,
    /// Camera distance of the primary hit (infinite on a miss).
    pub depth: f32,
    pub ao: f32,
    pub samples: u32,
}

/// The passes of a finished render over `region`.
#[derive(Clone, Debug, PartialEq)]
pub struct AovBuffers {
    pub region: Region,
    pub px: Vec<AovPixel>,
}

/// Blue -> cyan -> green -> yellow -> red ramp for `t` in 0..1.
pub fn heat(t: f32) -> Vec3 {
    let stops = [Vec3::new(0.1, 0.1, 0.6), Vec3::new(0.0, 0.7, 0.9), Vec3::new(0.1, 0.8, 0.2), Vec3::new(1.0, 0.9, 0.1), Vec3::new(0.9, 0.1, 0.05)];
    let x = t.clamp(0.0, 1.0) * 4.0;
    let i = (x.floor() as usize).min(3);
    stops[i].lerp(stops[i + 1], x - i as f32)
}

impl AovBuffers {
    /// Display RGBA (sRGB) of one pass over the region, row-major.
    pub fn view(&self, aov: Aov, tone: &Tone) -> Vec<Vec4> {
        let finite = self.px.iter().map(|p| p.depth).filter(|d| d.is_finite());
        let (near, far) = finite.fold((f32::MAX, f32::MIN), |(a, b), d| (a.min(d), b.max(d)));
        let max_n = self.px.iter().map(|p| p.samples).max().unwrap_or(1).max(1) as f32;
        let srgb = |c: Vec3| c.to_array().map(crate::materials::shading::linear_to_srgb);
        self.px
            .iter()
            .map(|p| {
                let lit = |c: Vec3| tone.map(c).extend(1.0);
                match aov {
                    Aov::Beauty | Aov::Direct => lit(p.direct),
                    Aov::Indirect => lit(p.indirect),
                    Aov::Volumetric => lit(p.volume),
                    Aov::Specular => lit(p.specular),
                    Aov::Subsurface => lit(p.sss),
                    Aov::Albedo => Vec3::from_array(srgb(p.albedo)).extend(1.0),
                    Aov::Normal if p.depth.is_finite() => (p.normal * 0.5 + 0.5).extend(1.0),
                    Aov::Normal => Vec4::new(0.0, 0.0, 0.0, 1.0),
                    Aov::Depth if p.depth.is_finite() => Vec3::splat(1.0 - (p.depth - near) / (far - near).max(1e-6)).extend(1.0),
                    Aov::Depth => Vec4::new(0.0, 0.0, 0.0, 1.0),
                    Aov::Ao => Vec3::splat(p.ao).extend(1.0),
                    Aov::Samples => heat(p.samples as f32 / max_n).extend(1.0),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(depth: f32, samples: u32) -> AovPixel {
        AovPixel { direct: Vec3::new(0.5, 0.0, 0.0), albedo: Vec3::new(0.0, 1.0, 0.0), normal: Vec3::Z, depth, ao: 0.25, samples, ..Default::default() }
    }

    #[test]
    fn passes_extract_their_channel() {
        let b = AovBuffers { region: [0, 0, 3, 1], px: vec![px(1.0, 8), px(3.0, 64), px(f32::INFINITY, 16)] };
        let tone = Tone::default();
        let d = b.view(Aov::Direct, &tone);
        assert!(d[0].x > 0.0 && d[0].y == 0.0 && d[0].z == 0.0, "direct is red only: {}", d[0]);
        assert!((b.view(Aov::Albedo, &tone)[0] - Vec4::new(0.0, 1.0, 0.0, 1.0)).abs().max_element() < 1e-5);
        assert_eq!(b.view(Aov::Normal, &tone)[0], Vec4::new(0.5, 0.5, 1.0, 1.0));
        let z = b.view(Aov::Depth, &tone);
        assert!(z[0].x == 1.0 && z[1].x == 0.0 && z[2].x == 0.0, "near white, far / miss black");
        assert_eq!(b.view(Aov::Ao, &tone)[1].x, 0.25);
        let s = b.view(Aov::Samples, &tone);
        assert!(s[1].x > 0.8 && s[0].z > s[0].x, "most samples red, fewest blue: {} {}", s[1], s[0]);
        assert_eq!(Aov::ALL.len(), 11);
    }
}
