//! Scene-file encoding of textures: PNG-compressed bytes instead of raw pixels.

use super::texture::Texture;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// Byte buffer written with `serialize_bytes` (compact in RON).
struct Bytes(Vec<u8>);

impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Bytes;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bytes")
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Bytes, E> {
                Ok(Bytes(v.to_vec()))
            }
            fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Bytes, E> {
                Ok(Bytes(v))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Bytes, A::Error> {
                let mut v = vec![];
                while let Some(b) = a.next_element()? {
                    v.push(b);
                }
                Ok(Bytes(v))
            }
        }
        d.deserialize_byte_buf(V)
    }
}

#[derive(Serialize, Deserialize)]
struct TexFile {
    width: u32,
    height: u32,
    png: Bytes,
}

impl Texture {
    pub fn to_png(&self) -> Vec<u8> {
        let mut out = std::io::Cursor::new(vec![]);
        let img = image::RgbaImage::from_raw(self.width, self.height, self.bytes()).expect("texture size");
        img.write_to(&mut out, image::ImageFormat::Png).expect("png encode");
        out.into_inner()
    }

    pub fn from_png(bytes: &[u8]) -> Result<Self, String> {
        let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?.to_rgba8();
        let (width, height) = img.dimensions();
        Ok(Self { width, height, data: img.pixels().map(|p| p.0).collect() })
    }
}

impl Serialize for Texture {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        TexFile { width: self.width, height: self.height, png: Bytes(self.to_png()) }.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Texture {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let f = TexFile::deserialize(d)?;
        let t = Texture::from_png(&f.png.0).map_err(de::Error::custom)?;
        if (t.width, t.height) != (f.width, f.height) {
            return Err(de::Error::custom("texture size mismatch"));
        }
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_round_trips_through_ron_as_png() {
        let mut t = Texture::new(3, 2, [10, 20, 30, 255]);
        t.data[4] = [200, 0, 7, 128];
        let s = ron::to_string(&t).unwrap();
        assert!(s.len() < 400, "{s}");
        let back: Texture = ron::from_str(&s).unwrap();
        assert_eq!(back, t);
    }
}
