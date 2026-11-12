//! A tiny software renderer for the hm08 head: textured, Lambert-lit,
//! z-buffered. Used to check the face detector and the fit against heads
//! whose true shape is known (the cloner's self-test and unit tests).

use crate::character::makehuman::obj::BaseMesh;
use crate::materials::texture::Texture;
use bevy::math::{Mat3, Vec2, Vec3};

/// An orthographic camera looking down -Z at `center`, turned by `yaw`
/// (radians, + turns the head to its left) and `pitch`; `scale` is pixels
/// per decimetre, the image is `size x size`.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub center: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub scale: f32,
    pub size: u32,
}

impl View {
    pub fn rot(&self) -> Mat3 {
        Mat3::from_rotation_x(self.pitch) * Mat3::from_rotation_y(self.yaw)
    }

    /// Image position (x right, y down) and depth (bigger = nearer).
    pub fn project(&self, p: Vec3) -> Vec3 {
        let q = self.rot() * (p - self.center);
        let h = self.size as f32 * 0.5;
        Vec3::new(h + q.x * self.scale, h - q.y * self.scale, q.z)
    }
}

fn edge(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    (b - a).perp_dot(p - a)
}

/// Renders the faces of `groups` (body skin from `skin`; eye helpers are
/// drawn as white eyeballs with a dark iris).
pub fn render(b: &BaseMesh, coords: &[Vec3], view: &View, skin: Option<&Texture>) -> image::RgbImage {
    let n = view.size;
    let mut img = image::RgbImage::from_pixel(n, n, image::Rgb([200, 205, 210]));
    let mut depth = vec![f32::NEG_INFINITY; (n * n) as usize];
    let light = view.rot().transpose() * Vec3::new(0.3, 0.5, 1.0).normalize();
    let eye_front: Vec<(u16, Vec3)> = ["helper-l-eye", "helper-r-eye"]
        .iter()
        .filter_map(|g| {
            let gi = b.groups.iter().position(|x| x == g)? as u16;
            let front = b.faces.iter().filter(|f| f.group == gi).flat_map(|f| f.verts().to_vec()).map(|v| coords[v as usize]).max_by(|a, c| a.z.total_cmp(&c.z))?;
            Some((gi, front))
        })
        .collect();
    for f in &b.faces {
        let g = b.groups[f.group as usize].as_str();
        let eye = eye_front.iter().find(|(gi, _)| *gi == f.group).map(|e| e.1);
        if g != "body" && eye.is_none() {
            continue;
        }
        let v = f.verts();
        let tris: &[[usize; 3]] = if v.len() == 4 { &[[0, 1, 2], [0, 2, 3]] } else { &[[0, 1, 2]] };
        for t in tris {
            let w = t.map(|k| coords[v[k] as usize]);
            let nrm = (w[1] - w[0]).cross(w[2] - w[0]).normalize_or_zero();
            let shade = 0.35 + 0.65 * nrm.dot(light).abs();
            let s = w.map(|p| view.project(p));
            let s2 = s.map(|p| p.truncate());
            let area = edge(s2[0], s2[1], s2[2]);
            if area.abs() < 1e-6 {
                continue;
            }
            let uv = t.map(|k| b.uvs.get(f.vt[k] as usize).copied().unwrap_or_default());
            let lo = s2[0].min(s2[1]).min(s2[2]).max(Vec2::ZERO).floor();
            let hi = s2[0].max(s2[1]).max(s2[2]).min(Vec2::splat(n as f32 - 1.0)).ceil();
            for y in lo.y as u32..=hi.y as u32 {
                for x in lo.x as u32..=hi.x as u32 {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let bc = [edge(s2[1], s2[2], p) / area, edge(s2[2], s2[0], p) / area, edge(s2[0], s2[1], p) / area];
                    if bc.iter().any(|&c| c < 0.0) {
                        continue;
                    }
                    let z = bc[0] * s[0].z + bc[1] * s[1].z + bc[2] * s[2].z;
                    let i = (y * n + x) as usize;
                    if z <= depth[i] {
                        continue;
                    }
                    depth[i] = z;
                    let col = if let Some(front) = eye {
                        let p3 = w[0] * bc[0] + w[1] * bc[1] + w[2] * bc[2];
                        let d = p3.distance(front);
                        if d < 0.045 { Vec3::splat(0.05) } else if d < 0.07 { Vec3::new(0.30, 0.20, 0.12) } else { Vec3::splat(0.92) }
                    } else {
                        let u = uv[0] * bc[0] + uv[1] * bc[1] + uv[2] * bc[2];
                        skin.map_or(Vec3::new(0.85, 0.68, 0.58), |t| t.sample(u).truncate())
                    };
                    let c = (col * shade).clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
                    img.put_pixel(x, y, image::Rgb([c.x as u8, c.y as u8, c.z as u8]));
                }
            }
        }
    }
    img
}
