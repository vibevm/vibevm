//! Real read-only requirements queries over opted-in host source inventories.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-INVENTORY",
    r = 1
);

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use vibe_test_support::UserScratch;
use vibe_wire::generated::requirements_report::{
    RequirementSourceKind, RequirementsReport, SourceResultState,
};

const HOST: &str = "[project]\ngroup='org.example'\nname='host'\nversion='1.0.0'\n";
const NATIVE_XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><title>Host</title><p><REAL fact=\"true\" status=\"impl/done\">native body</REAL></p><p><VOID fact=\"true\" status=\"spec/void\">void body</VOID></p></spec>";

fn write(root: &Path, relative: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "vibe.toml",
        format!(
            "{HOST}\
             [[project.sources]]\nfile='contract.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n\
             [[project.sources]]\ndirectory='rules'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
             [[project.sources]]\nfile='ordinary.md'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='explicit'\n\
             [[project.sources]]\nfile='ordinary.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n"
        ),
    );
    write(root.path(), "contract.payload", NATIVE_XML);
    write(
        root.path(),
        "rules/guide.md",
        "# Guide\n\n@fact:MARKDOWN native. @status:doc/work\n",
    );
    write(
        root.path(),
        "ordinary.md",
        "# Ordinary\n\n@fact:FAKE literal. @status:impl/done\n\n#use spec://missing/package/rule\n",
    );
    write(root.path(), "ordinary.xml", [0xff, 0, b'<', b'!']);
    write(root.path(), "ignored/broken.xml", "<malformed");
    root
}

fn snapshot(root: &Path) -> (BTreeSet<PathBuf>, BTreeMap<PathBuf, Vec<u8>>) {
    let mut directories = BTreeSet::new();
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                directories.insert(relative);
                pending.push(path);
            } else {
                files.insert(relative, fs::read(path).unwrap());
            }
        }
    }
    (directories, files)
}

fn query(user: &UserScratch, root: &Path) -> RequirementsReport {
    let before = snapshot(root);
    let output = user
        .vibe()
        .args(["--json", "requirements", "--path"])
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let report: RequirementsReport = serde_json::from_slice(&output.stdout).unwrap();
    vibe_wire::behaviour::requirements_report::validate(&report).unwrap();
    assert_eq!(snapshot(root), before, "query changed files or directories");
    assert!(
        !root.join(".vibe").exists(),
        "query created lifecycle state"
    );
    assert!(report.observation.lifecycle_run_id.is_none());
    assert_eq!(report.sources.len(), 1);
    assert_eq!(report.sources[0].source.kind, RequirementSourceKind::Host);
    assert_eq!(report.sources[0].source.package, "org.example/host");
    report
}

#[test]
fn declared_host_query_uses_native_media_and_directory_scope_without_loading_raw_directives() {
    let user = UserScratch::new();
    let root = fixture();
    let report = query(&user, root.path());
    assert_eq!(report.sources[0].state, SourceResultState::Available);
    assert!(report.sources[0].digest.is_some());
    let actual = report
        .rows
        .iter()
        .map(|row| {
            (
                row.address.as_str(),
                serde_json::to_value(&row.authoring.status).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual,
        BTreeMap::from([
            (
                "spec://org.example/host/contract.payload#REAL",
                serde_json::json!({"stage":"impl","state":"done"})
            ),
            (
                "spec://org.example/host/contract.payload#VOID",
                serde_json::json!({"stage":"spec","state":"void"})
            ),
            (
                "spec://org.example/host/rules/guide#MARKDOWN",
                serde_json::json!({"stage":"doc","state":"work"})
            ),
        ])
    );
}

#[test]
fn declared_host_query_distinguishes_invalid_native_bytes_from_missing_exact_source() {
    let user = UserScratch::new();
    let root = fixture();
    write(root.path(), "contract.payload", "<spec><unfinished");
    let invalid = query(&user, root.path());
    assert_eq!(invalid.sources[0].state, SourceResultState::Invalid);
    assert!(invalid.sources[0].digest.is_some());
    assert!(invalid.rows.is_empty());
    fs::remove_file(root.path().join("contract.payload")).unwrap();
    let missing = query(&user, root.path());
    assert_eq!(missing.sources[0].state, SourceResultState::Unavailable);
    assert!(missing.sources[0].digest.is_none());
    assert!(missing.rows.is_empty());
}

#[test]
fn empty_host_policy_is_available_zero_while_absent_policy_keeps_legacy_facts() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let legacy = vibe_core::layout::current_specs_root().join("LEGACY.md");
    write(
        root.path(),
        legacy.to_str().unwrap(),
        "# Legacy\n\n@fact:OLD native. @status:spec/done\n",
    );
    write(root.path(), "vibe.toml", format!("{HOST}sources=[]\n"));
    let empty = query(&user, root.path());
    assert_eq!(empty.sources[0].state, SourceResultState::Available);
    assert!(empty.sources[0].digest.is_some());
    assert!(empty.rows.is_empty());
    write(root.path(), "vibe.toml", HOST);
    let legacy = query(&user, root.path());
    assert_eq!(legacy.sources[0].state, SourceResultState::Available);
    assert_eq!(legacy.rows.len(), 1);
    assert_eq!(legacy.rows[0].address, "spec://org.example/host/LEGACY#OLD");
}
