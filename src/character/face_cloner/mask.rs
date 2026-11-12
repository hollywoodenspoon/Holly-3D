//! Which pixels of a photo are face skin, as a soft 0..1 mask: the face
//! outline of the landmarks, minus the eyes and the mouth opening, with
//! feathered edges, times a skin colour model learnt on the cheeks, nose
//! and chin (drops hair, brows, beard stubble and glasses frames that
//! are clearly not skin); MobileSAM (when installed) then cuts out hair,
//! hands and background of skin-like colour inside that outline.

use super::frames::Shot;
use super::sam;
use bevy::math::Vec2;

/// MediaPipe's face oval, in order around the face.
pub const FACE_OVAL: [usize; 36] = [10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288, 397, 365, 379, 378, 400, 377, 152, 148, 176, 149, 150, 136, 172, 58, 132, 93, 234, 127, 162, 21, 54, 103, 67, 109];
/// The eye openings (image left eye, image right eye).
pub const EYES: [[usize; 16]; 2] = [
    [33, 7, 163, 144, 145, 153, 154, 155, 133, 173, 157, 158, 159, 160, 161, 246],
    [263, 249, 390, 373, 374, 380, 381, 382, 362, 398, 384, 385, 386, 387, 388, 466],
];
/// The inner lip line (the mouth opening).
pub const INNER_LIPS: [usize; 20] = [78, 95, 88, 178, 87, 14, 317, 402, 318, 324, 308, 415, 310, 311, 312, 13, 82, 81, 80, 191];

/// A soft mask, row-major `w x h`.
#[derive(Clone, Debug)]
pub struct Mask {
    pub w: u32,
    pub h: u32,
    pub data: Vec<f32>,
    /// MobileSAM refined it.
    pub sam: bool,
}

impl Mask {
    /// Bilinear lookup at image pixel `p` (0 outside).
    pub fn at(&self, p: Vec2) -> f32 {
        let (x, y) = (p.x - 0.5, p.y - 0.5);
        if x < 0.0 || y < 0.0 || x >= self.w as f32 - 1.0 || y >= self.h as f32 - 1.0 {
            return 0.0;
        }
        let (x0, y0) = (x as usize, y as usize);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let g = |dx: usize, dy: usize| self.data[(y0 + dy) * self.w as usize + x0 + dx];
        (g(0, 0) * (1.0 - fx) + g(1, 0) * fx) * (1.0 - fy) + (g(0, 1) * (1.0 - fx) + g(1, 1) * fx) * fy
    }
}

/// Even-odd scanline fill of `poly` (pixel units) into `out` with `value`.
pub fn fill(out: &mut [f32], w: u32, h: u32, poly: &[Vec2], value: f32) {
    for y in 0..h {
        let fy = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();
        for (i, a) in poly.iter().enumerate() {
            let b = poly[(i + 1) % poly.len()];
            if (a.y <= fy) != (b.y <= fy) {
                xs.push(a.x + (fy - a.y) / (b.y - a.y) * (b.x - a.x));
            }
        }
        xs.sort_by(f32::total_cmp);
        for pair in xs.chunks(2) {
            if let [x0, x1] = pair {
                let (a, b) = ((x0 - 0.5).ceil().max(0.0) as u32, ((x1 - 0.5).floor() as i64).min(w as i64 - 1));
                for x in a as i64..=b {
                    out[(y * w) as usize + x as usize] = value;
                }
            }
        }
    }
}

/// Separable box blur of radius `r` pixels.
pub fn blur(data: &mut [f32], w: usize, h: usize, r: usize) {
    if r == 0 {
        return;
    }
    let mut tmp = vec![0.0f32; data.len()];
    let pass = |src: &[f32], dst: &mut [f32], len: usize, count: usize, at: &dyn Fn(usize, usize) -> usize| {
        for line in 0..count {
            let mut sum = 0.0;
            let get = |i: i64| src[at(line, i.clamp(0, len as i64 - 1) as usize)];
            for i in -(r as i64)..=r as i64 {
                sum += get(i);
            }
            for i in 0..len {
                dst[at(line, i)] = sum / (2 * r + 1) as f32;
                sum += get(i as i64 + r as i64 + 1) - get(i as i64 - r as i64);
            }
        }
    };
    pass(data, &mut tmp, w, h, &|line, i| line * w + i);
    pass(&tmp, data, h, w, &|line, i| i * w + line);
}

/// The landmark mask of `shot` at its analysed size: soft, edges faded
/// inward over a few percent of the eye span.
pub fn landmark_mask(shot: &Shot) -> Mask {
    let (w, h) = shot.size;
    let pt = |i: usize| {
        let p = shot.point(i);
        Vec2::new(p.x, p.y)
    };
    let mut data = vec![0.0f32; (w * h) as usize];
    fill(&mut data, w, h, &FACE_OVAL.map(pt), 1.0);
    // Eyes and the mouth opening, grown a little so lids and lips stay out.
    for poly in [EYES[0].to_vec(), EYES[1].to_vec(), INNER_LIPS.to_vec()] {
        let p: Vec<Vec2> = poly.into_iter().map(pt).collect();
        let c = p.iter().copied().sum::<Vec2>() / p.len() as f32;
        fill(&mut data, w, h, &p.iter().map(|&q| c + (q - c) * 1.35).collect::<Vec<_>>(), 0.0);
    }
    let r = (shot.eye_span() * 0.06).round().max(1.0) as usize;
    blur(&mut data, w as usize, h as usize, r);
    data.iter_mut().for_each(|v| *v = (*v * 2.0 - 1.0).clamp(0.0, 1.0));
    Mask { w, h, data, sam: false }
}

