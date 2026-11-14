//! Reads one float grid of an OpenVDB file (via the `vdb-rs` crate) into a
//! dense voxel grid in Ilex's Z-up space. Each voxel of the result averages
//! a block of file voxels (a box filter), so a huge simulation fits the
//! Max Resolution budget; values are normalised so the peak is 1 and the
//! peak is kept for the material. Results are cached by file, grid,
//! up axis, resolution and modification time.

use super::UpAxis;
use crate::render::volumes::density::Voxels;
use bevy::math::Vec3;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// "VDB " as OpenVDB writes it (a little-endian 64-bit magic number).
const MAGIC: u64 = 0x5644_4220;
/// Cached grids kept at most (each up to a few hundred MB).
const CACHE_MAX: usize = 6;

/// A grid ready for the volume renderers, in file units (before Scale).
#[derive(Debug)]
pub struct GridData {
    pub vox: Arc<Voxels>,
    pub lo: Vec3,
    pub hi: Vec3,
    /// Largest averaged value before normalising.
    pub peak: f32,
    /// Active file voxels and tiles read.
    pub active: usize,
}

type Key = (PathBuf, String, UpAxis, u32, Option<SystemTime>);

static GRIDS: Mutex<Vec<(Key, Arc<GridData>)>> = Mutex::new(Vec::new());
#[allow(clippy::type_complexity)]
static NAMES: Mutex<Option<HashMap<(PathBuf, Option<SystemTime>), Result<Vec<String>, String>>>> = Mutex::new(None);

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn open(p: &Path) -> Result<vdb_rs::VdbReader<BufReader<File>>, String> {
    let mut f = File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic).map_err(|_| format!("{} is not a VDB file", p.display()))?;
    if u64::from_le_bytes(magic) != MAGIC {
        return Err(format!("{} is not a VDB file", p.display()));
    }
    let f = File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let r = crate::diagnostics::hook::contain(|| vdb_rs::VdbReader::new(BufReader::new(f)).map_err(|e| e.to_string()));
    r.map_err(|e| format!("Unsupported VDB: {e}"))?.map_err(|e: String| format!("Could not read {}: {e}", p.display()))
}

/// The scalar float grids in the file, sorted by name (cached).
pub fn float_grids(path: &str) -> Result<Vec<String>, String> {
    let p = PathBuf::from(path);
    let key = (p.clone(), mtime(&p));
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let map = names.get_or_insert_with(HashMap::new);
    if let Some(r) = map.get(&key) {
        return r.clone();
    }
    let r = open(&p).map(|r| {
        let mut v: Vec<String> = r.grid_descriptors.values().filter(|g| is_float(&g.grid_type) && g.instance_parent.is_empty()).map(|g| g.name.clone()).collect();
        v.sort();
        v
    });
    map.insert(key, r.clone());
    r
}

fn is_float(grid_type: &str) -> bool {
    let t = grid_type.to_ascii_lowercase();
    t.contains("float") && !t.contains("vec")
}

/// Grid `name` of the VDB at `path`, resampled to at most `res` voxels
/// along its longest axis (cached).
pub fn grid(path: &str, name: &str, up: UpAxis, res: u32) -> Result<Arc<GridData>, String> {
    let p = PathBuf::from(path);
    let key: Key = (p.clone(), name.to_string(), up, res, mtime(&p));
    if let Some((_, g)) = GRIDS.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| *k == key) {
        return Ok(g.clone());
    }
    let mut r = open(&p)?;
    if !float_grids(path)?.iter().any(|g| g == name) {
        return Err(format!("{} has no float grid named \"{name}\"", p.display()));
    }
    let g = crate::diagnostics::hook::contain(|| r.read_grid::<f32>(name).map_err(|e| e.to_string()))
        .map_err(|e| format!("Unsupported VDB grid \"{name}\": {e}"))?
        .map_err(|e| format!("Could not read grid \"{name}\": {e}"))?;
    let (vs, t) = match g.transform {
        vdb_rs::Map::UniformScaleMap { voxel_size, .. } => (voxel_size, [0.0; 3]),
        vdb_rs::Map::ScaleTranslateMap { voxel_size, translation, .. } => (voxel_size, [translation.x, translation.y, translation.z]),
    };
    let entries = || g.iter().filter(|(_, v, _)| *v > 0.0).map(|(c, v, l)| ([c.x as i64, c.y as i64, c.z as i64], l.scale() as i64, v));
    let data = Arc::new(resample(entries, [vs.x, vs.y, vs.z], t, up, res).ok_or_else(|| format!("Grid \"{name}\" is empty"))?);
    let mut cache = GRIDS.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= CACHE_MAX {
        cache.remove(0);
    }
    cache.push((key, data.clone()));
    Ok(data)
}

