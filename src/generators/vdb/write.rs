//! Test-only: writes small, uncompressed OpenVDB files (file format 224,
//! float grids, one root node at the origin), so the import can be checked
//! end to end without bundling simulation files.

use std::collections::BTreeMap;

/// One float grid: active voxels and active 8³ tiles (index coordinates,
/// all in 0..4096; tile corners a multiple of 8).
pub struct TestGrid {
    pub name: String,
    pub voxels: Vec<([u32; 3], f32)>,
    pub tiles: Vec<([u32; 3], f32)>,
    pub voxel_size: f64,
    pub translation: [f64; 3],
}

fn put_u32(o: &mut Vec<u8>, v: u32) {
    o.extend_from_slice(&v.to_le_bytes());
}

fn put_str(o: &mut Vec<u8>, s: &str) {
    put_u32(o, s.len() as u32);
    o.extend_from_slice(s.as_bytes());
}

fn put_v3(o: &mut Vec<u8>, v: [f64; 3]) {
    v.iter().for_each(|x| o.extend_from_slice(&x.to_le_bytes()));
}

fn put_mask(o: &mut Vec<u8>, bits: &[bool]) {
    for w in bits.chunks(64) {
        let word = w.iter().enumerate().fold(0u64, |a, (i, b)| a | (u64::from(*b) << i));
        o.extend_from_slice(&word.to_le_bytes());
    }
}

/// Node data: "no mask, all values" then the raw floats.
fn put_vals(o: &mut Vec<u8>, vals: &[f32]) {
    o.push(6);
    vals.iter().for_each(|v| o.extend_from_slice(&v.to_le_bytes()));
}

fn idx(c: [u32; 3], shift: u32, log2: u32) -> usize {
    let m = (1 << log2) - 1;
    let [x, y, z] = c.map(|v| ((v >> shift) & m) as usize);
    (x << (2 * log2)) | (y << log2) | z
}

#[derive(Default)]
struct Inner {
    leaves: BTreeMap<usize, [f32; 512]>,
    active: BTreeMap<usize, Vec<usize>>,
    tiles: BTreeMap<usize, f32>,
}

fn grid_bytes(g: &TestGrid, base: usize) -> Vec<u8> {
    let mut tree: BTreeMap<usize, Inner> = BTreeMap::new();
    for &(c, v) in &g.voxels {
        let n4 = tree.entry(idx(c, 7, 5)).or_default();
        let i4 = idx(c, 3, 4);
        n4.leaves.entry(i4).or_insert([0.0; 512])[idx(c, 0, 3)] = v;
        n4.active.entry(i4).or_default().push(idx(c, 0, 3));
    }
    for &(c, v) in &g.tiles {
        tree.entry(idx(c, 7, 5)).or_default().tiles.insert(idx(c, 3, 4), v);
    }
    let mut o = Vec::new();
    put_u32(&mut o, 0); // compression: none
    put_u32(&mut o, 0); // grid metadata
    put_str(&mut o, "ScaleTranslateMap");
    let (s, t) = (g.voxel_size, g.translation);
    for v in [t, [s; 3], [s; 3], [1.0 / s; 3], [1.0 / (s * s); 3], [0.5 / s; 3]] {
        put_v3(&mut o, v);
    }
    for v in [1, 0, 0, 1] {
        put_u32(&mut o, v); // buffers, background, tiles, root nodes
    }
    o.extend_from_slice(&[0u8; 12]); // root origin
    let mut child5 = vec![false; 32768];
    tree.keys().for_each(|&k| child5[k] = true);
    put_mask(&mut o, &child5);
    put_mask(&mut o, &[false; 32768]);
    put_vals(&mut o, &[0.0; 32768]);
    for n4 in tree.values() {
        let (mut child, mut value, mut data) = (vec![false; 4096], vec![false; 4096], vec![0.0; 4096]);
        n4.leaves.keys().for_each(|&k| child[k] = true);
        for (&k, &v) in &n4.tiles {
            (value[k], data[k]) = (true, v);
        }
        put_mask(&mut o, &child);
        put_mask(&mut o, &value);
        put_vals(&mut o, &data);
        for k in n4.leaves.keys() {
            let mut m = [false; 512];
            n4.active[k].iter().for_each(|&i| m[i] = true);
            put_mask(&mut o, &m);
        }
    }
    let block = base + o.len();
    for n4 in tree.values() {
        for (k, vals) in &n4.leaves {
            let mut m = [false; 512];
            n4.active[k].iter().for_each(|&i| m[i] = true);
            put_mask(&mut o, &m);
            put_vals(&mut o, vals);
        }
    }
    // Block position goes first, as an 8-byte prefix the caller strips.
    let mut out = (block as u64).to_le_bytes().to_vec();
    out.extend(o);
    out
}

/// The bytes of a VDB file holding `grids`.
pub fn vdb_bytes(grids: &[TestGrid]) -> Vec<u8> {
    let mut o = Vec::new();
    o.extend_from_slice(&0x5644_4220u64.to_le_bytes());
    for v in [224, 10, 0] {
        put_u32(&mut o, v);
    }
    o.push(1); // grid offsets
    o.extend_from_slice(b"00000000-0000-0000-0000-000000000000");
    put_u32(&mut o, 0); // file metadata
    put_u32(&mut o, grids.len() as u32);
    for g in grids {
        put_str(&mut o, &g.name);
        put_str(&mut o, "Tree_float_5_4_3");
        put_str(&mut o, "");
        let grid_pos = o.len() + 24;
        let body = grid_bytes(g, grid_pos);
        let block = u64::from_le_bytes(body[..8].try_into().unwrap_or_default());
        let body = &body[8..];
        for v in [grid_pos as u64, block, (grid_pos + body.len()) as u64] {
            o.extend_from_slice(&v.to_le_bytes());
        }
        o.extend_from_slice(body);
    }
    o
}
