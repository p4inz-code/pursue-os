//! Safe report export: path traversal protection, secret scrubbing, and atomic writes.

use pursue_core::{Error, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Validates that an export path is safe and does not attempt directory traversal.
///
/// If `allowed_root` is provided, the validated target must reside within `allowed_root`.
pub fn validate_export_path(target_path: &Path, allowed_root: Option<&Path>) -> Result<PathBuf> {
    let path_str = target_path.to_string_lossy();
    if path_str.contains('\0') {
        return Err(Error::InvalidInput(
            "export path contains forbidden null byte".into(),
        ));
    }

    // Check components for ParentDir ('..')
    for comp in target_path.components() {
        if comp == Component::ParentDir {
            return Err(Error::InvalidInput(
                "export path must not contain parent directory traversal ('..')".into(),
            ));
        }
    }

    let absolute_path = if target_path.is_absolute() {
        target_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(target_path)
    };

    if let Some(root) = allowed_root {
        let root_canon = root.canonicalize().map_err(|e| {
            Error::InvalidInput(format!("failed to canonicalize allowed root: {e}"))
        })?;

        // Ensure parent directory of target resides within allowed root
        let parent = absolute_path
            .parent()
            .ok_or_else(|| Error::InvalidInput("target path has no parent directory".into()))?;

        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }

        let parent_canon = parent.canonicalize().map_err(|e| {
            Error::InvalidInput(format!("failed to canonicalize target parent: {e}"))
        })?;

        if !parent_canon.starts_with(&root_canon) {
            return Err(Error::InvalidInput(format!(
                "export target outside allowed root: target parent {:?} not in {:?}",
                parent_canon, root_canon
            )));
        }
    }

    Ok(absolute_path)
}

/// Scrubs obvious sensitive patterns (e.g. bearer tokens, passwords) from text fields.
pub fn scrub_secrets(text: &str) -> String {
    let mut result = text.to_string();

    let patterns = ["bearer ", "token=", "token: ", "password=", "secret="];
    for prefix in patterns {
        let mut search_from = 0;
        while search_from < result.len() {
            let lower = result[search_from..].to_lowercase();
            if let Some(offset) = lower.find(prefix) {
                let start = search_from + offset + prefix.len();
                let end = result[start..]
                    .find(|c: char| {
                        c.is_whitespace() || c == '"' || c == '\'' || c == ';' || c == ','
                    })
                    .map(|o| start + o)
                    .unwrap_or(result.len());
                if start < end {
                    result.replace_range(start..end, "[REDACTED_SECRET]");
                }
                search_from = start + "[REDACTED_SECRET]".len();
            } else {
                break;
            }
        }
    }

    result
}

/// Atomically writes report payload to the given destination path.
///
/// Writes to a temporary file in the destination directory first and
/// renames it to target to avoid corrupted or partial report files.
pub fn export_report_atomic(content: &[u8], target_path: &Path) -> Result<PathBuf> {
    let parent = target_path.parent().ok_or_else(|| {
        Error::InvalidInput("export target must specify a parent directory".into())
    })?;

    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let temp_name = format!(
        ".tmp_report_{}_{}_{}.part",
        std::process::id(),
        now_unix(),
        fastrand_u32()
    );
    let temp_path = parent.join(temp_name);

    {
        let mut file = File::create(&temp_path)?;
        file.write_all(content)?;
        file.flush()?;
        file.sync_all()?;
    }

    fs::rename(&temp_path, target_path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        Error::Io(e)
    })?;

    Ok(target_path.to_path_buf())
}

fn fastrand_u32() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(12345);
    nanos ^ (std::process::id() << 16)
}
