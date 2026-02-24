use hypr_conf::{IncludeLoadError, load_toml_with_includes};
use std::fs;
use tempfile::tempdir;
use toml::Value;

#[test]
fn loads_toml_with_recursive_includes() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path().join("root.conf");
    let includes = dir.path().join("includes");
    fs::create_dir_all(&includes).expect("create includes");

    let child = includes.join("child.conf");
    let nested = includes.join("nested.conf");

    fs::write(
        &root,
        r##"
include = ["includes/*.conf"]
[style]
bg = "#111111"
"##,
    )
    .expect("write root");

    fs::write(
        &child,
        r##"
include = ["nested.conf"]
[style]
fg = "#ffffff"
"##,
    )
    .expect("write child");

    fs::write(
        &nested,
        r##"
[layout]
strategy = "grid"
"##,
    )
    .expect("write nested");

    let loaded = load_toml_with_includes(&root, "include", dir.path()).expect("load includes");

    let style = loaded
        .get("style")
        .and_then(Value::as_table)
        .expect("style");
    let layout = loaded
        .get("layout")
        .and_then(Value::as_table)
        .expect("layout");

    assert_eq!(style.get("bg").and_then(Value::as_str), Some("#111111"));
    assert_eq!(style.get("fg").and_then(Value::as_str), Some("#ffffff"));
    assert_eq!(layout.get("strategy").and_then(Value::as_str), Some("grid"));
}

#[test]
fn cycles_in_include_chain_return_error() {
    let dir = tempdir().expect("tempdir");
    let a = dir.path().join("a.conf");
    let b = dir.path().join("b.conf");

    fs::write(&a, r#"include = ["b.conf"]"#).expect("write a");
    fs::write(&b, r#"include = ["a.conf"]"#).expect("write b");

    let err = load_toml_with_includes(&a, "include", dir.path()).expect_err("expected cycle");
    assert!(matches!(err, IncludeLoadError::CyclicInclude(_)));
}
