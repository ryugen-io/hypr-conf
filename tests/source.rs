use hypr_conf::{
    collect_source_graph, expand_source_expression_to_path, extract_sources, parse_source_value,
    resolve_source_targets,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn parses_source_value_with_quotes_and_comments() {
    let line = r#"source = "~/hypr/file.conf" # comment"#;
    let parsed = parse_source_value(line);
    assert_eq!(parsed, Some("~/hypr/file.conf"));
}

#[test]
fn extracts_sources_and_strips_lines() {
    let content = r#"
source = "./a.conf"

[section]
key = "value"
source = "$HOME/b.conf"
"#;

    let (sources, remaining) = extract_sources(content);
    assert_eq!(
        sources,
        vec!["./a.conf".to_string(), "$HOME/b.conf".to_string()]
    );
    assert!(remaining.contains("[section]"));
    assert!(!remaining.contains("source ="));
}

#[test]
fn expands_and_resolves_source_targets() {
    let dir = tempdir().expect("tempdir");
    let home = dir.path().join("home");
    let base = dir.path().join("base");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&base).expect("create base");

    let home_cfg = home.join("home.conf");
    let rel_cfg = base.join("rel.conf");
    let glob_a = base.join("glob-a.conf");
    let glob_b = base.join("glob-b.conf");
    fs::write(&home_cfg, "").expect("write home cfg");
    fs::write(&rel_cfg, "").expect("write rel cfg");
    fs::write(&glob_a, "").expect("write glob a");
    fs::write(&glob_b, "").expect("write glob b");

    let expanded = expand_source_expression_to_path("~/home.conf", &base, &home);
    assert_eq!(expanded, home_cfg);

    let resolved_rel = resolve_source_targets("rel.conf", &base, &home);
    assert_eq!(resolved_rel, vec![rel_cfg]);

    let mut resolved_glob = resolve_source_targets("glob-*.conf", &base, &home);
    resolved_glob.sort();
    assert_eq!(resolved_glob, vec![glob_a, glob_b]);
}

#[test]
fn collects_source_graph_cycle_safe() {
    let dir = tempdir().expect("tempdir");
    let a = dir.path().join("a.conf");
    let b = dir.path().join("b.conf");
    let c = dir.path().join("c.conf");

    fs::write(&a, r#"source = "b.conf""#).expect("write a");
    fs::write(&b, r#"source = "c.conf""#).expect("write b");
    fs::write(&c, r#"source = "a.conf""#).expect("write c");

    let graph = collect_source_graph(&a, dir.path());
    assert_eq!(graph.len(), 3);
    assert!(graph.contains(&a));
    assert!(graph.contains(&b));
    assert!(graph.contains(&c));
}
