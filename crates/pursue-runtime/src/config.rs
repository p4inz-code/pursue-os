//! Typed configuration foundation.
//!
//! # Design
//! - **Defaults vs. configuration** — [`Config::defaults`] defines the
//!   explicit baseline. Loading a TOML document overlays only the keys the
//!   document contains (every field carries `#[serde(default)]`), so partial
//!   user/system configuration is deterministic and additive.
//! - **Format** — TOML via the mature `toml` crate; output is deterministic
//!   pretty-printed TOML.
//! - **Validation** — [`Config::validate`] rejects unsupported versions and
//!   present-but-empty paths.
//! - **Secrets** — configuration never contains secrets. Credentials, tokens,
//!   and keys are intentionally out of scope here; they must be supplied via
//!   environment or OS secret facilities in later phases.

use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::log::Level;

/// The only supported configuration format version.
pub const CONFIG_VERSION: u32 = 1;

fn default_version() -> u32 {
    CONFIG_VERSION
}

fn default_log_level() -> Level {
    Level::Info
}

/// Typed PURSUE core configuration.
///
/// Every field carries `#[serde(default)]`, so a partial TOML document
/// overlays [`Config::defaults`] instead of failing on missing keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Configuration format version; must equal [`CONFIG_VERSION`].
    #[serde(default = "default_version")]
    pub version: u32,
    /// Minimum level emitted by the logging foundation.
    #[serde(default = "default_log_level")]
    pub log_level: Level,
    /// Base directory for application data; `None` selects a platform default
    /// at runtime.
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
    /// Unix-domain-socket path for the IPC service boundary. Only meaningful
    /// on Unix; ignored on other platforms.
    #[serde(default)]
    pub ipc_socket_path: Option<PathBuf>,
}

impl Config {
    /// The explicit baseline configuration.
    pub fn defaults() -> Self {
        Self {
            version: CONFIG_VERSION,
            log_level: Level::Info,
            data_dir: None,
            ipc_socket_path: None,
        }
    }

    /// Validates this configuration.
    ///
    /// Rejects an unsupported `version` and present-but-empty paths.
    pub fn validate(&self) -> Result<()> {
        if self.version != CONFIG_VERSION {
            return Err(Error::InvalidInput(format!(
                "unsupported configuration version {} (expected {CONFIG_VERSION})",
                self.version
            )));
        }
        if let Some(dir) = &self.data_dir {
            if dir.as_os_str().is_empty() {
                return Err(Error::InvalidInput("data_dir must not be empty".into()));
            }
        }
        if let Some(path) = &self.ipc_socket_path {
            if path.as_os_str().is_empty() {
                return Err(Error::InvalidInput(
                    "ipc_socket_path must not be empty".into(),
                ));
            }
        }
        Ok(())
    }

    /// Parses configuration from a TOML string, overlaying defaults for keys
    /// that are absent, then validates the result.
    pub fn from_toml_str(source: &str) -> Result<Self> {
        let config: Config = toml::from_str(source)
            .map_err(|e| Error::InvalidInput(format!("invalid configuration TOML: {e}")))?;
        config.validate()?;
        Ok(config)
    }

    /// Loads configuration from a TOML file.
    pub fn from_toml_file(path: &Path) -> Result<Self> {
        let source = std::fs::read_to_string(path)?;
        Self::from_toml_str(&source)
    }

    /// Serializes this configuration to a deterministic, pretty TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| Error::InvalidInput(format!("configuration serialization failed: {e}")))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::{CONFIG_VERSION, Config};
    use crate::log::Level;
    use std::path::{Path, PathBuf};

    #[test]
    fn defaults_are_sane() {
        let config = Config::defaults();
        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.log_level, Level::Info);
        assert_eq!(config.data_dir, None);
        assert_eq!(config.ipc_socket_path, None);
        config.validate().unwrap();
    }

    #[test]
    fn toml_roundtrip_is_deterministic() {
        let config = Config {
            log_level: Level::Debug,
            data_dir: Some(PathBuf::from("/var/lib/pursue")),
            ..Config::defaults()
        };
        let toml = config.to_toml_string().unwrap();
        assert_eq!(toml, config.to_toml_string().unwrap()); // deterministic
        let parsed = Config::from_toml_str(&toml).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn minimal_toml_uses_defaults_for_missing_keys() {
        let config = Config::from_toml_str("").unwrap();
        assert_eq!(config, Config::defaults());

        let partial = Config::from_toml_str("log_level = \"debug\"").unwrap();
        assert_eq!(partial.log_level, Level::Debug);
        assert_eq!(partial.data_dir, None);
        assert_eq!(partial.version, CONFIG_VERSION);
    }

    #[test]
    fn full_toml_parses_all_fields() {
        let source = "version = 1\nlog_level = \"trace\"\ndata_dir = \"/var/lib/pursue\"\nipc_socket_path = \"/run/pursue/ipc.sock\"\n";
        let config = Config::from_toml_str(source).unwrap();
        assert_eq!(config.log_level, Level::Trace);
        assert_eq!(
            config.data_dir.as_deref(),
            Some(Path::new("/var/lib/pursue"))
        );
        assert_eq!(
            config.ipc_socket_path.as_deref(),
            Some(Path::new("/run/pursue/ipc.sock"))
        );
    }

    #[test]
    fn invalid_toml_is_rejected() {
        assert!(Config::from_toml_str("log_level = [").is_err());
        assert!(Config::from_toml_str("log_level = 42").is_err());
    }

    #[test]
    fn unsupported_version_is_rejected() {
        assert!(Config::from_toml_str("version = 99").is_err());
    }

    #[test]
    fn empty_paths_are_rejected() {
        assert!(Config::from_toml_str("data_dir = \"\"").is_err());
        assert!(Config::from_toml_str("ipc_socket_path = \"\"").is_err());
    }

    #[test]
    fn invalid_log_level_is_rejected() {
        let err = Config::from_toml_str("log_level = \"loud\"").unwrap_err();
        assert!(err.to_string().contains("invalid configuration TOML"));
    }

    #[test]
    fn load_from_file_roundtrips() {
        let dir = temp_dir("config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pursue.toml");
        let config = Config {
            log_level: Level::Warn,
            ..Config::defaults()
        };
        std::fs::write(&path, config.to_toml_string().unwrap()).unwrap();
        let loaded = Config::from_toml_file(&path).unwrap();
        assert_eq!(loaded, config);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let path = temp_dir("config").join("does-not-exist.toml");
        let err = Config::from_toml_file(&path).unwrap_err();
        assert!(matches!(err, pursue_core::Error::Io(_)));
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = format!(
            "pursue-test-{label}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }
}
