//! The bundled MediaPipe Face Landmarker (Google LLC, Apache-2.0), run by
//! EffectCraft's pure-Rust TFLite interpreter. EffectCraft's tracker keeps
//! only the 2D points; the cloner runs the mesh model a second time on the
//! same crop to keep each point's depth too.

use bevy::math::Vec3;
use effectcraft_segment::face::{crop, Border, FaceModel, Roi};
use effectcraft_segment::mediapipe::{FaceLandmarker, MESH};
use effectcraft_segment::tflite::Model;
use std::sync::OnceLock;

/// MediaPipe face mesh points (468 mesh + 10 iris).
pub const POINTS: usize = 478;
/// Points of the canonical face mesh (no iris).
pub const MESH_POINTS: usize = 468;
const MESH_SIZE: usize = 256;
const ROI_SCALE: f32 = 1.5;
/// Outer eye corners (image left, image right): the crop is levelled on them.
pub const EYE_LINE: [usize; 2] = [33, 263];

pub fn task_bytes() -> &'static [u8] {
    include_bytes!("../../../assets/face/face_landmarker.task")
}

fn task_file(name: &str) -> Option<Vec<u8>> {
    crate::assets::zip::entries(task_bytes()).ok()?.into_iter().find(|e| e.name == name)?.read().ok()
}

struct Models {
    landmarker: FaceLandmarker,
    mesh: Model,
}

fn models() -> Option<&'static Models> {
    static CELL: OnceLock<Option<Models>> = OnceLock::new();
    CELL.get_or_init(|| {
        let landmarker = FaceLandmarker::from_task(task_bytes()).ok()?;
        let mesh = Model::read(&task_file(MESH)?).ok()?;
        Some(Models { landmarker, mesh })
    })
    .as_ref()
}

/// True when the bundled model loads.
pub fn available() -> bool {
    models().is_some()
}

/// A face found in one image: 478 points in image pixels (x right, y
/// down) with depth `z` in the same pixel scale (negative = toward the
/// camera), and the model's confidence.
#[derive(Clone, Debug, PartialEq)]
pub struct Detected {
    pub points: Vec<Vec3>,
    pub score: f32,
}

/// The crop MediaPipe uses for a face: its bounds grown 1.5x, levelled on the eyes.
pub fn roi_of(points: &[[f32; 2]]) -> Option<Roi> {
    let (a, b) = (points.get(EYE_LINE[0])?, points.get(EYE_LINE[1])?);
    let mut bb = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
    for p in points {
        bb = [bb[0].min(p[0]), bb[1].min(p[1]), bb[2].max(p[0]), bb[3].max(p[1])];
    }
    Some(Roi { center: [(bb[0] + bb[2]) / 2.0, (bb[1] + bb[3]) / 2.0], size: (bb[2] - bb[0]).max(bb[3] - bb[1]) * ROI_SCALE, angle: (b[1] - a[1]).atan2(b[0] - a[0]) })
}

/// Runs the mesh model in `roi` and returns the points with depth.
fn mesh_3d(m: &Models, rgb: &[[f32; 3]], w: usize, h: usize, roi: &Roi) -> Option<Vec<Vec3>> {
    let x = crop(rgb, w, h, roi, MESH_SIZE, [0.0, 1.0], Border::Replicate);
    let out = m.mesh.run(&[&x]).ok()?;
    let s = MESH_SIZE as f32;
    let pts: Vec<Vec3> = out
        .first()?
        .as_chunks::<3>()
        .0
        .iter()
        .take(POINTS)
        .map(|p| {
            let [x, y] = roi.to_frame(p[0] / s, p[1] / s);
            Vec3::new(x, y, p[2] / s * roi.size)
        })
        .collect();
    (pts.len() == POINTS).then_some(pts)
}

/// The most confident face in an RGB (0..1) image, or None.
pub fn detect(rgb: &[[f32; 3]], w: usize, h: usize) -> Option<Detected> {
    let m = models()?;
    let face = m.landmarker.find(rgb, w, h, [0.0, 0.0, w as f32, h as f32]).ok()??;
    let roi = roi_of(&face.points)?;
    Some(Detected { points: mesh_3d(m, rgb, w, h, &roi)?, score: face.score })
}

/// `detect` on an `image` crate picture.
pub fn detect_image(img: &image::RgbImage) -> Option<Detected> {
    let rgb: Vec<[f32; 3]> = img.pixels().map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]).collect();
    detect(&rgb, img.width() as usize, img.height() as usize)
}

/// MediaPipe's canonical face mesh (468 points, centimetres, x toward the
/// subject's left, y up, z toward the viewer), read from the bundle's
/// geometry metadata.
pub fn canonical() -> &'static [Vec3] {
    static CELL: OnceLock<Vec<Vec3>> = OnceLock::new();
    CELL.get_or_init(|| task_file("geometry_pipeline_metadata_landmarks.binarypb").map(|b| parse_canonical(&b)).unwrap_or_default())
}

fn varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let (mut r, mut s) = (0u64, 0);
    loop {
        let c = *b.get(*i)?;
        *i += 1;
        r |= ((c & 0x7f) as u64) << s;
        s += 7;
        if c < 0x80 {
            return Some(r);
        }
    }
}

/// `GeometryPipelineMetadata.canonical_mesh` (field 1): a `Mesh3d` whose
/// `vertex_buffer` (field 3) holds x, y, z, u, v per vertex.
fn parse_canonical(b: &[u8]) -> Vec<Vec3> {
    let mut i = 0;
    let (Some(10), Some(n)) = (varint(b, &mut i), varint(b, &mut i)) else { return vec![] }; // field 1, length-delimited
    let Some(mesh) = b.get(i..i + n as usize) else { return vec![] };
    let (mut j, mut floats) = (0, Vec::new());
    while j < mesh.len() {
        let Some(key) = varint(mesh, &mut j) else { break };
        match key & 7 {
            0 => {
                varint(mesh, &mut j);
            }
            5 => {
                let Some(f) = mesh.get(j..j + 4) else { break };
                if key >> 3 == 3 {
                    floats.push(f32::from_le_bytes(f.try_into().unwrap_or_default()));
                }
                j += 4;
            }
            2 => {
                let Some(len) = varint(mesh, &mut j) else { break };
                let Some(data) = mesh.get(j..j + len as usize) else { break };
                if key >> 3 == 3 {
                    floats.extend(data.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)));
                }
                j += len as usize;
            }
            _ => break,
        }
    }
    floats.as_chunks::<5>().0.iter().map(|v| Vec3::new(v[0], v[1], v[2])).collect()
}
