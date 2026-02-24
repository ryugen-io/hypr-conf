use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Parse `source = ...` from a single config line.
#[must_use]
pub fn parse_source_value(line: &str) -> Option<&str> {
    let clean = strip_comment(line).trim();
    if clean.is_empty() {
        return None;
    }

    let (lhs, rhs) = clean.split_once('=')?;
    if lhs.trim() != "source" {
        return None;
    }

    let value = rhs.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() {
        return None;
    }

    Some(value)
}

/// Extract all `source = ...` directives and return the remaining content.
///
/// This is useful for TOML configs where `source` is a Hyprland extension
/// and should not be deserialized as a TOML key.
#[must_use]
pub fn extract_sources(content: &str) -> (Vec<String>, String) {
    let mut sources = Vec::new();
    let mut remaining = String::new();

    for line in content.lines() {
        if let Some(source) = parse_source_value(line) {
            sources.push(source.to_string());
            continue;
        }

        remaining.push_str(line);
        remaining.push('\n');
    }

    (sources, remaining)
}

/// Returns `true` when the value contains glob wildcard syntax.
#[must_use]
pub fn has_glob_chars(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

/// Expand `source` expression into an absolute or base-relative path.
///
/// Supports:
/// - `${HOME}` and `$HOME`
/// - `~/...`
/// - relative paths resolved from `base_dir`
#[must_use]
pub fn expand_source_expression_to_path(value: &str, base_dir: &Path, home_dir: &Path) -> PathBuf {
    let mut out = value.trim().to_string();

    if let Some(home) = home_dir.to_str() {
        out = out.replace("${HOME}", home);
        out = out.replace("$HOME", home);
    }

    if let Some(rest) = out.strip_prefix("~/") {
        return home_dir.join(rest);
    }

    let path = PathBuf::from(&out);
    if path.is_absolute() {
        return path;
    }

    base_dir.join(path)
}

/// Resolve one `source` expression to concrete file targets.
///
/// Non-glob values always return a single path (even when the file does not exist).
/// Glob values return all existing files matching the pattern.
#[must_use]
pub fn resolve_source_targets(value: &str, base_dir: &Path, home_dir: &Path) -> Vec<PathBuf> {
    let expanded = expand_source_expression_to_path(value, base_dir, home_dir);
    let pattern = expanded.to_string_lossy();

    if has_glob_chars(&pattern) {
        match glob::glob(&pattern) {
            Ok(paths) => paths
                .flatten()
                .filter(|path| path.exists() && path.is_file())
                .collect(),
            Err(_) => Vec::new(),
        }
    } else {
        vec![expanded]
    }
}

/// Returns `true` when `target` is covered by the source expression.
#[must_use]
pub fn source_expression_matches_path(
    value: &str,
    base_dir: &Path,
    home_dir: &Path,
    target: &Path,
) -> bool {
    let expanded = expand_source_expression_to_path(value, base_dir, home_dir);
    let expanded_str = expanded.to_string_lossy();

    if has_glob_chars(&expanded_str) {
        if let Ok(pattern) = glob::Pattern::new(&expanded_str) {
            return pattern.matches_path(target);
        }
        return false;
    }

    expanded == target
}

/// Collect a recursive Hyprland `source = ...` graph starting from `root`.
///
/// Cycles are handled by deduplication; each file appears at most once.
#[must_use]
pub fn collect_source_graph(root: &Path, home_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut seen = HashSet::new();

    while let Some(file) = stack.pop() {
        let canonical = file.canonicalize().unwrap_or(file.clone());
        if !seen.insert(canonical) {
            continue;
        }

        out.push(file.clone());

        let content = match fs::read_to_string(&file) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let base_dir = file.parent().unwrap_or_else(|| Path::new("/"));
        for line in content.lines() {
            if let Some(source_value) = parse_source_value(line) {
                for resolved in resolve_source_targets(source_value, base_dir, home_dir) {
                    if resolved.exists() && resolved.is_file() {
                        stack.push(resolved);
                    }
                }
            }
        }
    }

    out
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(before, _)| before)
        .unwrap_or(line)
}
