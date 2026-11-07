//! Read-only checks of actual declared installed boot graphs.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-BOOT-CHECK",
    r = 1
);
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use vibe_test_support::UserScratch;
fn spec_rel(path: &str) -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_specs_root().join(path))
}
fn boot_rel(path: &str) -> PathBuf {
    vibe_core::layout::current_boot_dir().join(path)
}
fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(relative, None);
                pending.push(path);
            } else {
                out.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    out
}
fn fixture(user: &UserScratch, format: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let parent = registry.path().join("org.example/parent/v1.0.0");
    let child = registry.path().join("org.example/child/v1.0.0");
    let source = spec_rel("contract/parent.payload");
    let child_source = spec_rel("boot/child.md");
    fs::create_dir_all(parent.join(Path::new(&source).parent().unwrap())).unwrap();
    fs::create_dir_all(child.join(Path::new(&child_source).parent().unwrap())).unwrap();
    fs::write(parent.join("vibe.toml"),format!("[package]\ngroup='org.example'\nname='parent'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n[[package.sources]]\nfile='{source}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[requires.packages]\n'org.example/child'={{version='=1.0.0',link='static'}}\n[boot_snippet]\nsource='{source}'\n")).unwrap();
    fs::write(
        parent.join(&source),
        "# Parent {#root}\n\n##PARENT declared_cli_marker.\n",
    )
    .unwrap();
    fs::write(child.join("vibe.toml"),format!("[package]\ngroup='org.example'\nname='child'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='{child_source}'\n")).unwrap();
    fs::write(
        child.join(child_source),
        "# Child {#child}\n\nchild_cli_marker.\n",
    )
    .unwrap();
    fs::write(project.path().join("vibe.toml"),format!("[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\nspec_format='{format}'\n[requires.packages]\n'org.example/parent'={{version='=1.0.0',link='static'}}\n")).unwrap();
    let installed = user
        .vibe()
        .args(["install", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(installed.status.success(), "{format}: {installed:?}");
    project
}
fn check(user: &UserScratch, root: &Path) -> (std::process::Output, serde_json::Value) {
    let before = inventory(root);
    let settings = inventory(&user.settings);
    let result = user
        .vibe()
        .args(["--json", "check", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(
        inventory(root),
        before,
        "check mutated project state: {result:?}"
    );
    assert_eq!(
        inventory(&user.settings),
        settings,
        "check mutated operator state"
    );
    let report = serde_json::Deserializer::from_slice(&result.stdout)
        .into_iter::<serde_json::Value>()
        .map(Result::unwrap)
        .find(|v| v["command"] == "check")
        .expect("normal check report");
    (result, report)
}
fn graph_findings(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "boot_graph_integrity")
        .collect()
}
fn slot(root: &Path) -> PathBuf {
    root.join(vibe_core::layout::current_vibedeps_root())
        .join("org.example.parent/1.0.0")
}
#[test]
fn fresh_declared_mixed_and_xml_graphs_are_clean_and_read_only() {
    for format in ["mixed", "xml"] {
        let user = UserScratch::new();
        let root = fixture(&user, format);
        let (output, report) = check(&user, root.path());
        assert!(output.status.success(), "{output:?} {report}");
        assert!(graph_findings(&report).is_empty(), "{report}");
    }
}
#[test]
fn corrupt_and_missing_index_are_warnings_with_success_exit() {
    for format in ["mixed", "xml"] {
        let user = UserScratch::new();
        let root = fixture(&user, format);
        let index = slot(root.path()).join(boot_rel("INDEX.md"));
        let original = fs::read_to_string(&index).unwrap();
        for missing in [false, true] {
            if missing {
                fs::remove_file(&index).unwrap();
            } else {
                fs::write(&index, original.replace("# vibe:fp ", "# vibe:fp corrupt-")).unwrap();
            }
            let (output, report) = check(&user, root.path());
            assert!(output.status.success(), "{output:?} {report}");
            assert_eq!(report["ok"], true, "{report}");
            let findings = graph_findings(&report);
            assert_eq!(findings.len(), 1, "{report}");
            assert_eq!(findings[0]["severity"], "warning");
            assert!(
                findings[0]["message"]
                    .as_str()
                    .unwrap()
                    .contains("org.example/parent")
            );
            assert!(
                findings[0]["message"]
                    .as_str()
                    .unwrap()
                    .contains("vibe reinstall")
            );
            fs::write(&index, &original).unwrap();
        }
        assert!(graph_findings(&check(&user, root.path()).1).is_empty());
    }
}
#[test]
fn applicable_format_lock_and_slot_failures_are_visible_errors() {
    for failure in ["format", "lock", "slot", "nonregular", "manifest", "config"] {
        let user = UserScratch::new();
        let root = fixture(&user, "xml");
        match failure {
            "format" => {
                let path = root.path().join("vibe.toml");
                let text = fs::read_to_string(&path).unwrap();
                fs::write(
                    path,
                    text.replace("spec_format = \"xml\"", "spec_format = \"mixed\"")
                        .replace("spec_format='xml'", "spec_format='mixed'"),
                )
                .unwrap();
            }
            "lock" => fs::write(root.path().join("vibe.lock"), "malformed = [").unwrap(),
            "slot" => fs::remove_dir_all(slot(root.path())).unwrap(),
            "nonregular" => {
                fs::remove_file(root.path().join("vibe.lock")).unwrap();
                fs::create_dir(root.path().join("vibe.lock")).unwrap();
            }
            "manifest" => fs::write(root.path().join("vibe.toml"), "malformed = [").unwrap(),
            "config" => fs::write(user.settings.join("config.toml"), "malformed = [").unwrap(),
            _ => unreachable!(),
        }
        let (output, report) = check(&user, root.path());
        assert_eq!(
            output.status.code(),
            Some(1),
            "{failure}: {output:?} {report}"
        );
        assert_eq!(report["ok"], false, "{failure}: {report}");
        let findings = graph_findings(&report);
        assert_eq!(findings.len(), 1, "{failure}: {report}");
        assert_eq!(findings[0]["severity"], "error", "{failure}: {report}");
    }
}
#[test]
fn absent_lock_skips_graph_and_irrelevant_configuration_without_writes() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='bare'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(user.settings.join("config.toml"), "malformed = [").unwrap();
    let (_, report) = check(&user, root.path());
    assert!(graph_findings(&report).is_empty(), "{report}");
}

#[test]
fn operator_default_selects_format_when_project_has_no_override() {
    let user = UserScratch::new();
    let root = fixture(&user, "xml");
    let path = root.path().join("vibe.toml");
    let mut manifest: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    manifest["project"]
        .as_table_mut()
        .unwrap()
        .remove("spec_format");
    fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    fs::write(
        user.settings.join("config.toml"),
        "[install]\nspec_format='xml'\n",
    )
    .unwrap();
    let (output, report) = check(&user, root.path());
    assert!(output.status.success(), "{output:?} {report}");
    assert!(graph_findings(&report).is_empty(), "{report}");
    fs::write(
        user.settings.join("config.toml"),
        "[install]\nspec_format='mixed'\n",
    )
    .unwrap();
    let (output, report) = check(&user, root.path());
    assert_eq!(output.status.code(), Some(1), "{output:?} {report}");
    assert_eq!(graph_findings(&report)[0]["severity"], "error");
}

#[test]
fn valid_empty_installed_epoch_has_no_stale_units() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='bare'\nversion='1.0.0'\n",
    )
    .unwrap();
    vibe_core::manifest::Lockfile::empty("fixture", "1970-01-01T00:00:00Z")
        .write(root.path().join("vibe.lock"))
        .unwrap();
    let (_, report) = check(&user, root.path());
    assert!(graph_findings(&report).is_empty(), "{report}");
}
