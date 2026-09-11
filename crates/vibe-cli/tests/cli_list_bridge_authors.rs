//! Bridge authorship stays split on the `vibe list` operator surface.

fn fixture(root: &std::path::Path) {
    std::fs::write(
        root.join("vibe.toml"),
        "[project]\nname = \"consumer\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("vibe.lock"),
        format!(
            r#"[meta]
generated_by = "list-authors-fixture"
generated_at = "2026-09-11T00:00:00Z"
schema_version = {}
root_dependencies = []

[[package]]
kind = "feat"
name = "speckit"
group = "org.speckit"
version = "1.0.0"
bridge = true
authors = ["Bridge Maintainer"]
source_url = "file:///fixture/speckit"
content_hash = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"

[[package.embedded_source]]
name = "upstream"
source_url = "https://github.com/github/spec-kit.git"
resolved_commit = "96c9bd657bfd5de0d651a6165084932b7304ac99"
tree_oid = "0123456789abcdef0123456789abcdef01234567"
content_hash = "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
upstream_authors = ["GitHub, Inc."]
upstream_license = "MIT"
license_path = "LICENSE"
license_url = "https://github.com/github/spec-kit/blob/96c9bd657bfd5de0d651a6165084932b7304ac99/LICENSE"
license_file_sha256 = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
"#,
            vibe_core::manifest::CURRENT_SCHEMA_VERSION
        ),
    )
    .unwrap();
}

#[test]
fn json_and_verbose_text_keep_package_and_upstream_authors_separate() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());

    let json = vibe_test_support::vibe()
        .arg("--json")
        .arg("list")
        .arg("--path")
        .arg(root.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    let package = &report["packages"][0];
    assert_eq!(package["authors"], serde_json::json!(["Bridge Maintainer"]));
    assert_eq!(
        package["embedded_sources"][0]["upstream_authors"],
        serde_json::json!(["GitHub, Inc."])
    );

    let text = vibe_test_support::vibe()
        .arg("list")
        .arg("--verbose")
        .arg("--path")
        .arg(root.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(text).unwrap();
    assert!(
        text.contains("package authors: Bridge Maintainer"),
        "{text}"
    );
    assert!(text.contains("upstream authors:  GitHub, Inc."), "{text}");
}
