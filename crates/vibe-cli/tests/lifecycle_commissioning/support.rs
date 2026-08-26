use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::common::UserScratch;
use vibe_wire::generated::extensions_report::{ExtensionEntry, ExtensionsReport};
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};
use vibe_wire::generated::lifecycle_report::{LifecycleContributionReport, LifecycleReport};

pub const ANNOUNCER: &str = "org.vibevm.fixture/phase-announcer";
pub const STACK: &str = "org.vibevm.fixture/lifecycle-rust-stack";
pub const ANNOUNCE: &str = "org.vibevm.fixture/phase-announcer#announce";
pub const ANNOUNCE_TEST: &str = "org.vibevm.fixture/phase-announcer#announce-test";
pub const CARGO_BUILD: &str = "org.vibevm.fixture/lifecycle-rust-stack#cargo-build";
pub const CARGO_TEST: &str = "org.vibevm.fixture/lifecycle-rust-stack#cargo-test";
pub const MESSAGE: &str = "hello from {phase} in {project} by {package}";
pub const CHAIN: [&str; 5] = ["validate", "install", "generate", "build", "test"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedRow {
    pub key: &'static str,
    pub phase: &'static str,
    pub point: &'static str,
    pub tier: &'static str,
    pub provider: &'static str,
    pub handler: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TreeEntry {
    Directory,
    File(Vec<u8>),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineSnapshot {
    scenario: BTreeMap<String, TreeEntry>,
    settings: BTreeMap<String, TreeEntry>,
    cache: BTreeMap<String, TreeEntry>,
    search_cache: BTreeMap<String, TreeEntry>,
    lifecycle_modified: SystemTime,
}

pub(crate) fn snapshot(root: &Path) -> BTreeMap<String, TreeEntry> {
    fn walk(base: &Path, at: &Path, out: &mut BTreeMap<String, TreeEntry>) {
        let mut entries = fs::read_dir(at)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                out.insert(relative, TreeEntry::Directory);
                walk(base, &path, out);
            } else if kind.is_file() {
                out.insert(relative, TreeEntry::File(fs::read(path).unwrap()));
            } else {
                out.insert(relative, TreeEntry::Other);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

pub fn machine_snapshot(user: &UserScratch, scenario: &Path, project: &Path) -> MachineSnapshot {
    MachineSnapshot {
        scenario: snapshot(scenario),
        settings: snapshot(&user.settings),
        cache: snapshot(&user.cache),
        search_cache: snapshot(&user.search_cache),
        lifecycle_modified: fs::metadata(project.join(".vibe/lifecycle.toml"))
            .unwrap()
            .modified()
            .unwrap(),
    }
}

pub fn lifecycle_json(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
) -> (LifecyclePlan, LifecycleReport) {
    let output = user
        .vibe()
        .args(["test", "--json", "--path"])
        .arg(project)
        .arg("--registry")
        .arg(registry)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(documents.len(), 2);
    let plan: LifecyclePlan = serde_json::from_value(documents[0].clone()).unwrap();
    let report: LifecycleReport = serde_json::from_value(documents[1].clone()).unwrap();
    assert_eq!(plan.command, "lifecycle:plan");
    assert_eq!(report.command, "lifecycle");
    assert_eq!(plan.requested, "test");
    assert_eq!(report.requested, "test");
    assert_eq!(plan.chain, CHAIN);
    assert_eq!(report.chain, CHAIN);
    assert!(plan.notices.is_empty());
    assert!(report.notices.is_empty());
    assert!(report.ok);
    (plan, report)
}

pub const EXPECTED_FOUR: [ExpectedRow; 4] = [
    ExpectedRow {
        key: CARGO_BUILD,
        phase: "build",
        point: "phase:build",
        tier: "preset",
        provider: STACK,
        handler: "script",
    },
    ExpectedRow {
        key: ANNOUNCE,
        phase: "build",
        point: "phase:build",
        tier: "dependency",
        provider: ANNOUNCER,
        handler: "builtin",
    },
    ExpectedRow {
        key: CARGO_TEST,
        phase: "test",
        point: "phase:test",
        tier: "preset",
        provider: STACK,
        handler: "script",
    },
    ExpectedRow {
        key: ANNOUNCE_TEST,
        phase: "test",
        point: "phase:test",
        tier: "dependency",
        provider: ANNOUNCER,
        handler: "builtin",
    },
];

pub fn assert_plan_rows(plan: &LifecyclePlan, expected: &[ExpectedRow]) {
    assert_eq!(plan.contributions.len(), expected.len());
    for (row, expected) in plan.contributions.iter().zip(expected) {
        assert_plan_row(row, expected);
    }
}

fn assert_plan_row(row: &PlannedContribution, expected: &ExpectedRow) {
    assert_eq!(
        (
            row.key.as_str(),
            row.phase.as_str(),
            row.point.as_str(),
            row.tier.as_str(),
            row.provider.as_str(),
            row.handler.as_str(),
        ),
        (
            expected.key,
            expected.phase,
            expected.point,
            expected.tier,
            expected.provider,
            expected.handler,
        )
    );
    assert_eq!(row.version.as_deref(), Some("0.1.0"));
    assert!(row.reference.is_none());
    assert!(row.slot_target.is_none());
}

pub fn assert_report_rows(report: &LifecycleReport, expected: &[ExpectedRow]) {
    assert_eq!(report.contributions.len(), expected.len());
    for (row, expected) in report.contributions.iter().zip(expected) {
        assert_eq!(
            (
                row.key.as_str(),
                row.phase.as_str(),
                row.point.as_str(),
                row.tier.as_str(),
                row.provider.as_str(),
                row.handler.as_str(),
            ),
            (
                expected.key,
                expected.phase,
                expected.point,
                expected.tier,
                expected.provider,
                expected.handler,
            )
        );
        assert_eq!(row.version.as_deref(), Some("0.1.0"));
        assert!(row.reference.is_none());
        assert!(row.slot_target.is_none());
    }
}

pub fn steps(report: &LifecycleReport) -> Vec<(&str, &str)> {
    report
        .steps
        .iter()
        .map(|step| (step.phase.as_str(), step.status.as_str()))
        .collect()
}

pub fn assert_optional_shape(
    row: &LifecycleContributionReport,
    message: Option<&str>,
    stdout: Option<&str>,
) {
    assert_eq!(row.message.as_deref(), message);
    match stdout {
        Some(needle) => assert!(
            row.stdout
                .as_deref()
                .is_some_and(|text| text.contains(needle))
        ),
        None => assert!(row.stdout.is_none()),
    }
    assert!(row.flagged.is_none());
    assert!(row.reference.is_none());
    assert!(row.slot_target.is_none());
    assert!(row.stderr.is_none());
    assert!(row.stdout_truncated.is_none());
    assert!(row.stderr_truncated.is_none());
    assert_eq!(row.version.as_deref(), Some("0.1.0"));
}

pub fn exact_line_index(text: &str, expected: &str) -> usize {
    let hits = text
        .lines()
        .enumerate()
        .filter_map(|(index, line)| (line == expected).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(
        hits.len(),
        1,
        "expected one exact `{expected}` line in:\n{text}"
    );
    hits[0]
}

pub fn query(user: &UserScratch, project: &Path) -> ExtensionsReport {
    let output = user
        .vibe()
        .args(["extensions", "--json", "--path"])
        .arg(project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<ExtensionsReport>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(documents.len(), 1);
    documents.into_iter().next().unwrap()
}

pub fn report_row<'a>(report: &'a ExtensionsReport, key: &str) -> &'a ExtensionEntry {
    let rows = report
        .declarations
        .iter()
        .filter(|row| row.key == key)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "{key}: {report:?}");
    rows[0]
}

pub fn binary_candidate(project: &Path) -> PathBuf {
    let expected = if cfg!(windows) {
        "owner-scenario.exe"
    } else {
        "owner-scenario"
    };
    let candidates = walkdir::WalkDir::new(project.join("target"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() == expected)
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1, "binary candidates: {candidates:?}");
    candidates.into_iter().next().unwrap()
}

pub fn assert_real_binary(project: &Path) {
    let binary = binary_candidate(project);
    let metadata = fs::symlink_metadata(&binary).unwrap();
    let file_type = metadata.file_type();
    assert!(
        file_type.is_file() && !file_type.is_symlink(),
        "binary candidate is not a non-symlink regular file: {binary:?}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(metadata.permissions().mode() & 0o111, 0, "not executable");
    }
}
