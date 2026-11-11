//! Scene-file encoding of tiled images: only the tiles that hold data,
//! each deflate-compressed.

use super::{Cell, N, Tiles};
use crate::materials::tex_serde::Bytes;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct TilesFile {
    size: u32,
    tiles: Vec<(u32, Bytes)>,
}

impl<T: Cell> Serialize for Tiles<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let tiles = (0..self.tile_count())
            .filter_map(|i| {
                let data = self.tile(i)?;
                let mut raw = Vec::with_capacity(N * T::BYTES);
                data.iter().for_each(|c| c.write(&mut raw));
                Some((i as u32, Bytes(miniz_oxide::deflate::compress_to_vec(&raw, 6))))
            })
            .collect();
        TilesFile { size: self.size(), tiles }.serialize(s)
    }
}

impl<'de, T: Cell> Deserialize<'de> for Tiles<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let f = TilesFile::deserialize(d)?;
        if f.size == 0 || f.size > 16384 {
            return Err(D::Error::custom("paint layer size out of range"));
        }
        let mut out = Tiles::new(f.size);
        for (i, bytes) in f.tiles {
            let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(&bytes.0, N * T::BYTES).map_err(|_| D::Error::custom("bad paint tile"))?;
            if raw.len() != N * T::BYTES || i as usize >= out.tiles.len() {
                return Err(D::Error::custom("paint tile size mismatch"));
            }
            let data: Vec<T> = raw.chunks_exact(T::BYTES).map(T::read).collect();
            out.tiles[i as usize] = Some(Arc::from(data));
        }
        Ok(out)
    }
}
