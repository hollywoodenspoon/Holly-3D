//! Smoothing-group bitmasks straight from an FBX file. ufbx turns the
//! `Smoothing` layer into booleans, so this small reader pulls the raw
//! integers per Geometry name (ASCII and binary FBX, compressed arrays too).

use std::collections::HashMap;

/// Geometry name -> per-polygon smoothing values (`ByPolygon` layers only).
pub fn read(bytes: &[u8]) -> HashMap<String, Vec<u32>> {
    if bytes.starts_with(b"Kaydara FBX Binary") { binary(bytes) } else { ascii(&String::from_utf8_lossy(bytes)) }
}

fn ascii(text: &str) -> HashMap<String, Vec<u32>> {
    let mut out = HashMap::new();
    let mut rest = text;
    while let Some(g) = rest.find("Geometry::") {
        rest = &rest[g + 10..];
        let name: String = rest.chars().take_while(|&c| c != '"').collect();
        let end = rest.find("\n\tGeometry:").or_else(|| rest.find("\n\tModel:")).unwrap_or(rest.len());
        let block = &rest[..end];
        let Some(l) = block.find("LayerElementSmoothing") else { continue };
        let layer = &block[l..];
        if !layer.split('}').next().is_some_and(|h| h.contains("\"ByPolygon\"")) && !layer.contains("MappingInformationType: \"ByPolygon\"")
        {
            continue;
        }
        let Some(s) = layer.find("Smoothing:") else { continue };
        let Some(a) = layer[s..].find("a:") else { continue };
        let body = &layer[s + a + 2..];
        let body = &body[..body.find('}').unwrap_or(body.len())];
        let vals: Vec<u32> = body.split(',').filter_map(|v| v.trim().parse::<i64>().ok()).map(|v| v as u32).collect();
        out.insert(name, vals);
    }
    out
}

/// One parsed binary FBX record.
struct Node {
    name: String,
    props: Vec<Prop>,
    children: Vec<Node>,
}

enum Prop {
    Str(String),
    Ints(Vec<i64>),
    Other,
}

struct Reader<'a> {
    b: &'a [u8],
    wide: bool,
}

impl Reader<'_> {
    fn u32(&self, at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.b.get(at..at + 4)?.try_into().ok()?))
    }
    fn word(&self, at: usize) -> Option<u64> {
        if self.wide { Some(u64::from_le_bytes(self.b.get(at..at + 8)?.try_into().ok()?)) } else { self.u32(at).map(u64::from) }
    }

    /// Node at `at`; returns it with the offset just past it (`None` = null record).
    fn node(&self, at: usize) -> Option<(Option<Node>, usize)> {
        let w = if self.wide { 8 } else { 4 };
        let end = self.word(at)? as usize;
        if end == 0 {
            return Some((None, at + 3 * w + 1));
        }
        let nprops = self.word(at + w)? as usize;
        let name_len = *self.b.get(at + 3 * w)? as usize;
        let name_at = at + 3 * w + 1;
        let name = String::from_utf8_lossy(self.b.get(name_at..name_at + name_len)?).into_owned();
        let mut p = name_at + name_len;
        let mut props = vec![];
        for _ in 0..nprops {
            let (prop, next) = self.prop(p)?;
            props.push(prop);
            p = next;
        }
        let mut children = vec![];
        while p < end {
            let (child, next) = self.node(p)?;
            match child {
                Some(c) => children.push(c),
                None => break,
            }
            p = next;
        }
        Some((Some(Node { name, props, children }), end))
    }

    fn prop(&self, at: usize) -> Option<(Prop, usize)> {
        let t = *self.b.get(at)?;
        let p = at + 1;
        Some(match t {
            b'Y' => (Prop::Other, p + 2),
            b'C' => (Prop::Other, p + 1),
            b'I' | b'F' => (Prop::Other, p + 4),
            b'D' | b'L' => (Prop::Other, p + 8),
            b'S' | b'R' => {
                let n = self.u32(p)? as usize;
                let s = String::from_utf8_lossy(self.b.get(p + 4..p + 4 + n)?).into_owned();
                (Prop::Str(s), p + 4 + n)
            }
            b'f' | b'd' | b'l' | b'i' | b'b' => {
                let (count, enc, len) = (self.u32(p)? as usize, self.u32(p + 4)?, self.u32(p + 8)? as usize);
                let raw = self.b.get(p + 12..p + 12 + len)?;
                let data = if enc == 1 { miniz_oxide::inflate::decompress_to_vec_zlib(raw).ok()? } else { raw.to_vec() };
                let ints = match t {
                    b'i' => data.as_chunks::<4>().0.iter().take(count).map(|c| i32::from_le_bytes(*c) as i64).collect(),
                    b'l' => data.as_chunks::<8>().0.iter().take(count).map(|c| i64::from_le_bytes(*c)).collect(),
                    b'b' => data.iter().take(count).map(|&v| v as i64).collect(),
                    _ => vec![],
                };
                (Prop::Ints(ints), p + 12 + len)
            }
            _ => return None,
        })
    }
}

