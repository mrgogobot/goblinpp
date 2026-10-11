//! Bounded provenance links in standard PNG/SVG, not a new image codec.
//! Metadata is public and removable. Only a trusted receipt/signature can
//! authenticate an otherwise internally consistent replacement.
use crate::error::{GoblinError, Result};
use crate::hashing::canonical_json_bytes;
use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "goblin.plot-provenance.v1";
const MAX_METADATA_BYTES: usize = 4096;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
// iTXt keyword, compression flag/method, empty language and translated keyword.
const ITXT_PREFIX: &[u8] = b"Goblin++ provenance\0\0\0\0\0";
const SVG_OPEN: &str = "<metadata id=\"goblin-provenance\">";
const SVG_CLOSE: &str = "</metadata>\n";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub schema: String,
    pub plot_policy: String,
    pub manifest_sha256: String,
    pub evidence_name: String,
}
impl Provenance {
    pub fn new(digest: &str, evidence: &str) -> Self {
        Self {
            schema: SCHEMA.into(),
            plot_policy: crate::plots::POLICY.into(),
            manifest_sha256: digest.into(),
            evidence_name: evidence.into(),
        }
    }
    fn validate(&self) -> Result<()> {
        if self.schema != SCHEMA || self.plot_policy != crate::plots::POLICY {
            return Err(fail(
                "Unsupported embedded provenance schema or plot policy.",
            ));
        }
        if self.manifest_sha256.len() != 64
            || !self
                .manifest_sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(fail("Embedded provenance requires a lowercase SHA-256."));
        }
        crate::output::validate_name(&self.evidence_name)?;
        if !self.evidence_name.ends_with(".plot.json") {
            return Err(fail(
                "Embedded provenance requires a plot evidence filename.",
            ));
        }
        Ok(())
    }
}
fn fail(message: &str) -> GoblinError {
    GoblinError::artifact(message)
}

pub fn embed(figure: &[u8], png: bool, provenance: &Provenance) -> Result<Vec<u8>> {
    provenance.validate()?;
    let json = canonical_json_bytes(provenance)?;
    if json.len() > MAX_METADATA_BYTES {
        return Err(fail("Embedded plot provenance exceeds 4096 bytes."));
    }
    if png {
        // The private renderer always starts with the standard 13-byte IHDR.
        if figure.len() < 33
            || !figure.starts_with(PNG_SIGNATURE)
            || figure.get(8..16) != Some(b"\0\0\0\rIHDR")
        {
            return Err(fail("Invalid generated PNG header."));
        }
        let mut bytes = figure[..33].to_vec();
        let mut text = ITXT_PREFIX.to_vec();
        text.extend_from_slice(&json);
        crate::output::png_chunk(&mut bytes, b"iTXt", &text);
        bytes.extend_from_slice(&figure[33..]);
        Ok(bytes)
    } else {
        let text = std::str::from_utf8(figure).map_err(|_| fail("Invalid SVG encoding."))?;
        let end = text
            .find('>')
            .ok_or_else(|| fail("Invalid generated SVG header."))?
            + 1;
        let json = std::str::from_utf8(&json).unwrap();
        Ok(format!(
            "{}{SVG_OPEN}{}{SVG_CLOSE}{}",
            &text[..end],
            crate::output::xml(json),
            &text[end..]
        )
        .into_bytes())
    }
}

