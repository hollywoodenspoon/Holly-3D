//! Bakes the photos into the character's skin texture (hm08 UVs). Every
//! head texel is put back on the fitted head, projected into each photo
//! through that photo's fitted camera, and kept where the photo sees it
//! (z-buffer), the mask says skin, and the surface faces the camera. The
//! light is taken out of each photo (`delight`), the photos are blended
//! (squarer views count more), the rest of the body skin is tinted to the
//! face's tone, and the face is feathered into it.

use super::delight::{self, luminance, to_linear, to_srgb, Sample};
use super::mask::Mask;
use super::math::Similarity;
use crate::character::makehuman::obj::{base, BaseMesh};
use crate::materials::texture::Texture;
use bevy::math::{Vec2, Vec3};

/// One photo for the bake: its camera (head space -> canonical image
/// frame), the picture at the shot's analysed size, its skin mask and how
/// much the view counts.
pub struct View<'a> {
    pub cam: Similarity,
    pub img: &'a image::RgbImage,
    pub mask: &'a Mask,
    pub weight: f32,
}

/// What the bake did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Baked {
    /// Texels taken from the photos (alpha-weighted).
    pub covered: f32,
    /// Per-channel tint applied to the rest of the skin.
    pub tint: [f32; 3],
}

/// A head texel: where it is in the texture, on the head, and its normal.
struct Texel {
    index: usize,
    pos: Vec3,
    normal: Vec3,
}

fn edge(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    (b - a).perp_dot(p - a)
}

/// Triangles of a face (corner indices).
fn tris(n: u8) -> &'static [[usize; 3]] {
    if n == 4 { &[[0, 1, 2], [0, 2, 3]] } else { &[[0, 1, 2]] }
}

fn vertex_normals(b: &BaseMesh, coords: &[Vec3], body: u16) -> Vec<Vec3> {
    let mut n = vec![Vec3::ZERO; coords.len()];
    for f in b.faces.iter().filter(|f| f.group == body) {
        let v = f.verts();
        for t in tris(f.n) {
            let w = t.map(|k| coords[v[k] as usize]);
            let fn_ = (w[1] - w[0]).cross(w[2] - w[0]);
            t.iter().for_each(|&k| n[v[k] as usize] += fn_);
        }
    }
    n.iter().map(|v| v.normalize_or_zero()).collect()
}

/// Every texel of the body faces within `reach` of the head centre.
fn head_texels(b: &BaseMesh, coords: &[Vec3], head: (Vec3, f32), size: (u32, u32), body: u16) -> Vec<Texel> {
    let normals = vertex_normals(b, coords, body);
    let (w, h) = (size.0 as f32, size.1 as f32);
    let reach = head.1 * 1.9;
    let mut out = Vec::new();
    for f in b.faces.iter().filter(|f| f.group == body) {
        let v = f.verts();
        if v.iter().all(|&i| coords[i as usize].distance(head.0) > reach) {
            continue;
        }
        for t in tris(f.n) {
            let uv = t.map(|k| b.uvs.get(f.vt[k] as usize).copied().unwrap_or_default());
            let px = uv.map(|u| Vec2::new(u.x * w, (1.0 - u.y) * h));
            let area = edge(px[0], px[1], px[2]);
            if area.abs() < 1e-9 {
                continue;
            }
            let lo = px[0].min(px[1]).min(px[2]).floor().max(Vec2::ZERO);
            let hi = px[0].max(px[1]).max(px[2]).ceil().min(Vec2::new(w - 1.0, h - 1.0));
            for y in lo.y as u32..=hi.y as u32 {
                for x in lo.x as u32..=hi.x as u32 {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let bc = [edge(px[1], px[2], p) / area, edge(px[2], px[0], p) / area, edge(px[0], px[1], p) / area];
                    if bc.iter().any(|&c| c < -1e-4) {
                        continue;
                    }
                    let at = |k: usize| coords[v[t[k]] as usize] * bc[k];
                    let nr = |k: usize| normals[v[t[k]] as usize] * bc[k];
                    out.push(Texel { index: (y * size.0 + x) as usize, pos: at(0) + at(1) + at(2), normal: (nr(0) + nr(1) + nr(2)).normalize_or_zero() });
                }
            }
        }
    }
    out
}

/// Image pixel (x right, y down) and nearness of a head-space point.
fn to_image(cam: &Similarity, p: Vec3) -> (Vec2, f32) {
    let c = cam.apply(p);
    (Vec2::new(c.x, -c.y), c.z)
}

/// The nearest head surface per pixel of `v`'s picture.
fn depth_buffer(b: &BaseMesh, coords: &[Vec3], v: &View, head: (Vec3, f32), body: u16) -> Vec<f32> {
    let (w, h) = v.img.dimensions();
    let mut depth = vec![f32::NEG_INFINITY; (w * h) as usize];
    for f in b.faces.iter().filter(|f| f.group == body) {
        let vs = f.verts();
        if vs.iter().all(|&i| coords[i as usize].distance(head.0) > head.1 * 2.5) {
            continue;
        }
        for t in tris(f.n) {
            let s = t.map(|k| to_image(&v.cam, coords[vs[k] as usize]));
            let area = edge(s[0].0, s[1].0, s[2].0);
            if area.abs() < 1e-6 {
                continue;
            }
            let lo = s[0].0.min(s[1].0).min(s[2].0).floor().max(Vec2::ZERO);
            let hi = s[0].0.max(s[1].0).max(s[2].0).ceil().min(Vec2::new(w as f32 - 1.0, h as f32 - 1.0));
            if lo.x > hi.x || lo.y > hi.y {
                continue;
            }
            for y in lo.y as u32..=hi.y as u32 {
                for x in lo.x as u32..=hi.x as u32 {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let bc = [edge(s[1].0, s[2].0, p) / area, edge(s[2].0, s[0].0, p) / area, edge(s[0].0, s[1].0, p) / area];
                    if bc.iter().all(|&c| c >= 0.0) {
                        let z = bc[0] * s[0].1 + bc[1] * s[1].1 + bc[2] * s[2].1;
                        let i = (y * w + x) as usize;
                        depth[i] = depth[i].max(z);
                    }
                }
            }
        }
    }
    depth
}