fn binary(b: &[u8]) -> HashMap<String, Vec<u32>> {
    let version = Reader { b, wide: false }.u32(23).unwrap_or(7400);
    let r = Reader { b, wide: version >= 7500 };
    let mut out = HashMap::new();
    let mut at = 27;
    while let Some((Some(node), next)) = r.node(at) {
        if node.name == "Objects" {
            for g in node.children.iter().filter(|c| c.name == "Geometry") {
                let name = g
                    .props
                    .iter()
                    .find_map(|p| if let Prop::Str(s) = p { Some(s.split('\0').next().unwrap_or("").to_string()) } else { None });
                if let (Some(name), Some(vals)) = (name, smoothing_of(g)) {
                    out.insert(name, vals);
                }
            }
        }
        at = next;
    }
    out
}

fn smoothing_of(g: &Node) -> Option<Vec<u32>> {
    let layer = g.children.iter().find(|c| c.name == "LayerElementSmoothing")?;
    let mapping = layer.children.iter().find(|c| c.name == "MappingInformationType")?;
    if !matches!(mapping.props.first(), Some(Prop::Str(s)) if s == "ByPolygon") {
        return None;
    }
    let data = layer.children.iter().find(|c| c.name == "Smoothing")?;
    match data.props.first() {
        Some(Prop::Ints(v)) => Some(v.iter().map(|&x| x as u32).collect()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal binary FBX 7.4 with one Geometry carrying a zlib-compressed Smoothing array.
    fn binary_sample() -> Vec<u8> {
        fn node(name: &str, props: &[u8], nprops: u32, children: &[Vec<u8>], at: usize) -> Vec<u8> {
            let header = 13 + name.len();
            let kids: usize = children.iter().map(|c| c.len()).sum();
            let null = if children.is_empty() { 0 } else { 13 };
            let end = at + header + props.len() + kids + null;
            let mut o = vec![];
            o.extend((end as u32).to_le_bytes());
            o.extend(nprops.to_le_bytes());
            o.extend((props.len() as u32).to_le_bytes());
            o.push(name.len() as u8);
            o.extend(name.as_bytes());
            o.extend(props);
            for c in children {
                o.extend(c);
            }
            o.extend(vec![0u8; null]);
            o
        }
        let s = |v: &str| [vec![b'S'], (v.len() as u32).to_le_bytes().to_vec(), v.as_bytes().to_vec()].concat();
        let vals: Vec<u8> = [0i32, 5, 1 << 20].iter().flat_map(|v| v.to_le_bytes()).collect();
        let z = miniz_oxide::deflate::compress_to_vec_zlib(&vals, 6);
        let arr =
            [vec![b'i'], 3u32.to_le_bytes().to_vec(), 1u32.to_le_bytes().to_vec(), (z.len() as u32).to_le_bytes().to_vec(), z].concat();
        // children offsets are absolute, so build bottom-up with known starts
        let mut out = b"Kaydara FBX Binary  \0\x1a\0".to_vec();
        out.extend(7400u32.to_le_bytes());
        let objects_at = out.len();
        let geo_at = objects_at + 13 + "Objects".len();
        let geo_props = s("Box001\0\x01Geometry");
        let layer_at = geo_at + 13 + "Geometry".len() + geo_props.len();
        let map_props = s("ByPolygon");
        let map_at = layer_at + 13 + "LayerElementSmoothing".len();
        let mapping = node("MappingInformationType", &map_props, 1, &[], map_at);
        let smooth = node("Smoothing", &arr, 1, &[], map_at + mapping.len());
        let layer = node("LayerElementSmoothing", &[], 0, &[mapping, smooth], layer_at);
        let geo = node("Geometry", &geo_props, 1, &[layer], geo_at);
        out.extend(node("Objects", &[], 0, &[geo], objects_at));
        out.extend([0u8; 13]);
        out
    }

    #[test]
    fn reads_binary_and_ascii_smoothing_masks() {
        let b = read(&binary_sample());
        assert_eq!(b.get("Box001"), Some(&vec![0, 5, 1 << 20]));
        let a = "\tGeometry: 1, \"Geometry::Hull\", \"Mesh\" {\n\t\tLayerElementSmoothing: 0 {\n\t\t\tMappingInformationType: \"ByPolygon\"\n\t\t\tSmoothing: *2 {\n\t\t\t\ta: 3,4294967295\n\t\t\t}\n\t\t}\n\t}\n";
        assert_eq!(read(a.as_bytes()).get("Hull"), Some(&vec![3, u32::MAX]));
    }
}
