//! Log severity levels.

use pursue_core::{Error, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Log severity levels, ordered from most to least severe.
///
/// The derived ordering follows declaration order, so
/// `Error < Warn < Info < Debug < Trace`. A logger with threshold `L` emits
/// records at `level` when `level <= L`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Level {
    /// An error condition; the operation could not be completed.
    Error,
    /// A potentially harmful condition the system can continue past.
    Warn,
    /// Normal operational information.
    #[default]
    Info,
    /// Detailed diagnostic information for developers.
    Debug,
    /// Very fine-grained diagnostic detail.
    Trace,
}

impl Level {
    /// The canonical lowercase name of this level.
    pub const fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }
}

/// Parses a level name (trimmed, case-insensitive).
///
/// Accepts `error`, `warn`, `warning`, `info`, `debug`, and `trace`.
impl std::str::FromStr for Level {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "error" => Ok(Level::Error),
            "warn" | "warning" => Ok(Level::Warn),
            "info" => Ok(Level::Info),
            "debug" => Ok(Level::Debug),
            "trace" => Ok(Level::Trace),
            other => Err(Error::InvalidInput(format!("unknown log level: {other:?}"))),
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Level {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse::<Level>().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Level;
    use pursue_core::Error;

    #[test]
    fn names_roundtrip() {
        for level in [
            Level::Error,
            Level::Warn,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ] {
            assert_eq!(level.as_str().parse::<Level>().unwrap(), level);
        }
    }

    #[test]
    fn parsing_is_case_insensitive_and_trimmed() {
        assert_eq!("  DEBUG ".parse::<Level>().unwrap(), Level::Debug);
        assert_eq!("WARNING".parse::<Level>().unwrap(), Level::Warn);
    }

    #[test]
    fn unknown_level_is_rejected() {
        let err = "loud".parse::<Level>().unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn severity_ordering() {
        assert!(Level::Error < Level::Warn);
        assert!(Level::Warn < Level::Info);
        assert!(Level::Info < Level::Debug);
        assert!(Level::Debug < Level::Trace);
    }

    #[test]
    fn serde_roundtrip() {
        let json = serde_json::to_string(&Level::Debug).unwrap();
        assert_eq!(json, "\"debug\"");
        let back: Level = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Level::Debug);
        assert!(serde_json::from_str::<Level>("\"loud\"").is_err());
    }
}
