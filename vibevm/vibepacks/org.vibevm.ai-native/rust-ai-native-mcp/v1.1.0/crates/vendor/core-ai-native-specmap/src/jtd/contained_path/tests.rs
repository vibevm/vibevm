use std::path::Path;

use crate::config::Config;
use crate::scanner::CodeScanner;

use super::super::JtdScanner;
use super::{MAX_DECLARED_DISPLAY, bounded_declared_path};

const URI: &str = "spec://project/modules/path/PROP-001#contained";

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn write_thin_schema(root: &Path) {
    write(
        root,
        "schemas/shared.jtd.json",
        r#"{"metadata":{"x-vocabularies":["shared"]},"ref":"shared"}"#,
    );
}

fn write_external_vocabulary(path: &Path) {
    std::fs::write(
        path,
        format!(
            r#"{{"shared":{{"metadata":{{"x-vocabularies":["tail"],"spec":{{"implements":"{URI}"}}}}}},"tail":{{"type":"string"}}}}"#
        ),
    )
    .unwrap();
}

fn scan(root: &Path, declared: String) -> (usize, usize, Vec<(String, String)>) {
    let cfg = Config {
        schema_roots: vec!["schemas".into()],
        schema_vocabulary: Some(declared),
        ..Config::default()
    };
    let (items, edges, warnings) = JtdScanner.scan(root, &cfg);
    let warnings = warnings
        .into_iter()
        .map(|warning| {
            (
                warning.code,
                format!("{}|{}", warning.file, warning.message),
            )
        })
        .collect();
    (items.len(), edges.len(), warnings)
}

fn assert_external_import_rejected(result: (usize, usize, Vec<(String, String)>), reason: &str) {
    assert_eq!(result.0, 1, "only the thin schema root may remain");
    assert_eq!(result.1, 0, "external metadata must mint no edge");
    let path_warnings: Vec<&(String, String)> = result
        .2
        .iter()
        .filter(|(code, _)| code == "invalid-schema-vocabulary-path")
        .collect();
    assert_eq!(path_warnings.len(), 1, "typed path warning: {:?}", result.2);
    assert!(path_warnings[0].1.starts_with("specmap.toml|"));
    assert!(path_warnings[0].1.contains(reason), "{:?}", result.2);
}

#[test]
fn absolute_path_cannot_import_vocabulary_even_when_target_is_in_project() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_thin_schema(root);
    write(root, "formats/absolute.json", "{}");
    let absolute = std::fs::canonicalize(root.join("formats/absolute.json")).unwrap();
    assert_external_import_rejected(
        scan(root, absolute.to_string_lossy().into_owned()),
        "absolute paths are not allowed",
    );
}

/// This is the mutation sentinel: bypassing component/canonical containment
/// imports the sibling vocabulary's `tail` unit and root edge, making it RED.
#[test]
fn parent_traversal_cannot_import_external_vocabulary() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(&root).unwrap();
    write_thin_schema(&root);
    let external = tmp.path().join("outside.json");
    write_external_vocabulary(&external);
    assert_external_import_rejected(
        scan(&root, "../outside.json".to_string()),
        "parent traversal",
    );
}

#[cfg(any(unix, windows))]
#[test]
fn symlink_escape_cannot_import_external_vocabulary_when_supported() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("formats")).unwrap();
    write_thin_schema(&root);
    let external = tmp.path().join("outside.json");
    write_external_vocabulary(&external);
    let link = root.join("formats/link.json");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&external, &link).unwrap();
    #[cfg(windows)]
    if std::os::windows::fs::symlink_file(&external, &link).is_err() {
        return;
    }
    assert_external_import_rejected(
        scan(&root, "formats/link.json".to_string()),
        "canonical target escapes",
    );
}

#[test]
fn rejected_declaration_display_is_bounded_and_single_line() {
    let declared = format!("{}\nsecret", "x".repeat(MAX_DECLARED_DISPLAY + 50));
    let display = bounded_declared_path(&declared);
    assert!(display.chars().count() <= MAX_DECLARED_DISPLAY + 1);
    assert!(!display.contains('\n'));
    assert!(display.ends_with('…'));
}
