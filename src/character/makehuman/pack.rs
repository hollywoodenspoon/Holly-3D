//! The `ILXPAK1` record pack written by `tools/pack_makehuman.py`: a
//! magic line, then (u32 LE name length, name, u32 LE data length, data)
//! records. Records borrow the pack bytes; nothing is copied.

use std::collections::BTreeMap;

pub const MAGIC: &[u8] = b"ILXPAK1\n";

#[derive(Debug, Default)]
pub struct Pack<'a> {
    pub records: BTreeMap<&'a str, &'a [u8]>,
}

impl<'a> Pack<'a> {
    /// Parses `bytes`; a damaged tail is dropped, earlier records are kept.
    pub fn parse(bytes: &'a [u8]) -> Self {
        let mut p = Pack::default();
        if !bytes.starts_with(MAGIC) {
            return p;
        }
        let mut o = MAGIC.len();
        let u32_at = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        while let Some(n) = u32_at(o) {
            let Some(name) = bytes.get(o + 4..o + 4 + n).and_then(|b| std::str::from_utf8(b).ok()) else { break };
            let Some(len) = u32_at(o + 4 + n) else { break };
            let start = o + 8 + n;
            let Some(data) = bytes.get(start..start + len) else { break };
            p.records.insert(name, data);
            o = start + len;
        }
        p
    }

    pub fn get(&self, name: &str) -> Option<&'a [u8]> {
        self.records.get(name).copied()
    }

    pub fn text(&self, name: &str) -> Option<&'a str> {
        self.get(name).and_then(|b| std::str::from_utf8(b).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_records_and_survives_a_cut_tail() {
        let mut b = MAGIC.to_vec();
        for (name, data) in [("a/x.obj", &b"v 1 2 3"[..]), ("b", &b"\x00\x01"[..])] {
            b.extend((name.len() as u32).to_le_bytes());
            b.extend(name.as_bytes());
            b.extend((data.len() as u32).to_le_bytes());
            b.extend(data);
        }
        let p = Pack::parse(&b);
        assert_eq!(p.text("a/x.obj"), Some("v 1 2 3"));
        assert_eq!(p.get("b"), Some(&b"\x00\x01"[..]));
        let cut = Pack::parse(&b[..b.len() - 1]);
        assert_eq!(cut.records.len(), 1);
        assert!(Pack::parse(b"nope").records.is_empty());
    }
}