/// Where skin is sure to show (cheeks, nose, chin): the colour model is learnt there.
const SURE_SKIN: [usize; 6] = [50, 280, 4, 199, 205, 425];
/// Where hair or a hat may cover the face (forehead, temples).
const MAYBE_HAIR: [usize; 7] = [151, 108, 337, 67, 297, 54, 284];

/// Skin colour of one photo: mean chroma and its spread, mean brightness.
pub struct SkinColor {
    chroma: Vec2,
    spread: Vec2,
    lum: f32,
}

fn chroma_lum(c: [u8; 3]) -> (Vec2, f32) {
    let (r, g, b) = (c[0] as f32, c[1] as f32, c[2] as f32);
    let sum = (r + g + b).max(1.0);
    (Vec2::new(r / sum, g / sum), (0.299 * r + 0.587 * g + 0.114 * b) / 255.0)
}

impl SkinColor {
    /// Learns the skin colour from small disks around the sure-skin points.
    pub fn learn(shot: &Shot, img: &image::RgbImage) -> Option<Self> {
        let r = (shot.eye_span() * 0.05).max(2.0) as i64;
        let mut px = Vec::new();
        for &i in &SURE_SKIN {
            let p = shot.point(i);
            for dy in -r..=r {
                for dx in -r..=r {
                    let (x, y) = (p.x as i64 + dx, p.y as i64 + dy);
                    if dx * dx + dy * dy <= r * r && x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
                        px.push(chroma_lum(img.get_pixel(x as u32, y as u32).0));
                    }
                }
            }
        }
        if px.len() < 20 {
            return None;
        }
        let n = px.len() as f32;
        let chroma = px.iter().map(|p| p.0).sum::<Vec2>() / n;
        let var = px.iter().map(|p| (p.0 - chroma) * (p.0 - chroma)).sum::<Vec2>() / n;
        let lum = px.iter().map(|p| p.1).sum::<f32>() / n;
        Some(Self { chroma, spread: (var.max(Vec2::splat(1e-6)).map(f32::sqrt) * 2.5).max(Vec2::splat(0.012)), lum })
    }

    /// How skin-like a pixel is (0..1): close in chroma, not much darker or brighter.
    pub fn likeness(&self, c: [u8; 3]) -> f32 {
        let (ch, l) = chroma_lum(c);
        let d = ((ch - self.chroma) / self.spread).length_squared();
        let k = l / self.lum.max(1e-3);
        let bright = ((k - 0.35) / 0.2).clamp(0.0, 1.0) * ((2.2 - k) / 0.4).clamp(0.0, 1.0);
        (-0.5 * d).exp() * bright
    }
}

/// The landmark mask times the skin colour model, cut down by MobileSAM
/// when it is installed.
pub fn skin_mask(shot: &Shot, img: &image::RgbImage) -> Mask {
    let mut m = landmark_mask(shot);
    let Some(skin) = SkinColor::learn(shot, img) else { return m };
    let mut like: Vec<f32> = img.pixels().map(|p| skin.likeness(p.0)).collect();
    if like.len() != m.data.len() {
        return m;
    }
    blur(&mut like, m.w as usize, m.h as usize, 1);
    m.data.iter_mut().zip(&like).for_each(|(a, b)| *a *= (b * 1.6).min(1.0));
    let Some(net) = sam::model() else { return m };
    let pt = |i: usize| {
        let p = shot.point(i);
        Vec2::new(p.x, p.y)
    };
    let px = |p: Vec2| img.get_pixel((p.x as u32).min(img.width() - 1), (p.y as u32).min(img.height() - 1)).0;
    let oval: Vec<Vec2> = FACE_OVAL.iter().map(|&i| pt(i)).collect();
    let (lo, hi) = oval.iter().fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(a, b), &p| (a.min(p), b.max(p)));
    let grow = (hi - lo) * 0.08;
    let (lo, hi) = ((lo - grow).max(Vec2::ZERO), (hi + grow).min(Vec2::new(m.w as f32 - 1.0, m.h as f32 - 1.0)));
    // Skin: the sure points plus forehead points of skin colour; not skin:
    // forehead points of another colour (hair) and above the hairline.
    let mut fg: Vec<[f32; 2]> = SURE_SKIN.iter().map(|&i| pt(i).to_array()).collect();
    let mut bg: Vec<[f32; 2]> = Vec::new();
    for &i in &MAYBE_HAIR {
        let p = pt(i);
        if skin.likeness(px(p)) > 0.5 { fg.push(p.to_array()) } else { bg.push(p.to_array()) }
    }
    let up = pt(10) + (pt(10) - pt(152)) * 0.22;
    if up.y > 0.0 {
        bg.push(up.to_array());
    }
    if let Some(mut prob) = sam::segment(&*net, img, &fg, &bg, [lo.x, lo.y, hi.x, hi.y]).filter(|p| p.len() == m.data.len()) {
        blur(&mut prob, m.w as usize, m.h as usize, 2);
        // A mask that drops most of the face is a failed prompt: keep the landmarks.
        let (kept, total) = m.data.iter().zip(&prob).fold((0.0, 0.0), |(k, t), (a, b)| (k + a * b.clamp(0.0, 1.0), t + a));
        if total > 0.0 && kept / total > 0.45 {
            m.data.iter_mut().zip(&prob).for_each(|(a, b)| *a *= ((b - 0.3) / 0.4).clamp(0.0, 1.0));
            m.sam = true;
        }
    }
    m
}
