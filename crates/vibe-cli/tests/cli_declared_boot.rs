//! Real binary install and bound extension analysis for declared source inputs.
mod common;
use common::UserScratch;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for row in fs::read_dir(at).unwrap() {
            let path = row.unwrap().path();
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
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}
fn exercise(format: &str) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let parent = registry.path().join("org.example/parent/v1.0.0");
    let child = registry.path().join("org.example/child/v1.0.0");
    let source = common::spec_rel("contract/parent.payload");
    let child_source = common::spec_rel("boot/child.md");
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
    let lane = fs::read_to_string(project.path().join(common::boot_rel(if format == "xml" {
        "STATIC.xml"
    } else {
        "STATIC.md"
    })))
    .unwrap();
    assert_eq!(lane.matches("declared_cli_marker").count(), 1, "{lane}");
    assert_eq!(lane.matches("child_cli_marker").count(), 1, "{lane}");
    let before = inventory(project.path());
    let analyzed = user
        .vibe()
        .args(["--json", "extensions", "analyze", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(analyzed.status.success(), "{format}: {analyzed:?}");
    let report: serde_json::Value = serde_json::from_slice(&analyzed.stdout).unwrap();
    assert!(!report["artifacts"].as_array().unwrap().is_empty());
    assert_eq!(
        inventory(project.path()),
        before,
        "analysis wrote project state"
    );
    // Existing selection helper must disagree with the installed Markdown
    // representation when config changes to Mixed; STATIC.md alone cannot decide.
    if format == "markdown" {
        let manifest = project.path().join("vibe.toml");
        fs::write(
            &manifest,
            fs::read_to_string(&manifest)
                .unwrap()
                .replace("spec_format='markdown'", "spec_format='mixed'"),
        )
        .unwrap();
        let failed = user
            .vibe()
            .args(["extensions", "analyze", "--path"])
            .arg(project.path())
            .output()
            .unwrap();
        assert!(
            !failed.status.success(),
            "format drift accepted: {failed:?}"
        );
        assert!(
            String::from_utf8_lossy(&failed.stderr).contains("format"),
            "{failed:?}"
        );
    }
}

#[test]
fn declared_markdown_install_and_analysis_use_selected_input_format() {
    exercise("markdown");
}
#[test]
fn declared_xml_install_and_analysis_use_selected_input_format() {
    exercise("xml");
}
