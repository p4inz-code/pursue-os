//! Content addressing: evidence identity derived from SHA-256.

use pursue_core::{Error, Result, hex};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

/// A content address: the SHA-256 digest of a piece of evidence.
///
/// Two artifacts with identical bytes share an address (content addressing),
/// which makes identity independent of location and rename.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentAddress([u8; 32]);

impl ContentAddress {
    /// Computes the content address of `data`.
    pub fn hash(data: &[u8]) -> Self {
        Self::from_digest(Sha256::digest(data).into())
    }

    /// Constructs an address from a raw 32-byte SHA-256 digest.
    ///
    /// Crate-internal: addresses must normally be produced by [`Self::hash`].
    pub(crate) fn from_digest(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Parses a content address from a 64-character hex string.
    pub fn from_hex(s: &str) -> Result<Self> {
        let bytes = hex::decode(s)
            .map_err(|_| Error::InvalidContentAddress(format!("invalid hex: {s:?}")))?;
        if bytes.len() != 32 {
            return Err(Error::InvalidContentAddress(format!(
                "expected 32 bytes (64 hex chars), got {} chars",
                s.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Returns the raw 32-byte digest.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hex-encodes the address.
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl std::fmt::Display for ContentAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl std::fmt::Debug for ContentAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sha256:{}", self.to_hex())
    }
}

impl Serialize for ContentAddress {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for ContentAddress {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ContentAddress::from_hex(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::ContentAddress;

    /// NIST SHA-256 test vector for the ASCII string "abc".
    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn hash_matches_known_vector() {
        let addr = ContentAddress::hash(b"abc");
        assert_eq!(addr.to_hex(), ABC_SHA256);
    }

    #[test]
    fn hash_empty_input() {
        // SHA-256 of the empty input (well-known digest).
        let addr = ContentAddress::hash(b"");
        assert_eq!(
            addr.to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn from_hex_roundtrips() {
        let addr = ContentAddress::hash(b"evidence");
        let parsed = ContentAddress::from_hex(&addr.to_hex()).unwrap();
        assert_eq!(parsed, addr);
        assert_eq!(parsed.as_bytes(), addr.as_bytes());
    }

    #[test]
    fn from_hex_rejects_wrong_length() {
        assert!(ContentAddress::from_hex("abcd").is_err());
        assert!(ContentAddress::from_hex("").is_err());
    }

    #[test]
    fn from_hex_rejects_invalid_characters() {
        let hex = ABC_SHA256.replace('a', "z");
        assert!(ContentAddress::from_hex(&hex).is_err());
    }

    #[test]
    fn serde_roundtrip_as_hex_string() {
        let addr = ContentAddress::hash(b"serialize me");
        let json = serde_json::to_string(&addr).unwrap();
        assert_eq!(json, format!("\"{}\"", addr.to_hex()));
        let back: ContentAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(back, addr);
    }

    #[test]
    fn serde_rejects_malformed() {
        assert!(serde_json::from_str::<ContentAddress>("\"nope\"").is_err());
    }
}
