//! Volume objects for the GPU kernel (`wgsl/volobj.wgsl`): every medium's
//! transform, majorant grid, per-cell puff lists, puffs and glow ramp in one
//! flat `f32` buffer (integers bit-cast), read by the same delta / ratio
//! tracking as the CPU (`crate::render::volumes`).
//!
//! Layout (word indices):
//! - `0..4`: volume count, Max Volume Bounces, Cloud Bounce Boost, shadow estimates.
//! - `4 + REC * i ..`: one record per volume (see [`record`]).
//! - then the shared data the records point into: cell majorants, cell
//!   `(start, count)` pairs, puff indices and puffs (`c.xyz, r, sq, low, w`),
//!   and voxel-grid values (`(z * ny + y) * nx + x`).

use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::volumes::VolumeObj;
use crate::render::volumes::density::Source;

/// Words per volume record.
pub const REC: usize = 96;
/// Glow ramp samples over density 0..=1 (the kernel interpolates).
pub const GLOW_STEPS: usize = 16;
/// Words per puff.
const PUFF: usize = 7;

/// The packed volume buffer (one dummy word when the scene has none).
pub fn pack(rs: &RenderScene, cfg: &RenderSettings) -> Vec<u8> {
    let list = &rs.volumes.list;
    let vs = &cfg.volumes;
    let mut w: Vec<f32> = vec![f32::from_bits(list.len() as u32), vs.max_bounces.clamp(1, 64) as f32, vs.boost.clamp(0.0, 1.0), vs.shadow_quality.estimates() as f32];
    w.resize(4 + REC * list.len(), 0.0);
    for (i, v) in list.iter().enumerate() {
        let rec = record(v, &mut w);
        w[4 + REC * i..4 + REC * (i + 1)].copy_from_slice(&rec);
    }
    if w.is_empty() {
        w.push(0.0);
    }
    w.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn word(u: usize) -> f32 {
    f32::from_bits(u as u32)
}

/// One volume's record; its grid and puffs are appended to `w`.
///
/// `0..16` world-to-local matrix (columns), `16..20` grid lo + extinction,
/// `20..24` grid hi + kind (0 uniform, 1 puffs, 2 voxels), `24..28` cells per axis +
/// majorant offset, `28..32` albedo + forward g, `32..36` back g, blend,
/// cell-list offset, puff offset, `36` glow on, `37..85` glow ramp (rgb),
/// then cloud erosion: `85` edge softness, `86` erosion, `87` wispiness,
/// `88` noise frequency, `89..92` noise offset, `92..94` base / top z,
/// `94` octaves. A voxel volume uses `85..88` for its voxels per axis and
/// `88` for their offset instead.
fn record(v: &VolumeObj, w: &mut Vec<f32>) -> [f32; REC] {
    let mut r = [0.0f32; REC];
    let g = &v.grid;
    r[..16].copy_from_slice(&v.inv.to_cols_array());
    r[16..19].copy_from_slice(&g.lo.to_array());
    r[19] = v.sigma();
    r[20..23].copy_from_slice(&g.hi.to_array());
    r[23] = word(match v.src {
        Source::Uniform => 0,
        Source::Puffs(_) => 1,
        Source::Grid(_) => 2,
    });
    r[24..27].copy_from_slice(&g.dims.map(word));
    r[27] = word(w.len());
    w.extend_from_slice(&g.max);
    let m = &v.mat;
    r[28..31].copy_from_slice(&m.albedo().to_array());
    (r[31], r[32], r[33]) = (m.g_forward, m.g_back, m.blend.clamp(0.0, 1.0));
    if let Source::Puffs(cloud) = &v.src {
        let (blobs, l) = (&cloud.blobs, &cloud.look);
        (r[85], r[86], r[87], r[88]) = (cloud.edge(), l.erosion.clamp(0.0, 1.0), l.wispiness.clamp(0.0, 1.0), cloud.inv_scale);
        r[89..92].copy_from_slice(&cloud.offset.to_array());
        (r[92], r[93], r[94]) = (cloud.z0, cloud.z1, cloud.octaves as f32);
        // Cell (start, count) pairs, then the indices they point at.
        let pairs = w.len();
        r[34] = word(pairs);
        w.resize(pairs + 2 * g.lists.len(), 0.0);
        for (c, list) in g.lists.iter().enumerate() {
            (w[pairs + 2 * c], w[pairs + 2 * c + 1]) = (word(w.len()), word(list.len()));
            w.extend(list.iter().map(|&k| word(k as usize)));
        }
        r[35] = word(w.len());
        for b in blobs {
            w.extend_from_slice(&[b.c.x, b.c.y, b.c.z, b.r, b.sq, b.low, b.w]);
        }
        debug_assert_eq!(w.len() - word_of(r[35]), PUFF * blobs.len());
    }
    if let Source::Grid(vox) = &v.src {
        r[85..88].copy_from_slice(&vox.dims.map(word));
        r[88] = word(w.len());
        w.extend_from_slice(&vox.data);
    }
    let glow: Vec<_> = (0..GLOW_STEPS).map(|k| m.glow(k as f32 / (GLOW_STEPS - 1) as f32)).collect();
    r[36] = f32::from(u8::from(glow.iter().any(|c| c.max_element() > 0.0)));
    for (k, c) in glow.iter().enumerate() {
        r[37 + 3 * k..40 + 3 * k].copy_from_slice(&c.to_array());
    }
    r
}

fn word_of(f: f32) -> usize {
    f.to_bits() as usize
}

/// Voxel-grid bytes above which a scene renders on the CPU instead.
pub const GRID_CAP_BYTES: u64 = 1 << 30;

/// Bytes of voxel grids the buffer would carry.
pub fn grid_bytes(rs: &RenderScene) -> u64 {
    rs.volumes.list.iter().map(|v| if let Source::Grid(g) = &v.src { g.data.len() as u64 * 4 } else { 0 }).sum()
}

/// The scene's voxel grids are too big for the GPU buffer.
pub fn needs_cpu(rs: &RenderScene) -> bool {
    grid_bytes(rs) > GRID_CAP_BYTES
}
