//! MakeHuman `.target` morphs: sparse per-vertex offsets of the hm08 base
//! mesh. Two sources: the bundled pack (`targets.mht`, 868 targets,
//! quantised to 1/3000 dm and delta-encoded) and plain ASCII `.target`
//! text (`index dx dy dz` lines), so user targets load too.

use super::assets;
use bevy::math::Vec3;
use std::collections::HashMap;
use std::sync::OnceLock;

/// A sparse morph: `delta[i]` moves base vertex `index[i]` (decimetres).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target {
    pub index: Vec<u32>,
    pub delta: Vec<Vec3>,
}

impl Target {
    /// Parses ASCII `.target` text.
    pub fn parse_ascii(src: &str) -> Self {
        let mut t = Target::default();
        for line in src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let f: Vec<&str> = line.split_ascii_whitespace().collect();
            if f.len() < 4 {
                continue;
            }
            let (Ok(i), Ok(x), Ok(y), Ok(z)) = (f[0].parse::<u32>(), f[1].parse::<f32>(), f[2].parse::<f32>(), f[3].parse::<f32>()) else {
                continue;
            };
            t.index.push(i);
            t.delta.push(Vec3::new(x, y, z));
        }
        t
    }

    /// Adds `weight` x this morph to `coords`.
    pub fn apply(&self, coords: &mut [Vec3], weight: f32) {
        if weight == 0.0 {
            return;
        }
        for (&i, d) in self.index.iter().zip(&self.delta) {
            if let Some(c) = coords.get_mut(i as usize) {
                *c += *d * weight;
            }
        }
    }

    fn decode(b: &[u8]) -> Self {
        let rd_u32 = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let rd_16 = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let n = rd_u32(0) as usize;
        let mut index = Vec::with_capacity(n);
        let mut acc = 0u32;
        for k in 0..n {
            acc += rd_16(4 + 2 * k) as u32;
            index.push(acc);
        }
        let mut delta = vec![Vec3::ZERO; n];
        for axis in 0..3 {
            let base = 4 + 2 * n * (axis + 1);
            let mut v = 0i16;
            for (k, d) in delta.iter_mut().enumerate() {
                v = v.wrapping_add(rd_16(base + 2 * k) as i16);
                d[axis] = v as f32 / 3000.0;
            }
        }
        Target { index, delta }
    }
}

/// Index of the bundled pack: name -> byte range.
pub struct Pack {
    data: &'static [u8],
    entries: HashMap<String, (usize, usize)>,
}

impl Pack {
    fn parse(data: &'static [u8]) -> Self {
        let mut entries = HashMap::new();
        if data.len() < 8 || &data[..4] != b"MHT1" {
            return Pack { data, entries };
        }
        let count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let mut o = 8;
        let mut list = Vec::with_capacity(count);
        for _ in 0..count {
            let l = u16::from_le_bytes([data[o], data[o + 1]]) as usize;
            let name = String::from_utf8_lossy(&data[o + 2..o + 2 + l]).into_owned();
            o += 2 + l;
            let len = u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]) as usize;
            o += 4;
            list.push((name, len));
        }
        for (name, len) in list {
            entries.insert(name, (o, len));
            o += len;
        }
        Pack { data, entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Decodes target `name` (e.g. `"nose/nose-base-up"`).
    pub fn get(&self, name: &str) -> Option<Target> {
        let &(o, len) = self.entries.get(name)?;
        Some(Target::decode(&self.data[o..o + len]))
    }

    /// `coords += weight * target(name)`; false if the target is missing.
    pub fn apply(&self, name: &str, coords: &mut [Vec3], weight: f32) -> bool {
        if weight.abs() < 1e-5 {
            return self.contains(name);
        }
        match self.get(name) {
            Some(t) => {
                t.apply(coords, weight);
                true
            }
            None => false,
        }
    }
}

/// The bundled target pack (cached).
pub fn pack() -> &'static Pack {
    static CELL: OnceLock<Pack> = OnceLock::new();
    CELL.get_or_init(|| Pack::parse(assets::targets_pack()))
}
