use hypr_conf::{
    ConfigMetaSpec, TYPE_KEY, discover_config_files, parse_metadata_header, resolve_config_path,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn parses_header_keys() {
    let content = r#"# hypr metadata
# type = bar

[layout]
left = 33
"#;

    let parsed = parse_metadata_header(content);
    assert_eq!(parsed.get(TYPE_KEY), Some(&"bar".to_string()));
}

#[test]
fn discovers_renamed_config_by_metadata() {
    let dir = tempdir().expect("tempdir");
    let nested = dir.path().join("hypr").join("split");
    fs::create_dir_all(&nested).expect("create dirs");

    let config_path = nested.join("theme-main.conf");
    fs::write(
        &config_path,
        "# hypr metadata\n# type = theme\n[theme]\nname = \"x\"\n",
    )
    .expect("write config");

    let spec = ConfigMetaSpec::for_type("theme", &["conf"]);
    let found = discover_config_files(dir.path(), &spec);

    assert_eq!(found, vec![config_path]);
}

#[test]
fn fallback_wins_when_present() {
    let dir = tempdir().expect("tempdir");
    let fallback = dir.path().join("hyprbar.conf");
    fs::write(&fallback, "# hypr metadata\n# type = bar\n").expect("write fallback");

    let renamed = dir.path().join("custom.conf");
    fs::write(&renamed, "# hypr metadata\n# type = bar\n").expect("write renamed");

    let spec = ConfigMetaSpec::for_type("bar", &["conf"]);
    let resolved = resolve_config_path(dir.path(), &fallback, &spec);
    assert_eq!(resolved, fallback);
}
