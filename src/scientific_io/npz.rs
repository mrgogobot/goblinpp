//! Bounded NPZ containers. Never extract names to the filesystem.
use super::{MAX_BYTES, fail};
use crate::error::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use zip::{CompressionMethod, ZipArchive};

pub struct Archive<'a> {
    pub keys: BTreeMap<String, usize>,
    zip: ZipArchive<Cursor<&'a [u8]>>,
}
impl<'a> Archive<'a> {
    pub fn open(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES || bytes.len() < 22 {
            return Err(fail("NPZ file is invalid or exceeds 16 MiB."));
        }
        // Preflight entry count before the ZIP library allocates its central index.
        // Ordinary NumPy ZIP64 local headers are supported; ZIP64 central directories
        // and self-extracting/prefixed or multidisk archives are deliberately refused.
        let end = (bytes.len().saturating_sub(65557)..=bytes.len() - 22)
            .rev()
            .find(|i| {
                bytes[*i..].starts_with(b"PK\x05\x06")
                    && *i
                        + 22
                        + usize::from(u16::from_le_bytes(
                            bytes[*i + 20..*i + 22].try_into().unwrap(),
                        ))
                        == bytes.len()
            })
            .ok_or_else(|| fail("NPZ end-of-directory record is missing."))?;
        let word = |i| u16::from_le_bytes(bytes[end + i..end + i + 2].try_into().unwrap());
        if word(4) != 0 || word(6) != 0 || word(8) != word(10) || word(10) > 128 {
            return Err(fail(
                "NPZ supports at most 128 entries, one disk, and no ZIP64 central directory.",
            ));
        }
        let central_size =
            u32::from_le_bytes(bytes[end + 12..end + 16].try_into().unwrap()) as usize;
        let central_offset =
            u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
        if central_offset.checked_add(central_size) != Some(end)
            || !(bytes.starts_with(b"PK\x03\x04")
                || word(10) == 0 && bytes.starts_with(b"PK\x05\x06"))
        {
            return Err(fail("NPZ central directory boundaries/prefix are invalid."));
        }
        let mut zip = ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| fail(format!("Invalid NPZ archive: {e}")))?;
        if zip.len() != usize::from(word(10)) {
            return Err(fail("NPZ central-directory entry count mismatch."));
        }
        let (mut keys, mut seen, mut total) = (BTreeMap::new(), BTreeSet::new(), 0u64);
        for index in 0..zip.len() {
            let member = zip
                .by_index(index)
                .map_err(|e| fail(format!("Unreadable NPZ member: {e}")))?;
            let name = std::str::from_utf8(member.name_raw())
                .map_err(|_| fail("NPZ member names must be UTF-8."))?;
            let key = name
                .strip_suffix(".npy")
                .ok_or_else(|| fail("NPZ entries must all be named .npy arrays."))?;
            if key.is_empty()
                || key.len() > 256
                || key == "."
                || key == ".."
                || key
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
                || !seen.insert(name.to_owned())
                || member.is_symlink()
                || !member.is_file()
                || member.encrypted()
            {
                return Err(fail(
                    "NPZ duplicate/unsafe name, link, nonfile or encrypted member refused.",
                ));
            }
            if !matches!(
                member.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            ) {
                return Err(fail("NPZ supports only stored and DEFLATE compression."));
            }
            total = total
                .checked_add(member.size())
                .ok_or_else(|| fail("NPZ expanded-byte count overflow."))?;
            if member.size() > MAX_BYTES as u64 || total > 64 * 1024 * 1024 {
                return Err(fail(
                    "NPZ expanded size exceeds 16 MiB/member or 64 MiB/archive.",
                ));
            }
            keys.insert(key.to_owned(), index);
        }
        Ok(Self { keys, zip })
    }
    pub fn member(&mut self, key: &str) -> Result<Vec<u8>> {
        let index = *self
            .keys
            .get(key)
            .ok_or_else(|| fail(format!("NPZ has no exact array key {key:?}.")))?;
        let member = self
            .zip
            .by_index(index)
            .map_err(|e| fail(format!("NPZ member open failed: {e}")))?;
        let expected = member.size();
        let mut out = Vec::new();
        member
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut out)
            .map_err(|e| fail(format!("NPZ decompression/CRC check failed: {e}")))?;
        if out.len() > MAX_BYTES || out.len() as u64 != expected {
            return Err(fail(
                "NPZ actual decompressed length differs from its bounded declaration.",
            ));
        }
        Ok(out)
    }
}
