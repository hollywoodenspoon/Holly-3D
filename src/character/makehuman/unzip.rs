//! A small `.zip` reader for importing MakeHuman asset downloads: reads
//! the central directory and inflates stored and deflated entries with
//! `miniz_oxide` (no extra crate). Zip64, encryption and other methods
//! are reported as errors.

fn u16_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as u32)
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Every file in the archive as `(path, bytes)`; folders are skipped.
pub fn entries(zip: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    // End of central directory: signature 0x06054b50 within the last 64 KiB.
    let lo = zip.len().saturating_sub(65_557);
    let eocd = (lo..zip.len().saturating_sub(21)).rev().find(|&i| u32_at(zip, i) == Some(0x0605_4b50)).ok_or("not a zip file")?;
    let count = u16_at(zip, eocd + 10).ok_or("bad zip")? as usize;
    let mut at = u32_at(zip, eocd + 16).ok_or("bad zip")? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if u32_at(zip, at) != Some(0x0201_4b50) {
            return Err("damaged zip directory".into());
        }
        let field = |o: usize| u16_at(zip, at + o).ok_or("bad zip");
        let (flags, method) = (field(8)?, field(10)?);
        let size = u32_at(zip, at + 20).ok_or("bad zip")? as usize;
        let (name_len, extra_len, note_len) = (field(28)? as usize, field(30)? as usize, field(32)? as usize);
        let local = u32_at(zip, at + 42).ok_or("bad zip")? as usize;
        let name = String::from_utf8_lossy(zip.get(at + 46..at + 46 + name_len).ok_or("bad zip")?).replace('\\', "/");
        at += 46 + name_len + extra_len + note_len;
        if name.ends_with('/') {
            continue;
        }
        if flags & 1 != 0 {
            return Err(format!("{name}: encrypted zips are not supported"));
        }
        if u32_at(zip, local) != Some(0x0403_4b50) {
            return Err("damaged zip entry".into());
        }
        let start = local + 30 + u16_at(zip, local + 26).ok_or("bad zip")? as usize + u16_at(zip, local + 28).ok_or("bad zip")? as usize;
        let raw = zip.get(start..start + size).ok_or("truncated zip")?;
        let data = match method {
            0 => raw.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec(raw).map_err(|e| format!("{name}: {e:?}"))?,
            m => return Err(format!("{name}: zip method {m} is not supported (use a plain zip)")),
        };
        out.push((name, data));
    }
    Ok(out)
}

#[cfg(test)]
pub mod tests {
    /// A one-file stored zip, built by hand for tests.
    pub fn stored(files: &[(&str, &[u8])]) -> Vec<u8> {
        let (mut body, mut dir) = (Vec::new(), Vec::new());
        for (name, data) in files {
            let off = body.len() as u32;
            let n = name.len() as u16;
            let sz = data.len() as u32;
            body.extend(0x0403_4b50u32.to_le_bytes());
            body.extend([20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            body.extend(sz.to_le_bytes());
            body.extend(sz.to_le_bytes());
            body.extend(n.to_le_bytes());
            body.extend(0u16.to_le_bytes());
            body.extend(name.as_bytes());
            body.extend(*data);
            dir.extend(0x0201_4b50u32.to_le_bytes());
            dir.extend([20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            dir.extend(sz.to_le_bytes());
            dir.extend(sz.to_le_bytes());
            dir.extend(n.to_le_bytes());
            dir.extend([0u8; 12]);
            dir.extend(off.to_le_bytes());
            dir.extend(name.as_bytes());
        }
        let (dir_off, dir_len) = (body.len() as u32, dir.len() as u32);
        body.extend(dir);
        body.extend(0x0605_4b50u32.to_le_bytes());
        body.extend([0, 0, 0, 0]);
        body.extend((files.len() as u16).to_le_bytes());
        body.extend((files.len() as u16).to_le_bytes());
        body.extend(dir_len.to_le_bytes());
        body.extend(dir_off.to_le_bytes());
        body.extend(0u16.to_le_bytes());
        body
    }

    #[test]
    fn reads_stored_entries_and_rejects_junk() {
        let z = stored(&[("hair/a/a.mhclo", b"name a"), ("hair/a/", b"")]);
        let e = super::entries(&z).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0], ("hair/a/a.mhclo".to_string(), b"name a".to_vec()));
        assert!(super::entries(b"nope").is_err());
    }
}