fn sample_img(img: &image::RgbImage, p: Vec2) -> Vec3 {
    let (w, h) = img.dimensions();
    let (x, y) = ((p.x - 0.5).clamp(0.0, w as f32 - 1.001), (p.y - 0.5).clamp(0.0, h as f32 - 1.001));
    let (x0, y0) = (x as u32, y as u32);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let g = |dx: u32, dy: u32| Vec3::from_array(img.get_pixel((x0 + dx).min(w - 1), (y0 + dy).min(h - 1)).0.map(to_linear));
    g(0, 0).lerp(g(1, 0), fx).lerp(g(0, 1).lerp(g(1, 1), fx), fy)
}

/// The samples one photo gives the head texels (index into `texels`).
fn samples_of(b: &BaseMesh, coords: &[Vec3], texels: &[Texel], v: &View, head: (Vec3, f32), body: u16) -> Vec<(usize, Sample)> {
    let depth = depth_buffer(b, coords, v, head, body);
    let (w, h) = v.img.dimensions();
    let tol = v.cam.scale * 0.04;
    let mut out = Vec::new();
    for (k, t) in texels.iter().enumerate() {
        let (p, z) = to_image(&v.cam, t.pos);
        if p.x < 0.0 || p.y < 0.0 || p.x >= w as f32 || p.y >= h as f32 {
            continue;
        }
        let facing = (v.cam.rot * t.normal).z;
        let m = v.mask.at(p);
        if facing < 0.2 || m <= 0.01 || depth[(p.y as u32 * w + p.x as u32) as usize] - z > tol {
            continue;
        }
        let f = ((facing - 0.2) / 0.6).clamp(0.0, 1.0);
        out.push((k, Sample { color: sample_img(v.img, p), normal: t.normal, weight: m * f * f * v.weight }));
    }
    out
}

/// Bakes `views` onto the head of `coords` over `base` (the current skin).
pub fn bake(coords: &[Vec3], head: (Vec3, f32), views: &[View], base_tex: &Texture) -> (Texture, Baked) {
    let b = base();
    let body = b.groups.iter().position(|g| g == "body").unwrap_or(0) as u16;
    let size = (base_tex.width, base_tex.height);
    let texels = head_texels(b, coords, head, size, body);
    let (mut col, mut wsum, mut cover) = (vec![Vec3::ZERO; texels.len()], vec![0.0f32; texels.len()], vec![0.0f32; texels.len()]);
    let (mut exposure, mut exp_w) = (0.0, 0.0);
    for v in views {
        let mut s = samples_of(b, coords, &texels, v, head, body);
        let mut plain: Vec<Sample> = s.iter().map(|e| e.1).collect();
        let Some(raw) = delight::delight(&mut plain) else { continue };
        let tw: f32 = plain.iter().map(|p| p.weight).sum();
        (exposure, exp_w) = (exposure + raw * tw, exp_w + tw);
        for ((k, _), p) in s.iter_mut().zip(plain) {
            let w2 = p.weight * p.weight;
            col[*k] += p.color * w2;
            wsum[*k] += w2;
            cover[*k] += p.weight;
        }
    }
    let exposure = if exp_w > 0.0 { exposure / exp_w } else { 0.0 };
    // Photo colour and coverage per texel.
    let photo: Vec<(Vec3, f32)> = col.iter().zip(&wsum).zip(&cover).map(|((c, w), a)| (if *w > 0.0 { *c / *w * exposure } else { Vec3::ZERO }, (a / 0.6).clamp(0.0, 1.0))).collect();
    let lin = |p: [u8; 4]| Vec3::new(to_linear(p[0]), to_linear(p[1]), to_linear(p[2]));
    // Tone match: the mean photo colour against the base skin under it.
    let (mut ps, mut bs, mut aw) = (Vec3::ZERO, Vec3::ZERO, 0.0);
    for (t, (c, a)) in texels.iter().zip(&photo) {
        if *a > 0.5 {
            (ps, bs, aw) = (ps + *c * *a, bs + lin(base_tex.data[t.index]) * *a, aw + *a);
        }
    }
    let tint = if aw > 0.0 && luminance(bs) > 1e-4 { (ps / bs.max(Vec3::splat(1e-4))).clamp(Vec3::splat(0.4), Vec3::splat(2.5)) } else { Vec3::ONE };
    let mut out = Texture { width: size.0, height: size.1, data: base_tex.data.iter().map(|p| tinted(*p, tint)).collect() };
    let mut covered = 0.0;
    for (t, (c, a)) in texels.iter().zip(&photo) {
        if *a <= 0.0 {
            continue;
        }
        let under = lin(base_tex.data[t.index]) * tint;
        let mixed = under.lerp(*c, *a);
        out.data[t.index] = [to_srgb(mixed.x), to_srgb(mixed.y), to_srgb(mixed.z), base_tex.data[t.index][3]];
        covered += a;
    }
    (out, Baked { covered, tint: tint.to_array() })
}

fn tinted(p: [u8; 4], t: Vec3) -> [u8; 4] {
    [to_srgb(to_linear(p[0]) * t.x), to_srgb(to_linear(p[1]) * t.y), to_srgb(to_linear(p[2]) * t.z), p[3]]
}
