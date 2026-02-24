use crate::resolve_source_targets;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

#[derive(Debug)]
pub enum IncludeLoadError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    CyclicInclude(PathBuf),
}

impl fmt::Display for IncludeLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Parse(e) => write!(f, "TOML parse error: {e}"),
            Self::CyclicInclude(path) => write!(f, "cyclic include: {}", path.display()),
        }
    }
}

impl std::error::Error for IncludeLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
            Self::CyclicInclude(_) => None,
        }
    }
}

impl From<std::io::Error> for IncludeLoadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<toml::de::Error> for IncludeLoadError {
    fn from(value: toml::de::Error) -> Self {
        Self::Parse(value)
    }
}

/// Load TOML and recursively resolve top-level include patterns.
///
/// Included files are merged recursively, where later merged values overwrite
/// earlier scalar values, while table values merge by key.
pub fn load_toml_with_includes(
    path: &Path,
    include_key: &str,
    home_dir: &Path,
) -> Result<Value, IncludeLoadError> {
    let mut stack = HashSet::new();
    load_toml_with_includes_inner(path, include_key, home_dir, &mut stack)
}

fn load_toml_with_includes_inner(
    path: &Path,
    include_key: &str,
    home_dir: &Path,
    stack: &mut HashSet<PathBuf>,
) -> Result<Value, IncludeLoadError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !stack.insert(canonical.clone()) {
        return Err(IncludeLoadError::CyclicInclude(canonical));
    }

    let result = (|| -> Result<Value, IncludeLoadError> {
        let content = fs::read_to_string(path)?;
        let mut root_value: Value = toml::from_str(&content)?;

        let include_patterns = extract_include_patterns(&root_value, include_key);
        let base_dir = path.parent().unwrap_or_else(|| Path::new("/"));

        for pattern in include_patterns {
            for include_path in resolve_source_targets(&pattern, base_dir, home_dir) {
                if !include_path.exists() || !include_path.is_file() {
                    continue;
                }

                let included =
                    load_toml_with_includes_inner(&include_path, include_key, home_dir, stack)?;
                merge_toml_values(&mut root_value, included);
            }
        }

        Ok(root_value)
    })();

    stack.remove(&canonical);
    result
}

fn extract_include_patterns(root: &Value, include_key: &str) -> Vec<String> {
    let mut include_patterns = Vec::new();
    if let Some(includes) = root.get(include_key).and_then(Value::as_array) {
        for include in includes {
            if let Some(include_str) = include.as_str() {
                include_patterns.push(include_str.to_string());
            }
        }
    }
    include_patterns
}

fn merge_toml_values(base: &mut Value, other: Value) {
    match (base, other) {
        (Value::Table(base_map), Value::Table(other_map)) => {
            for (key, value) in other_map {
                match base_map.get_mut(&key) {
                    Some(base_value) => merge_toml_values(base_value, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (base_value, other_value) => {
            *base_value = other_value;
        }
    }
}