/// Turns file index boxes (`corner`, `size`, value) into a dense,
/// box-filtered, normalised grid. `entries` is walked twice (bounds, then
/// values). `vs` / `t`: voxel size and translation of the file's
/// index-to-world map.
pub fn resample<I: Iterator<Item = ([i64; 3], i64, f32)>>(entries: impl Fn() -> I, vs: [f64; 3], t: [f64; 3], up: UpAxis, res: u32) -> Option<GridData> {
    // Ilex fine-index boxes [a, a + s) per axis; Y-up files turn so their
    // +Y points up (+Z) and their +Z points towards -Y.
    let to_ilex = move |c: [i64; 3], s: i64| match up {
        UpAxis::Z => c,
        UpAxis::Y => [c[0], -c[2] - s, c[1]],
    };
    let (mut lo, mut hi) = ([i64::MAX; 3], [i64::MIN; 3]);
    let mut active = 0;
    for (c, s, _) in entries() {
        let a = to_ilex(c, s);
        for k in 0..3 {
            lo[k] = lo[k].min(a[k]);
            hi[k] = hi[k].max(a[k] + s);
        }
        active += 1;
    }
    if active == 0 {
        return None;
    }
    let ext = [0, 1, 2].map(|k| hi[k] - lo[k]);
    let res = i64::from(res.clamp(16, 384));
    let f = ext.iter().map(|e| (e + res - 1) / res).max().unwrap_or(1).max(1);
    let dims = ext.map(|e| ((e + f - 1) / f).max(1) as usize);
    let mut data = vec![0.0f32; dims[0] * dims[1] * dims[2]];
    for (c, s, v) in entries() {
        let a = to_ilex(c, s);
        let r = [0, 1, 2].map(|k| (a[k] - lo[k], a[k] - lo[k] + s));
        // Coarse cells the box overlaps, with the overlap per axis.
        let span = |k: usize| ((r[k].0 / f)..=((r[k].1 - 1) / f)).map(move |i| (i as usize, (r[k].1.min((i + 1) * f) - r[k].0.max(i * f)) as f32));
        for (z, wz) in span(2) {
            for (y, wy) in span(1) {
                for (x, wx) in span(0) {
                    data[(z * dims[1] + y) * dims[0] + x] += v * wx * wy * wz;
                }
            }
        }
    }
    let n = (f * f * f) as f32;
    let peak = data.iter().fold(0.0f32, |m, d| m.max(d / n));
    if peak <= 0.0 {
        return None;
    }
    data.iter_mut().for_each(|d| *d /= n * peak);
    // World corners: a file voxel is centred on its index.
    let (vs, t, flip) = match up {
        UpAxis::Z => (vs, t, [false; 3]),
        UpAxis::Y => ([vs[0], vs[2], vs[1]], [t[0], -t[2], t[1]], [false, true, false]),
    };
    let corner = |k: usize, n: i64| (if flip[k] { (n as f64 + 0.5) * vs[k] - t[k] } else { (n as f64 - 0.5) * vs[k] + t[k] }) as f32;
    let lo_w = Vec3::new(corner(0, lo[0]), corner(1, lo[1]), corner(2, lo[2]));
    let size = Vec3::new((dims[0] as i64 * f) as f32 * vs[0] as f32, (dims[1] as i64 * f) as f32 * vs[1] as f32, (dims[2] as i64 * f) as f32 * vs[2] as f32);
    Some(GridData { vox: Arc::new(Voxels { dims, data }), lo: lo_w, hi: lo_w + size, peak, active })
}
