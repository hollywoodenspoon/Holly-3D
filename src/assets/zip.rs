//! A small ZIP reader (stored and deflated entries, no ZIP64): enough to
//! unpack the model archives the low-poly pack downloads.

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize)
}

fn u32_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
}

/// One file inside an archive.
pub struct Entry<'a> {
    pub name: String,
    data: &'a [u8],
    deflated: bool,
    size: usize,
}

impl Entry<'_> {
    /// The file's bytes, inflated if needed.
    pub fn read(&self) -> Result<Vec<u8>, String> {
        if !self.deflated {
            return Ok(self.data.to_vec());
        }
        miniz_oxide::inflate::decompress_to_vec_with_limit(self.data, self.size.max(1)).map_err(|e| format!("{}: {e:?}", self.name))
    }
}

/// The files of a ZIP archive (directories are skipped).
pub fn entries(zip: &[u8]) -> Result<Vec<Entry<'_>>, String> {
    let bad = || "not a ZIP archive".to_string();
    // End of central directory: the last "PK\x05\x06".
    let eocd = (0..=zip.len().saturating_sub(22)).rev().find(|&i| zip[i..].starts_with(b"PK\x05\x06")).ok_or_else(bad)?;
    let count = u16_at(zip, eocd + 10).ok_or_else(bad)?;
    let mut at = u32_at(zip, eocd + 16).ok_or_else(bad)?;
    let mut out = vec![];
    for _ in 0..count {
        if !zip.get(at..).is_some_and(|b| b.starts_with(b"PK\x01\x02")) {
            return Err(bad());
        }
        let field = |off: usize| u16_at(zip, at + off).ok_or_else(bad);
        let (method, name_len, extra, comment) = (field(10)?, field(28)?, field(30)?, field(32)?);
        let (packed, size, local) = (u32_at(zip, at + 20).ok_or_else(bad)?, u32_at(zip, at + 24).ok_or_else(bad)?, u32_at(zip, at + 42).ok_or_else(bad)?);
        let name = String::from_utf8_lossy(zip.get(at + 46..at + 46 + name_len).ok_or_else(bad)?).into_owned();
        at += 46 + name_len + extra + comment;
        if name.ends_with('/') {
            continue;
        }
        // The local header repeats the name and has its own extra field.
        let start = local + 30 + u16_at(zip, local + 26).ok_or_else(bad)? + u16_at(zip, local + 28).ok_or_else(bad)?;
        let data = zip.get(start..start + packed).ok_or_else(bad)?;
        match method {
            0 | 8 => out.push(Entry { name, data, deflated: method == 8, size }),
            m => return Err(format!("{name}: unsupported ZIP compression {m}")),
        }
    }
    Ok(out)
}

/// Builds a ZIP archive (deflated entries); used for test fixtures.
#[cfg(test)]
pub fn build(files: &[(&str, &[u8])]) -> Vec<u8> {
    let (mut out, mut central) = (vec![], vec![]);
    for (name, data) in files {
        let packed = miniz_oxide::deflate::compress_to_vec(data, 6);
        let local = out.len() as u32;
        let sizes = |v: &mut Vec<u8>| {
            v.extend_from_slice(&0u32.to_le_bytes()); // crc (not checked)
            v.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            v.extend_from_slice(&(data.len() as u32).to_le_bytes());
            v.extend_from_slice(&(name.len() as u16).to_le_bytes());
        };
        out.extend_from_slice(b"PK\x03\x04\x14\x00\x00\x00\x08\x00\x00\x00\x00\x00");
        sizes(&mut out);
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);
        central.extend_from_slice(b"PK\x01\x02\x14\x00\x14\x00\x00\x00\x08\x00\x00\x00\x00\x00");
        sizes(&mut central);
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&local.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let (cd_at, cd_len, n) = (out.len() as u32, central.len() as u32, files.len() as u16);
    out.extend_from_slice(&central);
    out.extend_from_slice(b"PK\x05\x06\x00\x00\x00\x00");
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&cd_len.to_le_bytes());
    out.extend_from_slice(&cd_at.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}
