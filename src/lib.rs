//! Shared config metadata utilities for hypr* tools.
//!
//! `hypr-conf` lets tools identify config files by a simple human-readable
//! header instead of hard-coded filenames.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Primary metadata key for config type.
pub const TYPE_KEY: &str = "type";
/// Required first-line marker for metadata-enabled config files.
pub const HEADER_LINE: &str = "# hypr metadata";

/// Metadata contract for selecting config files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigMetaSpec<'a> {
    /// Logical config type (for example `theme`, `bar`, `logging`, `deck`).
    pub config_type: &'a str,
    /// Allowed file extensions (without leading dot).
    pub extensions: &'a [&'a str],
}

impl<'a> ConfigMetaSpec<'a> {
    /// Convenience constructor.
    #[must_use]
    pub fn for_type(config_type: &'a str, extensions: &'a [&'a str]) -> Self {
        Self {
            config_type,
            extensions,
        }
    }
}

/// Parsed metadata values from a file header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigMetadata {
    pub config_type: String,
}

/// Parse comment header key/value lines from first 64 lines.
///
/// Accepted separators: `=` and `:`.
#[must_use]
pub fn parse_metadata_header(content: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut lines = content.lines();

    let Some(first_line) = lines.next() else {
        return out;
    };
    let first_line = first_line.trim_start_matches('\u{feff}').trim();
    if !first_line.eq_ignore_ascii_case(HEADER_LINE) {
        return out;
    }

    for line in lines.take(63) {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            continue;
        }

        let body = trimmed.trim_start_matches('#').trim();
        let pair = body
            .split_once('=')
            .or_else(|| body.split_once(':'));
        let Some((key, value)) = pair else {
            continue;
        };

        let key = key.trim().to_lowercase();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            out.insert(key, value.to_string());
        }
    }

    out
}

/// Parse required metadata keys from content.
#[must_use]
pub fn metadata_from_content(content: &str) -> Option<ConfigMetadata> {
    let parsed = parse_metadata_header(content);
    let config_type = parsed.get(TYPE_KEY)?.clone();

    Some(ConfigMetadata { config_type })
}

/// Check whether file content matches the expected metadata spec.
#[must_use]
pub fn matches_spec(content: &str, spec: &ConfigMetaSpec<'_>) -> bool {
    matches!(
        metadata_from_content(content),
        Some(meta) if meta.config_type == spec.config_type
    )
}

/// Check whether a file path matches extension + metadata requirements.
#[must_use]
pub fn file_matches(path: &Path, spec: &ConfigMetaSpec<'_>) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    if !spec
        .extensions
        .iter()
        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
    {
        return false;
    }

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return false,
    };

    matches_spec(&content, spec)
}

/// Discover matching config files recursively below `root`.
///
/// Returned paths are sorted for deterministic behavior.
#[must_use]
pub fn discover_config_files(root: &Path, spec: &ConfigMetaSpec<'_>) -> Vec<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut matches = Vec::new();

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if file_matches(&path, spec) {
                matches.push(path);
            }
        }
    }

    matches.sort();
    matches
}

/// Resolve config path using metadata discovery with deterministic fallback.
///
/// Resolution order:
/// 1. `fallback` if it exists and matches the metadata spec
/// 2. first metadata-matching file below `root`
/// 3. `fallback` (even if missing)
#[must_use]
pub fn resolve_config_path(root: &Path, fallback: &Path, spec: &ConfigMetaSpec<'_>) -> PathBuf {
    resolve_config_path_strict(root, fallback, spec).unwrap_or_else(|| fallback.to_path_buf())
}

/// Resolve config path with strict metadata enforcement.
///
/// Returns `None` when no file below `root` satisfies the metadata spec.
#[must_use]
pub fn resolve_config_path_strict(
    root: &Path,
    fallback: &Path,
    spec: &ConfigMetaSpec<'_>,
) -> Option<PathBuf> {
    if fallback.exists() && file_matches(fallback, spec) {
        return Some(fallback.to_path_buf());
    }

    discover_config_files(root, spec).into_iter().next()
}
