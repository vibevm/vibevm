//! Host policy activation refuses unsupported readers before semantic effects.
mod common;
use common::UserScratch;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out)
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn manifest(rows: &str) -> String {
    format!("[project]\ngroup='org.example'\nname='host'\nversion='1.0.0'\n{rows}")
}
const ROW: &str = "[[project.sources]]\nfile='data.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='explicit'\n";
#[test]
fn host_progress_show_convert_and_checks_do_not_parse_ordinary_content() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "vibe.toml", &manifest(ROW));
    write(
        root.path(),
        "data.md",
        "# Ordinary\n\n@fact:NOT-A-FACT raw. @status:impl/done\n",
    );
    write(
        root.path(),
        &common::boot_rel("00-core.md"),
        "malformed native <spec\n",
    );
    write(root.path(), &common::spec_rel("WAL.xml"), "<broken");
    let before = files(root.path());
    let progress = user
        .vibe()
        .args(["progress", "check", "--path"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(progress.status.success(), "{progress:?}");
    assert!(!String::from_utf8_lossy(&progress.stdout).contains("NOT-A-FACT"));
    assert_eq!(files(root.path()), before, "progress wrote project files");
    {
        let args = ["check", "--path"];
        let result = user.vibe().args(args).arg(root.path()).output().unwrap();
        assert!(!result.status.success(), "{args:?}: {result:?}");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            text.contains("explicitly classified") || text.contains("SOURCE-ADMISSION-BOUNDARY"),
            "{args:?}: {text}"
        );
        assert!(!text.contains("unclosed XML"));
        assert_eq!(files(root.path()), before, "{args:?} wrote project files");
    }
    let shown = user
        .vibe()
        .args(["--json", "show", "effective", "--path"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(shown.status.success(), "{shown:?}");
    let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert!(
        shown["sections"].as_array().unwrap().is_empty(),
        "ordinary boot content is not enrolled: {shown}"
    );
    assert_eq!(files(root.path()), before, "show wrote project files");
    let result = user
        .vibe()
        .args(["--json", "tree", "--path"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    let tree: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(tree["in_place_specs"].as_array().unwrap().is_empty());
    assert_eq!(files(root.path()), before, "tree wrote project files");
    let result = user
        .vibe()
        .args(["refactor", "convert-source", "--to", "xml"])
        .arg(root.path().join("data.md"))
        .output()
        .unwrap();
    assert!(!result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stderr).contains("SOURCE-ADMISSION-BOUNDARY"));
    assert_eq!(files(root.path()), before);
}
#[test]
fn progress_seal_refuses_nested_explicit_policy_without_changing_valid_verdict_cache() {
    let user = UserScratch::new();
    for rows in [ROW, "sources=[]\n"] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "vibe.toml", &manifest(""));
        write(root.path(), "nested/vibe.toml", &manifest(rows));
        let rel = "nested/data.md";
        let old = "# One {#one}\n\n##a1 old claim. @impl/done\n";
        let changed = old.replace("old", "changed");
        write(root.path(), rel, &changed);
        let doc = progress_core::parse::parse_document(rel, old);
        let mut cache = progress_core::cache::Cache::default();
        cache.upsert(&doc, &progress_core::rollup::rollup_doc(&doc));
        let fields = &mut cache.files.get_mut(rel).unwrap().campaign;
        fields.insert(
            "verdicts".into(),
            serde_json::json!({"a1":{"v":"confirmed"}}),
        );
        fields.insert("processed_hash".into(), serde_json::json!(doc.content_hash));
        fields.insert(
            "verified_at".into(),
            serde_json::json!("2026-01-01T00:00:00Z"),
        );
        let campaign = root.path().join("campaigns/test/run");
        fs::create_dir_all(&campaign).unwrap();
        cache.store(&campaign.join("cache.json")).unwrap();
        let before = files(root.path());
        let result = user
            .vibe()
            .args(["progress", "seal", "--path"])
            .arg(root.path())
            .args(["--campaign", "test"])
            .arg(root.path().join(rel))
            .output()
            .unwrap();
        assert!(!result.status.success(), "{result:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("progress seal does not support"),
            "{result:?}"
        );
        assert_eq!(files(root.path()), before);
        // The same existing per-file legacy operation remains usable.
        write(root.path(), "nested/vibe.toml", &manifest(""));
        let result = user
            .vibe()
            .args(["progress", "seal", "--path"])
            .arg(root.path())
            .args(["--campaign", "test"])
            .arg(root.path().join(rel))
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        let recorded = progress_core::cache::Cache::load(&campaign.join("cache.json")).unwrap();
        assert_eq!(
            recorded.files[rel].campaign["processed_hash"],
            serde_json::json!(progress_core::parse::parse_document(rel, &changed).content_hash)
        );
    }
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(root, &path, out)
            } else {
                out.insert(rel, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[test]
fn host_missing_selected_boot_refuses_before_writes_and_empty_keeps_dependency_boot() {
    for empty in [false, true] {
        let user = UserScratch::new();
        let root = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let package = registry.path().join("org.source/plain/v1.0.0");
        let boot = common::boot_rel("package.md");
        write(
            &package,
            "vibe.toml",
            &format!(
                "[package]\nkind='flow'\ngroup='org.source'\nname='plain'\nversion='1.0.0'\n[boot_snippet]\nsource='{boot}'\nlink='static'\n"
            ),
        );
        write(&package, &boot, "# Dependency\n\nDEPENDENCY-CONTENT\n");
        let selected = format!(
            "[[project.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n",
            common::boot_rel("missing.payload")
        );
        write(
            root.path(),
            "vibe.toml",
            &manifest(if empty { "sources=[]\n" } else { &selected }),
        );
        write(root.path(), "data.md", "ordinary content");
        write(
            root.path(),
            &common::boot_rel("00-core.md"),
            "# HOST-MUST-NOT-BE-PARSED\n",
        );
        let before = inventory(root.path());
        let result = user
            .vibe()
            .args(["install", "org.source/plain@=1.0.0", "--path"])
            .arg(root.path())
            .arg("--registry")
            .arg(registry.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        if empty {
            assert!(result.status.success(), "{result:?}");
            let lane = fs::read_to_string(root.path().join(common::boot_rel("STATIC.md"))).unwrap();
            assert!(lane.contains("DEPENDENCY-CONTENT"), "{lane}");
            assert!(!lane.contains("HOST-MUST-NOT-BE-PARSED"));
            let shown = user
                .vibe()
                .args(["show", "effective", "--path"])
                .arg(root.path())
                .output()
                .unwrap();
            assert!(shown.status.success(), "{shown:?}");
            let text = String::from_utf8_lossy(&shown.stdout);
            assert!(text.contains("DEPENDENCY-CONTENT"));
            assert!(!text.contains("HOST-MUST-NOT-BE-PARSED"));
        } else {
            assert!(!result.status.success(), "{result:?}");
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains("exact authored boot source absent"),
                "{result:?}"
            );
            let mut after = inventory(root.path());
            for rel in [".vibe", ".vibe/lifecycle"] {
                let path = PathBuf::from(rel);
                if !before.contains_key(&path) {
                    assert_eq!(after.remove(&path), Some(None));
                }
            }
            assert_eq!(
                after.remove(&PathBuf::from(".vibe/lifecycle.lock")),
                Some(Some(Vec::new()))
            );
            let runs = after
                .keys()
                .filter(|path| {
                    path.parent() == Some(Path::new(".vibe/lifecycle"))
                        && !before.contains_key(*path)
                })
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(runs.len(), 1);
            let name = runs[0].file_name().unwrap().to_str().unwrap();
            assert_eq!(name.len(), 32);
            assert!(
                name.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            );
            assert_eq!(after.remove(&runs[0]), Some(None));
            assert_eq!(after, before);
        }
    }
}

#[test]
fn generated_only_show_is_available_under_nonempty_host_policy() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "vibe.toml", &manifest(ROW));
    write(
        root.path(),
        &common::boot_rel("STATIC.xml"),
        "<?xml version=\"1.0\"?><spec><title>ONE-GENERATED</title></spec>\n<?xml version=\"1.0\"?><spec><title>TWO-GENERATED</title></spec>\n",
    );
    let before = files(root.path());
    let result = user
        .vibe()
        .args(["show", "effective", "--path"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    let text = String::from_utf8_lossy(&result.stdout);
    assert!(text.contains("ONE-GENERATED") && text.contains("TWO-GENERATED"));
    assert_eq!(files(root.path()), before);
}

#[test]
fn legacy_show_keeps_missing_written_file_diagnostic_without_parent_directory() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "vibe.toml", &manifest(""));
    write(
        root.path(),
        "vibe.lock",
        &format!(
            "[meta]\nschema_version={}\ngenerated_by='test'\ngenerated_at='2026-01-01T00:00:00Z'\n[[package]]\ngroup='org.source'\nname='plain'\nkind='flow'\nversion='1.0.0'\ncontent_hash='sha256:{}'\nsource_url='https://example.invalid/plain.git'\nfiles_written=['missing-parent/file.md']\n",
            vibe_core::manifest::CURRENT_SCHEMA_VERSION,
            "a".repeat(64)
        ),
    );
    let before = files(root.path());
    let output = user
        .vibe()
        .args(["show", "effective", "--path"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("MISSING ON DISK"));
    assert_eq!(files(root.path()), before);
}
