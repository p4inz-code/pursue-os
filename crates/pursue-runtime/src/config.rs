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
///
/// # Path Resolution
/// Optional path fields (`case_dir`, `browser_profile_dir`, `report_dir`,
/// `log_file`) default to subdirectories of `data_dir` when absent. Use the
/// `resolve_*` methods to obtain the effective path for each facility.
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
    /// Directory for case repositories. Defaults to `{data_dir}/cases`.
    #[serde(default)]
    pub case_dir: Option<PathBuf>,
    /// Directory for browser profile storage. Defaults to `{data_dir}/profiles`.
    #[serde(default)]
    pub browser_profile_dir: Option<PathBuf>,
    /// Directory for exported reports. Defaults to `{data_dir}/reports`.
    #[serde(default)]
    pub report_dir: Option<PathBuf>,
    /// Path to the structured JSON log file. Defaults to
    /// `{log_base}/runtime.log` where `log_base` is `/var/log/pursue` on
    /// Linux or `{data_dir}/logs` elsewhere.
    #[serde(default)]
    pub log_file: Option<PathBuf>,
}

/// Default base data directory used when `data_dir` is `None`.
#[cfg(target_os = "linux")]
fn platform_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/pursue")
}

/// Default base data directory used when `data_dir` is `None`.
#[cfg(not(target_os = "linux"))]
fn platform_data_dir() -> PathBuf {
    std::env::temp_dir().join("pursue-data")
}

impl Config {
    /// The explicit baseline configuration.
    pub fn defaults() -> Self {
        Self {
            version: CONFIG_VERSION,
            log_level: Level::Info,
            data_dir: None,
            ipc_socket_path: None,
            case_dir: None,
            browser_profile_dir: None,
            report_dir: None,
            log_file: None,
        }
    }

    // -- Path resolvers ---------------------------------------------------

    /// Resolved base data directory.
    pub fn resolve_data_dir(&self) -> PathBuf {
        self.data_dir.clone().unwrap_or_else(platform_data_dir)
    }

    /// Resolved case repository directory.
    pub fn resolve_case_dir(&self) -> PathBuf {
        self.case_dir
            .clone()
            .unwrap_or_else(|| self.resolve_data_dir().join("cases"))
    }

    /// Resolved browser profile directory.
    pub fn resolve_browser_profile_dir(&self) -> PathBuf {
        self.browser_profile_dir
            .clone()
            .unwrap_or_else(|| self.resolve_data_dir().join("profiles"))
    }

    /// Resolved report export directory.
    pub fn resolve_report_dir(&self) -> PathBuf {
        self.report_dir
            .clone()
            .unwrap_or_else(|| self.resolve_data_dir().join("reports"))
    }

    /// Resolved log file path.
    pub fn resolve_log_file(&self) -> PathBuf {
        self.log_file.clone().unwrap_or_else(|| {
            #[cfg(target_os = "linux")]
            {
                PathBuf::from("/var/log/pursue/runtime.log")
            }
            #[cfg(not(target_os = "linux"))]
            {
                self.resolve_data_dir().join("logs").join("runtime.log")
            }
        })
    }

    // -- Validation -------------------------------------------------------

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
        self.reject_empty_path(&self.data_dir, "data_dir")?;
        self.reject_empty_path(&self.ipc_socket_path, "ipc_socket_path")?;
        self.reject_empty_path(&self.case_dir, "case_dir")?;
        self.reject_empty_path(&self.browser_profile_dir, "browser_profile_dir")?;
        self.reject_empty_path(&self.report_dir, "report_dir")?;
        self.reject_empty_path(&self.log_file, "log_file")?;
        Ok(())
    }

    fn reject_empty_path(&self, field: &Option<PathBuf>, name: &str) -> Result<()> {
        if let Some(p) = field {
            if p.as_os_str().is_empty() {
                return Err(Error::InvalidInput(format!("{name} must not be empty")));
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
        assert_eq!(config.case_dir, None);
        assert_eq!(config.browser_profile_dir, None);
        assert_eq!(config.report_dir, None);
        assert_eq!(config.log_file, None);
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
        let source = concat!(
            "version = 1\n",
            "log_level = \"trace\"\n",
            "data_dir = \"/var/lib/pursue\"\n",
            "ipc_socket_path = \"/run/pursue/ipc.sock\"\n",
            "case_dir = \"/var/lib/pursue/cases\"\n",
            "browser_profile_dir = \"/var/lib/pursue/profiles\"\n",
            "report_dir = \"/var/lib/pursue/reports\"\n",
            "log_file = \"/var/log/pursue/runtime.log\"\n",
        );
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
        assert_eq!(
            config.case_dir.as_deref(),
            Some(Path::new("/var/lib/pursue/cases"))
        );
        assert_eq!(
            config.browser_profile_dir.as_deref(),
            Some(Path::new("/var/lib/pursue/profiles"))
        );
        assert_eq!(
            config.report_dir.as_deref(),
            Some(Path::new("/var/lib/pursue/reports"))
        );
        assert_eq!(
            config.log_file.as_deref(),
            Some(Path::new("/var/log/pursue/runtime.log"))
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
        assert!(Config::from_toml_str("case_dir = \"\"").is_err());
        assert!(Config::from_toml_str("browser_profile_dir = \"\"").is_err());
        assert!(Config::from_toml_str("report_dir = \"\"").is_err());
        assert!(Config::from_toml_str("log_file = \"\"").is_err());
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

    #[test]
    fn resolver_defaults_derive_from_data_dir() {
        let config = Config {
            data_dir: Some(PathBuf::from("/var/lib/pursue")),
            ..Config::defaults()
        };
        assert_eq!(config.resolve_data_dir(), PathBuf::from("/var/lib/pursue"));
        assert_eq!(
            config.resolve_case_dir(),
            PathBuf::from("/var/lib/pursue/cases")
        );
        assert_eq!(
            config.resolve_browser_profile_dir(),
            PathBuf::from("/var/lib/pursue/profiles")
        );
        assert_eq!(
            config.resolve_report_dir(),
            PathBuf::from("/var/lib/pursue/reports")
        );
    }

    #[test]
    fn resolver_explicit_overrides_take_precedence() {
        let config = Config {
            data_dir: Some(PathBuf::from("/var/lib/pursue")),
            case_dir: Some(PathBuf::from("/custom/cases")),
            browser_profile_dir: Some(PathBuf::from("/custom/profiles")),
            report_dir: Some(PathBuf::from("/custom/reports")),
            log_file: Some(PathBuf::from("/custom/pursue.log")),
            ..Config::defaults()
        };
        assert_eq!(config.resolve_case_dir(), PathBuf::from("/custom/cases"));
        assert_eq!(
            config.resolve_browser_profile_dir(),
            PathBuf::from("/custom/profiles")
        );
        assert_eq!(
            config.resolve_report_dir(),
            PathBuf::from("/custom/reports")
        );
        assert_eq!(
            config.resolve_log_file(),
            PathBuf::from("/custom/pursue.log")
        );
    }

    #[test]
    fn new_fields_default_to_none_on_partial_toml() {
        let config = Config::from_toml_str("data_dir = \"/data\"").unwrap();
        assert_eq!(config.case_dir, None);
        assert_eq!(config.browser_profile_dir, None);
        assert_eq!(config.report_dir, None);
        assert_eq!(config.log_file, None);
        // Resolvers still produce sane paths
        assert_eq!(config.resolve_case_dir(), PathBuf::from("/data/cases"));
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
