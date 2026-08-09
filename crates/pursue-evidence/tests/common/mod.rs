//! Shared helpers for pursue-evidence integration tests.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Creates a unique temporary directory for a test and returns its path.
pub fn temp_dir(label: &str) -> PathBuf {
    let unique = format!(
        "pursue-it-{label}-{}-{:?}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    std::env::temp_dir().join(unique)
}

/// Removes a temporary directory tree, ignoring errors.
pub fn cleanup(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}
