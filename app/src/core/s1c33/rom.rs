// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Bounded, content-verified import forms. Originals and save identity stay unchanged.
use super::profile::HardwareProfile;
use crate::core::state::{hex, sha256};
use std::{borrow::Cow, io::Read};

const ARCHIVE: &str = "3395f83033596c03a9844ce98fcde6f1e3b4cf7a3fe9e2acb12410bc09a7fa05";
const JAPANESE: &str = "59c067987804a6c2a200e0a98b95b332c452c1b8b11daa4fd80cd2cbaff319ff";
const ENGLISH: &str = "40639648dafd133ed350635819d1bb724ddfde4bc076c81580180a42c17e4caf";

pub(super) fn source_size_matches(p: HardwareProfile, hash: &str, size: usize) -> bool {
    size == p.rom_size as usize
        || match (p.key, size) {
            ("idl", 6_039_638) => hash == ARCHIVE,
            ("idl", 8_387_584) => hash == JAPANESE,
            ("idl-en", 8_386_560) => hash == ENGLISH,
            _ => false,
        }
}

pub(super) fn decode<'a>(bytes: &'a [u8], p: HardwareProfile) -> Result<Cow<'a, [u8]>, String> {
    if bytes.len() == p.rom_size as usize {
        return Ok(Cow::Borrowed(bytes));
    }
    let mut image =
        if p.key == "idl" && bytes.starts_with(b"PK\x03\x04") && hex(&sha256(bytes)) == ARCHIVE {
            Cow::Owned(zip_nor(bytes)?)
        } else {
            Cow::Borrowed(bytes)
        };
    let known_short = match p.key {
        "idl" => image.len() == 0x800000 - 1024 && hex(&sha256(&image)) == JAPANESE,
        "idl-en" => image.len() == 0x800000 - 2048 && hex(&sha256(&image)) == ENGLISH,
        _ => false,
    };
    if known_short && p.rom_size == 0x800000 {
        // tamaemu main.c calloc()s the entire NOR before fread(): omitted bytes are zero.
        // Apply that behavior only to these exact, independently tested import identities.
        image.to_mut().resize(p.rom_size as usize, 0);
    }
    if image.len() != p.rom_size as usize {
        return Err(format!(
            "{} requires a complete {}-byte NOR image or an explicitly verified import form",
            p.title, p.rom_size
        ));
    }
    Ok(image)
}
fn u16_at(b: &[u8], at: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        b.get(at..at + 2)
            .ok_or("Truncated ZIP header")?
            .try_into()
            .unwrap(),
    ))
}
fn u32_at(b: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        b.get(at..at + 4)
            .ok_or("Truncated ZIP header")?
            .try_into()
            .unwrap(),
    ))
}
fn zip_nor(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let end = bytes.len().checked_sub(22).ok_or("Truncated ZIP")?;
    let eocd = (bytes.len().saturating_sub(65557)..=end)
        .rev()
        .find(|&i| bytes.get(i..i + 4) == Some(b"PK\x05\x06"))
        .ok_or("Missing ZIP directory")?;
    if u16_at(bytes, eocd + 4)? != 0 || u16_at(bytes, eocd + 6)? != 0 {
        return Err("Split ZIPs are unsupported".into());
    }
    let count = u16_at(bytes, eocd + 10)? as usize;
    if count > 4096 {
        return Err("ZIP directory is too large".into());
    }
    let mut at = u32_at(bytes, eocd + 16)? as usize;
    let mut candidate = None;
    for _ in 0..count {
        if bytes.get(at..at + 4) != Some(b"PK\x01\x02") {
            return Err("Invalid ZIP directory".into());
        }
        let flags = u16_at(bytes, at + 8)?;
        let method = u16_at(bytes, at + 10)?;
        let crc = u32_at(bytes, at + 16)?;
        let compressed = u32_at(bytes, at + 20)? as usize;
        let length = u32_at(bytes, at + 24)? as usize;
        let names = u16_at(bytes, at + 28)? as usize;
        let extra = u16_at(bytes, at + 30)? as usize;
        let comment = u16_at(bytes, at + 32)? as usize;
        let local = u32_at(bytes, at + 42)? as usize;
        let name = bytes
            .get(at + 46..at + 46 + names)
            .ok_or("Truncated ZIP name")?;
        if name.to_ascii_lowercase().ends_with(b".bin") {
            if candidate.is_some() || flags & 1 != 0 || length > 0x800000 {
                return Err("Ambiguous, encrypted or oversized NOR in ZIP".into());
            }
            candidate = Some((local, compressed, length, method, crc));
        }
        at = at
            .checked_add(46 + names + extra + comment)
            .ok_or("Invalid ZIP offsets")?;
    }
    let (local, compressed, length, method, expected_crc) =
        candidate.ok_or("ZIP contains no unique NOR image")?;
    if bytes.get(local..local + 4) != Some(b"PK\x03\x04") {
        return Err("Invalid ZIP local header".into());
    }
    let start =
        local + 30 + u16_at(bytes, local + 26)? as usize + u16_at(bytes, local + 28)? as usize;
    let data = bytes
        .get(start..start.checked_add(compressed).ok_or("Invalid ZIP size")?)
        .ok_or("Truncated ZIP payload")?;
    let mut image = Vec::new();
    match method {
        0 if compressed == length => image.extend_from_slice(data),
        0 => return Err("Invalid stored ZIP size".into()),
        8 => {
            flate2::read::DeflateDecoder::new(data)
                .take(length as u64 + 1)
                .read_to_end(&mut image)
                .map_err(|e| format!("Invalid ZIP deflate stream: {e}"))?;
        }
        _ => return Err("Unsupported ZIP compression".into()),
    }
    let mut crc = flate2::Crc::new();
    crc.update(&image);
    if image.len() != length || crc.sum() != expected_crc {
        return Err("ZIP NOR integrity check failed".into());
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(names: &[&str], payload: &[u8], method: u16) -> Vec<u8> {
        use std::io::Write;
        let data = if method == 8 {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(payload).unwrap();
            encoder.finish().unwrap()
        } else {
            payload.to_vec()
        };
        let mut crc = flate2::Crc::new();
        crc.update(payload);
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for name in names {
            let offset = out.len() as u32;
            let mut header = vec![0; 30];
            header[..4].copy_from_slice(b"PK\x03\x04");
            header[8..10].copy_from_slice(&method.to_le_bytes());
            header[14..18].copy_from_slice(&crc.sum().to_le_bytes());
            header[18..22].copy_from_slice(&(data.len() as u32).to_le_bytes());
            header[22..26].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            header[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend(header);
            out.extend(name.as_bytes());
            out.extend(&data);
            let mut central = vec![0; 46];
            central[..4].copy_from_slice(b"PK\x01\x02");
            central[10..12].copy_from_slice(&method.to_le_bytes());
            central[16..20].copy_from_slice(&crc.sum().to_le_bytes());
            central[20..24].copy_from_slice(&(data.len() as u32).to_le_bytes());
            central[24..28].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            central[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
            central[42..46].copy_from_slice(&offset.to_le_bytes());
            directory.extend(central);
            directory.extend(name.as_bytes());
        }
        let offset = out.len() as u32;
        let mut end = vec![0; 22];
        end[..4].copy_from_slice(b"PK\x05\x06");
        end[8..10].copy_from_slice(&(names.len() as u16).to_le_bytes());
        end[10..12].copy_from_slice(&(names.len() as u16).to_le_bytes());
        end[12..16].copy_from_slice(&(directory.len() as u32).to_le_bytes());
        end[16..20].copy_from_slice(&offset.to_le_bytes());
        out.extend(directory);
        out.extend(end);
        out
    }
    #[test]
    fn bounded_zip_extraction_checks_crc_size_and_unique_nor_without_writing_paths() {
        for method in [0, 8] {
            let bytes = archive(&["nested/NOR.BIN"], b"Synthetic NOR", method);
            assert_eq!(zip_nor(&bytes).unwrap(), b"Synthetic NOR");
            assert!(zip_nor(&bytes[..bytes.len() - 1]).is_err());
            let mut bad = bytes.clone();
            let central = bad.windows(4).position(|v| v == b"PK\x01\x02").unwrap();
            bad[central + 16] ^= 1;
            assert!(zip_nor(&bad).is_err());
            bad[central + 16] ^= 1;
            bad[central + 24..central + 28].copy_from_slice(&0x800001u32.to_le_bytes());
            assert!(zip_nor(&bad).is_err());
        }
        assert!(zip_nor(&archive(&["a.bin", "b.bin"], b"synthetic", 0)).is_err());
        assert!(zip_nor(&archive(&["readme.txt"], b"synthetic", 0)).is_err());
        assert!(zip_nor(&archive(&["a.bin"], b"synthetic", 12)).is_err());
        let mut encrypted = archive(&["a.bin"], b"synthetic", 0);
        let at = encrypted
            .windows(4)
            .position(|v| v == b"PK\x01\x02")
            .unwrap();
        encrypted[at + 8] = 1;
        assert!(zip_nor(&encrypted).is_err());
    }
    #[test]
    fn only_exact_verified_import_forms_relax_source_length() {
        let jp = HardwareProfile::find("idl").unwrap();
        let en = HardwareProfile::find("idl-en").unwrap();
        for (profile, hash, size) in [
            (jp, ARCHIVE, 6039638),
            (jp, JAPANESE, 8387584),
            (en, ENGLISH, 8386560),
        ] {
            assert!(source_size_matches(profile, hash, size));
            assert!(!source_size_matches(profile, "unknown", size));
            assert!(!source_size_matches(profile, hash, size - 1));
            assert!(decode(&vec![255; size], profile).is_err());
        }
        assert!(!source_size_matches(en, ARCHIVE, 6039638));
        assert!(decode(&archive(&["a.bin"], b"synthetic", 8), jp).is_err());
        let exact = vec![255; jp.rom_size as usize];
        assert!(matches!(decode(&exact, jp).unwrap(), Cow::Borrowed(_)));
    }
}
