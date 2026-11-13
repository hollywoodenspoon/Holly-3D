//! World / Triplanar mapping: the material is projected along the X, Y and
//! Z axes and the three projections are blended by the surface normal, so
//! textures tile seamlessly on meshes that have no UVs (Settlement, Terrain,
//! GeoGen rocks, Battlefield rubble). The projection space is the object's
//! own space at world scale: `size` is in scene units and the texture
//! stays put when the object moves or turns.

use bevy::math::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mapping {
    /// The mesh's own UVs.
    #[default]
    Uv,
    Triplanar,
}

impl Mapping {
    pub const ALL: [Mapping; 2] = [Mapping::Uv, Mapping::Triplanar];
    pub fn label(self) -> &'static str {
        match self {
            Mapping::Uv => "UV Channel",
            Mapping::Triplanar => "World / Triplanar",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Projection {
    pub mapping: Mapping,
    /// Scene units covered by one repeat of the texture.
    pub size: f32,
    /// 0 = hard switch between the three sides, 1 = widest blend.
    pub blend: f32,
}

impl Default for Projection {
    fn default() -> Self {
        Self { mapping: Mapping::Uv, size: 1.0, blend: 0.3 }
    }
}

impl Projection {
    pub fn triplanar(size: f32) -> Self {
        Self { mapping: Mapping::Triplanar, size, ..Default::default() }
    }

    pub fn on(&self) -> bool {
        self.mapping == Mapping::Triplanar
    }

    fn scale(&self) -> f32 {
        1.0 / self.size.max(1e-4)
    }

    /// Blend weights of the X, Y and Z projections for normal `n` (sum 1).
    pub fn weights(&self, n: Vec3) -> Vec3 {
        let k = 1.0 + (1.0 - self.blend.clamp(0.0, 1.0)) * 15.0;
        let w = n.abs().powf(k);
        let sum = w.x + w.y + w.z;
        if sum > 1e-12 { w / sum } else { Vec3::Z }
    }

    /// UVs of point `p` in the X, Y and Z projections.
    pub fn planes(&self, p: Vec3) -> [Vec2; 3] {
        let q = p * self.scale();
        [Vec2::new(q.y, q.z), Vec2::new(q.x, q.z), Vec2::new(q.x, q.y)]
    }

    /// How a point moves with U and V in each projection.
    pub fn frames(&self) -> [(Vec3, Vec3); 3] {
        let s = self.size.max(1e-4);
        [(Vec3::Y * s, Vec3::Z * s), (Vec3::X * s, Vec3::Z * s), (Vec3::X * s, Vec3::Y * s)]
    }

    /// Box mapping (the side the normal faces most): what the GPU viewport
    /// shows, since it has one UV set per mesh.
    pub fn box_uv(&self, p: Vec3, n: Vec3) -> Vec2 {
        let a = n.abs();
        let k = if a.x >= a.y && a.x >= a.z {
            0
        } else if a.y >= a.z {
            1
        } else {
            2
        };
        self.planes(p)[k]
    }

    /// Blends `f(uv, plane)` over the projections that matter at `n`.
    pub fn blend<T: std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>>(&self, p: Vec3, n: Vec3, mut f: impl FnMut(Vec2, usize) -> T) -> T {
        let (w, uv) = (self.weights(n).to_array(), self.planes(p));
        // Renormalise over the sides that carry weight.
        let live: Vec<usize> = (0..3).filter(|&k| w[k] > 0.01).collect();
        let sum: f32 = live.iter().map(|&k| w[k]).sum();
        let mut out: Option<T> = None;
        for k in live {
            let v = f(uv[k], k) * (w[k] / sum);
            out = Some(match out {
                Some(o) => o + v,
                None => v,
            });
        }
        out.expect("a unit normal always has one side with weight")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_one_and_follow_the_normal() {
        let p = Projection::triplanar(2.0);
        for n in [Vec3::X, Vec3::new(1.0, 1.0, 0.0).normalize(), Vec3::new(0.2, -0.5, 0.9).normalize()] {
            let w = p.weights(n);
            assert!((w.x + w.y + w.z - 1.0).abs() < 1e-5, "{w}");
        }
        assert_eq!(p.weights(Vec3::NEG_Y), Vec3::Y, "a wall facing -Y uses the Y projection only");
        let edge = p.weights(Vec3::new(1.0, 1.0, 0.0).normalize());
        assert!((edge.x - 0.5).abs() < 1e-5 && edge.z < 1e-6, "a 45 degree edge is half and half");
        // A wider blend lets the minor side in sooner.
        let n = Vec3::new(0.9, 0.436, 0.0).normalize();
        assert!(Projection { blend: 1.0, ..p.clone() }.weights(n).y > p.weights(n).y);
    }

    #[test]
    fn coordinates_are_seamless_across_faces_and_repeat_every_size() {
        let p = Projection::triplanar(2.0);
        // A point on the edge between a +X wall and a +Y wall: both
        // projections share the coordinate along the edge (Z).
        let q = Vec3::new(1.0, 1.0, 0.7);
        let [x, y, _] = p.planes(q);
        assert_eq!(x.y, y.y, "V runs up the edge in both projections");
        // One repeat every `size` units.
        let a = p.box_uv(Vec3::new(0.25, 0.5, 0.0), Vec3::Z);
        let b = p.box_uv(Vec3::new(2.25, 0.5, 0.0), Vec3::Z);
        assert!((b - a - Vec2::X).length() < 1e-6, "{a} {b}");
        // Box mapping never stretches: a step along a wall moves the UV as far.
        let (u0, u1) = (p.box_uv(Vec3::new(3.0, 0.0, 0.0), Vec3::Y), p.box_uv(Vec3::new(3.0, 0.0, 1.0), Vec3::Y));
        assert!(((u1 - u0).length() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn blending_mixes_only_the_sides_that_face_the_normal() {
        let p = Projection::triplanar(1.0);
        let mut seen = vec![];
        let v = p.blend(Vec3::new(0.3, 0.6, 0.9), Vec3::Z, |uv, k| {
            seen.push(k);
            uv.x + uv.y
        });
        assert_eq!(seen, vec![2], "a floor samples the Z projection once");
        assert!((v - 0.9).abs() < 1e-6);
        let mix = p.blend(Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0).normalize(), |_, k| k as f32);
        assert!((mix - 0.5).abs() < 1e-5, "half X (0) and half Y (1)");
    }
}
