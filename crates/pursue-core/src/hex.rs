//! Minimal, dependency-free hexadecimal encoding used by content addressing.

use crate::{Error, Result};

/// Encodes bytes as a lowercase hexadecimal string.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Decodes a hexadecimal string (lowercase or uppercase).
///
/// Rejects odd-length strings and non-hex characters.
pub fn decode(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        return Err(Error::InvalidHex(format!(
            "odd length {} (expected even number of hex digits)",
            s.len()
        )));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16)
            .map_err(|_| Error::InvalidHex(format!("invalid hex digit in {:?}", &s[i..i + 2])))?;
        out.push(byte);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn encode_produces_lowercase_hex() {
        assert_eq!(encode(&[0x00, 0xff, 0x10]), "00ff10");
        assert_eq!(encode(&[]), "");
    }

    #[test]
    fn decode_roundtrips() {
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        assert_eq!(decode(&encode(&bytes)).unwrap(), bytes);
    }

    #[test]
    fn decode_accepts_uppercase() {
        assert_eq!(decode("DEADBEEF").unwrap(), [0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn decode_rejects_odd_length() {
        assert!(decode("abc").is_err());
    }

    #[test]
    fn decode_rejects_invalid_characters() {
        assert!(decode("zz").is_err());
        assert!(decode("0g").is_err());
    }
}