/// Only Goblin++'s bounded profile is recognized. No decompression, XML entity
/// expansion, external references, image decoding or network requests occur.
pub fn extract(figure: &[u8], png: bool) -> Result<Provenance> {
    if figure.len() > crate::output::MAX_OUTPUT_BYTES {
        return Err(fail("Plot provenance figure byte limit exceeded."));
    }
    if png {
        extract_png(figure)
    } else {
        extract_svg(figure)
    }
}
fn parse(json: &[u8]) -> Result<Provenance> {
    if json.len() > MAX_METADATA_BYTES {
        return Err(fail("Embedded plot provenance exceeds 4096 bytes."));
    }
    let value: Provenance = serde_json::from_slice(json)?;
    value.validate()?;
    if canonical_json_bytes(&value)? != json {
        return Err(fail("Embedded provenance is not canonical JSON."));
    }
    Ok(value)
}
fn extract_png(figure: &[u8]) -> Result<Provenance> {
    if !figure.starts_with(PNG_SIGNATURE) {
        return Err(fail("Invalid PNG signature for plot provenance."));
    }
    let mut offset = 8;
    let mut found = None;
    for index in 0..4096 {
        let header = figure
            .get(offset..offset + 8)
            .ok_or_else(|| fail("Truncated PNG chunk header."))?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(length)
            .and_then(|n| n.checked_add(12))
            .filter(|n| *n <= figure.len())
            .ok_or_else(|| fail("Truncated or oversized PNG chunk."))?;
        let kind = &header[4..];
        let data = &figure[offset + 8..end - 4];
        let crc = u32::from_be_bytes(figure[end - 4..end].try_into().unwrap());
        if crate::output::crc32(&figure[offset + 4..end - 4]) != crc {
            return Err(fail("PNG chunk checksum mismatch."));
        }
        if index == 0 && (kind != b"IHDR" || length != 13) {
            return Err(fail("PNG must begin with a 13-byte IHDR."));
        }
        if kind == b"iTXt" && data.starts_with(b"Goblin++ provenance\0") {
            if found.is_some() {
                return Err(fail("Duplicate embedded plot provenance."));
            }
            let json = data.strip_prefix(ITXT_PREFIX).ok_or_else(|| {
                fail("Unsupported PNG provenance encoding; only uncompressed iTXt is permitted.")
            })?;
            found = Some(parse(json)?);
        }
        if kind == b"IEND" {
            if !data.is_empty() || end != figure.len() {
                return Err(fail("Invalid PNG IEND or trailing bytes."));
            }
            return found
                .ok_or_else(|| fail("PROVENANCE_MISSING: PNG has no Goblin++ provenance link."));
        }
        offset = end;
    }
    Err(fail("PNG provenance chunk limit exceeded."))
}
fn extract_svg(figure: &[u8]) -> Result<Provenance> {
    let text = std::str::from_utf8(figure).map_err(|_| fail("Invalid SVG encoding."))?;
    let end = text
        .find('>')
        .filter(|n| *n < MAX_METADATA_BYTES)
        .ok_or_else(|| fail("Invalid SVG root header."))?
        + 1;
    if !text.starts_with("<svg ") {
        return Err(fail("Unsupported SVG provenance profile."));
    }
    let after = text[end..].strip_prefix(SVG_OPEN).ok_or_else(|| {
        fail("PROVENANCE_MISSING: SVG has no canonical Goblin++ provenance link.")
    })?;
    let close = after
        .find(SVG_CLOSE)
        .filter(|n| *n <= MAX_METADATA_BYTES * 6)
        .ok_or_else(|| fail("Missing or oversized SVG provenance metadata."))?;
    if after[close + SVG_CLOSE.len()..].contains(SVG_OPEN) {
        return Err(fail("Duplicate or ambiguous SVG provenance marker."));
    }
    let escaped = &after[..close];
    // Decode only the five predefined XML entities emitted by output::xml.
    // Ampersand is last: literal entity-like text is never expanded twice.
    let json = escaped
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    let value = parse(json.as_bytes())?;
    if crate::output::xml(&json) != escaped {
        return Err(fail("Noncanonical SVG provenance escaping."));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn link() -> Provenance {
        Provenance::new(&"a".repeat(64), "a.png.plot.json")
    }
    fn png_with_text(text: &[u8]) -> Vec<u8> {
        let original = crate::output::encode_png(1, 1, &[0, 0, 0]);
        let mut bytes = original[..33].to_vec();
        crate::output::png_chunk(&mut bytes, b"iTXt", text);
        bytes.extend_from_slice(&original[33..]);
        bytes
    }
    #[test]
    fn invalid_compressed_png_metadata_is_not_decompressed() {
        let mut text = ITXT_PREFIX.to_vec();
        text[b"Goblin++ provenance\0".len()] = 1;
        text.extend_from_slice(&canonical_json_bytes(&link()).unwrap());
        assert!(
            extract(&png_with_text(&text), true)
                .unwrap_err()
                .to_string()
                .contains("encoding")
        );
    }
    #[test]
    fn malformed_truncated_png_headers_lengths_crc_and_trailing_bytes_refuse() {
        let original = crate::output::encode_png(1, 1, &[0, 0, 0]);
        let image = embed(&original, true, &link()).unwrap();
        for length in [0, 1, 7, 8, 12, 16, 32, 33, 40, image.len() - 1] {
            assert!(extract(&image[..length], true).is_err());
        }
        let mut huge = image.clone();
        huge[33..37].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(extract(&huge, true).is_err());
        let mut crc = image.clone();
        crc[29] ^= 1;
        assert!(
            extract(&crc, true)
                .unwrap_err()
                .to_string()
                .contains("checksum")
        );
        let mut trailing = image;
        trailing.push(0);
        assert!(
            extract(&trailing, true)
                .unwrap_err()
                .to_string()
                .contains("trailing")
        );
        assert!(embed(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", true, &link()).is_err());
    }
    #[test]
    fn oversized_unknown_noncanonical_duplicate_json_fields_refuse() {
        let canonical = canonical_json_bytes(&link()).unwrap();
        let mut cases = vec![vec![b' '; MAX_METADATA_BYTES + 1]];
        let mut spaced = canonical.clone();
        spaced.push(b' ');
        cases.push(spaced);
        let mut extra: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
        extra["unexpected"] = serde_json::json!(1);
        cases.push(canonical_json_bytes(&extra).unwrap());
        extra.as_object_mut().unwrap().remove("unexpected");
        extra["schema"] = serde_json::json!("unknown");
        cases.push(canonical_json_bytes(&extra).unwrap());
        cases.push(
            format!(
                "{{\"schema\":\"{SCHEMA}\",{}",
                std::str::from_utf8(&canonical)
                    .unwrap()
                    .trim_start_matches('{')
            )
            .into_bytes(),
        );
        for json in cases {
            let mut text = ITXT_PREFIX.to_vec();
            text.extend_from_slice(&json);
            assert!(extract(&png_with_text(&text), true).is_err());
        }
    }
    #[test]
    fn unsafe_paths_and_invalid_hashes_refuse() {
        for name in ["../a.plot.json", "/a.plot.json", "a.txt", "a\n.plot.json"] {
            let mut value = link();
            value.evidence_name = name.into();
            assert!(value.validate().is_err());
        }
        for hash in ["A".repeat(64), "a".repeat(63), "g".repeat(64)] {
            let mut value = link();
            value.manifest_sha256 = hash;
            assert!(value.validate().is_err());
        }
    }
    #[test]
    fn invalid_svg_entities_utf8_and_oversized_metadata_refuse() {
        assert!(extract(&[0xff], false).is_err());
        let original = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>\n";
        let image = embed(original, false, &link()).unwrap();
        let text = std::str::from_utf8(&image).unwrap();
        assert!(extract(text.replace("&quot;", "&#34;").as_bytes(), false).is_err());
        assert!(extract(text.replace("</metadata>", "").as_bytes(), false).is_err());
        let huge = format!(
            "<svg >{SVG_OPEN}{}{SVG_CLOSE}</svg>",
            " ".repeat(MAX_METADATA_BYTES * 6 + 1)
        );
        assert!(extract(huge.as_bytes(), false).is_err());
        assert!(
            extract(
                b"<!DOCTYPE svg [<!ENTITY a SYSTEM 'https://example.invalid'>]><svg />",
                false
            )
            .is_err()
        );
    }
}
